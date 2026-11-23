use crate::ipc::IpcHub;
use aes::Aes128;
use cbc::Decryptor;
use cipher::{BlockDecryptMut, KeyIvInit};
use serde_json::json;
use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use tokio::net::UdpSocket;
use tokio::sync::watch;

type Aes128CbcDec = Decryptor<Aes128>;

const NO_DATA_MARKER: [u8; 4] = [0x00, 0x68, 0x34, 0x00];

fn decrypt_audio_payload(aeskey: &[u8; 16], aesiv: &[u8; 16], encrypted: &[u8]) -> Vec<u8> {
    let mut out = encrypted.to_vec();
    let enc_len = (out.len() / 16) * 16;
    if enc_len > 0 {
        let mut dec = Aes128CbcDec::new(aeskey.into(), aesiv.into());
        for chunk in out[..enc_len].chunks_exact_mut(16) {
            dec.decrypt_block_mut(chunk.into());
        }
    }
    out
}

fn seq_diff(a: u16, b: u16) -> i32 {
    (a.wrapping_sub(b) as i16) as i32
}

/// Spawns UDP RTP audio data + RTCP control receiver for AirPlay (AAC-ELD ct=8, ALAC ct=2, AAC-LC ct=4, LPCM ct=1).
pub async fn spawn_audio_receiver(
    data_port: u16,
    control_port: u16,
    remote_ip: IpAddr,
    remote_control_port: u16,
    aeskey: [u8; 16],
    aesiv: [u8; 16],
    ct: u8,
    ipc: IpcHub,
) -> std::io::Result<(u16, u16, watch::Sender<bool>)> {
    let dsock = match UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], data_port))).await {
        Ok(s) => s,
        Err(_) => UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?,
    };
    let csock = match UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], control_port))).await {
        Ok(s) => s,
        Err(_) => UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?,
    };

    let actual_dport = dsock.local_addr()?.port();
    let actual_cport = csock.local_addr()?.port();
    let (stop_tx, mut stop_rx) = watch::channel(false);

    let codec_name = match ct {
        1 => "LPCM",
        2 => "ALAC",
        4 => "AAC-LC",
        8 => "AAC-ELD",
        _ => "UNKNOWN",
    };
    eprintln!(
        "[Audio] Listening UDP data={} control={} codec={} (ct={})",
        actual_dport, actual_cport, codec_name, ct
    );

    ipc.send_event(&json!({
        "type": "audio_format",
        "ct": ct,
        "codec": codec_name,
        "sample_rate": 44100,
        "channels": 2,
    }));

    tokio::spawn(async move {
        let mut dbuf = vec![0u8; 16384];
        let mut cbuf = vec![0u8; 16384];
        let mut remote_ctrl_addr: Option<SocketAddr> = if remote_control_port != 0 {
            Some(SocketAddr::new(remote_ip, remote_control_port))
        } else {
            None
        };
        let mut ctrl_seq: u16 = 0;

        // Lightweight jitter/dedup ring (remember AAC-ELD sends each packet 3x!)
        let mut reorder: BTreeMap<u16, (u32, Vec<u8>)> = BTreeMap::new();
        let mut expected_seq: Option<u16> = None;

        loop {
            tokio::select! {
                _ = stop_rx.changed() => break,
                res = dsock.recv_from(&mut dbuf) => {
                    if let Ok((n, _)) = res {
                        handle_audio_rtp(
                            &dbuf[..n],
                            ct,
                            &aeskey,
                            &aesiv,
                            &mut reorder,
                            &mut expected_seq,
                            &ipc,
                            &csock,
                            remote_ctrl_addr,
                            &mut ctrl_seq,
                        ).await;
                    }
                }
                res = csock.recv_from(&mut cbuf) => {
                    if let Ok((n, from)) = res {
                        if remote_ctrl_addr.is_none() {
                            remote_ctrl_addr = Some(from);
                        }
                        if n >= 8 {
                            let pkt_type = cbuf[1] & 0x7f;
                            if pkt_type == 0x56 && n >= 16 {
                                // Resent audio RTP packet starts at offset 4
                                handle_audio_rtp(
                                    &cbuf[4..n],
                                    ct,
                                    &aeskey,
                                    &aesiv,
                                    &mut reorder,
                                    &mut expected_seq,
                                    &ipc,
                                    &csock,
                                    remote_ctrl_addr,
                                    &mut ctrl_seq,
                                ).await;
                            }
                        }
                    }
                }
            }
        }

        ipc.send_event(&json!({ "type": "audio_stopped" }));
    });

    Ok((actual_dport, actual_cport, stop_tx))
}

#[allow(clippy::too_many_arguments)]
async fn handle_audio_rtp(
    pkt: &[u8],
    ct: u8,
    aeskey: &[u8; 16],
    aesiv: &[u8; 16],
    reorder: &mut BTreeMap<u16, (u32, Vec<u8>)>,
    expected_seq: &mut Option<u16>,
    ipc: &IpcHub,
    csock: &UdpSocket,
    remote_ctrl_addr: Option<SocketAddr>,
    ctrl_seq: &mut u16,
) {
    if pkt.len() <= 12 {
        return;
    }
    // Skip ALAC 44-byte format header packets (UxPlay raop_rtp.c:630)
    if ct == 2 && pkt.len() == 44 {
        return;
    }

    let seq = u16::from_be_bytes([pkt[2], pkt[3]]);
    let rtp_ts = u32::from_be_bytes([pkt[4], pkt[5], pkt[6], pkt[7]]);
    let payload_enc = &pkt[12..];

    // Skip empty no_data_marker
    if payload_enc.len() == 4 && payload_enc == NO_DATA_MARKER {
        if expected_seq.is_none() {
            *expected_seq = Some(seq.wrapping_add(1));
        }
        return;
    }

    if let Some(exp) = *expected_seq {
        let diff = seq_diff(seq, exp);
        if diff < 0 {
            // Duplicate or already played (normal for AAC-ELD 3x redundancy)
            return;
        }
        if diff > 128 {
            // Large jump, resync
            reorder.clear();
            *expected_seq = Some(seq);
        }
    } else {
        *expected_seq = Some(seq);
    }

    if reorder.contains_key(&seq) {
        return;
    }

    let decrypted = decrypt_audio_payload(aeskey, aesiv, payload_enc);
    reorder.insert(seq, (rtp_ts, decrypted));

    // Drain in-order frames
    while let Some(exp) = *expected_seq {
        if let Some((ts, data)) = reorder.remove(&exp) {
            let ts_ns = (ts as u64) * 1_000_000_000 / 44_100;
            ipc.send_audio(ct, ts_ns, data);
            *expected_seq = Some(exp.wrapping_add(1));
        } else {
            // If buffer has grown past 4 packets waiting for a missing seq, request resend or skip
            if reorder.len() > 6 {
                if let Some(addr) = remote_ctrl_addr {
                    let s = *ctrl_seq;
                    *ctrl_seq = ctrl_seq.wrapping_add(1);
                    let req: [u8; 8] = [
                        0x80,
                        0x55 | 0x80,
                        (s >> 8) as u8,
                        s as u8,
                        (exp >> 8) as u8,
                        exp as u8,
                        0x00,
                        0x01,
                    ];
                    let _ = csock.send_to(&req, addr).await;
                }
                if reorder.len() > 12 {
                    *expected_seq = Some(exp.wrapping_add(1));
                    continue;
                }
            }
            break;
        }
    }
}
