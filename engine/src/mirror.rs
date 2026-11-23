use crate::ipc::IpcHub;
use aes::Aes128;
use cipher::{KeyIvInit, StreamCipher};
use serde_json::json;
use sha2::{Digest, Sha512};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::sync::watch;

type Aes128Ctr = ctr::Ctr128BE<Aes128>;

const NAL_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

fn derive_video_cipher(aeskey: &[u8; 16], stream_connection_id: u64) -> Aes128Ctr {
    let key_label = format!("AirPlayStreamKey{}", stream_connection_id);
    let iv_label = format!("AirPlayStreamIV{}", stream_connection_id);

    let mut hk = Sha512::new();
    hk.update(key_label.as_bytes());
    hk.update(aeskey);
    let dk = hk.finalize();

    let mut hi = Sha512::new();
    hi.update(iv_label.as_bytes());
    hi.update(aeskey);
    let di = hi.finalize();

    let mut key = [0u8; 16];
    let mut iv = [0u8; 16];
    key.copy_from_slice(&dk[..16]);
    iv.copy_from_slice(&di[..16]);

    Aes128Ctr::new(&key.into(), &iv.into())
}

/// Parses an unencrypted type-0x01 codec config packet into Annex-B parameter sets (SPS/PPS or VPS/SPS/PPS).
/// Returns `(is_h265, annexb_param_sets)`.
fn parse_codec_packet(payload: &[u8]) -> Result<(bool, Vec<u8>), String> {
    if payload.len() < 12 {
        return Err(format!("Codec payload too short: {}", payload.len()));
    }

    // Check for HEVC 'hvc1' marker at payload[4..8] (UxPlay raop_rtp_mirror.c:615)
    if payload.len() >= 8 && &payload[4..8] == b"hvc1" {
        const VPS_SC: [u8; 4] = [0xa0, 0x00, 0x01, 0x00];
        const SPS_SC: [u8; 4] = [0xa1, 0x00, 0x01, 0x00];
        const PPS_SC: [u8; 4] = [0xa2, 0x00, 0x01, 0x00];

        // Locate VPS start code (usually at offset 0x75, also search defensively if shifted)
        let mut pos = if payload.len() > 0x75 + 5 && payload[0x75..0x79] == VPS_SC {
            0x75
        } else {
            payload
                .windows(4)
                .position(|w| w == VPS_SC)
                .ok_or("HEVC VPS start code not found")?
        };

        if pos + 5 > payload.len() {
            return Err("Truncated HEVC VPS".into());
        }
        let vps_size = u16::from_be_bytes([payload[pos + 3], payload[pos + 4]]) as usize;
        pos += 5;
        if pos + vps_size + 5 > payload.len() {
            return Err("Truncated HEVC VPS data".into());
        }
        let vps = &payload[pos..pos + vps_size];
        pos += vps_size;

        if payload[pos..pos + 4] != SPS_SC {
            return Err("HEVC SPS start code mismatch".into());
        }
        let sps_size = u16::from_be_bytes([payload[pos + 3], payload[pos + 4]]) as usize;
        pos += 5;
        if pos + sps_size + 5 > payload.len() {
            return Err("Truncated HEVC SPS data".into());
        }
        let sps = &payload[pos..pos + sps_size];
        pos += sps_size;

        if payload[pos..pos + 4] != PPS_SC {
            return Err("HEVC PPS start code mismatch".into());
        }
        let pps_size = u16::from_be_bytes([payload[pos + 3], payload[pos + 4]]) as usize;
        pos += 5;
        if pos + pps_size > payload.len() {
            return Err("Truncated HEVC PPS data".into());
        }
        let pps = &payload[pos..pos + pps_size];

        let mut out = Vec::with_capacity(vps_size + sps_size + pps_size + 12);
        out.extend_from_slice(&NAL_START_CODE);
        out.extend_from_slice(vps);
        out.extend_from_slice(&NAL_START_CODE);
        out.extend_from_slice(sps);
        out.extend_from_slice(&NAL_START_CODE);
        out.extend_from_slice(pps);
        return Ok((true, out));
    }

    // H.264 AVCC configuration record
    let sps_size = u16::from_be_bytes([payload[6], payload[7]]) as usize;
    if 8 + sps_size + 3 > payload.len() {
        return Err(format!("Truncated H.264 SPS (sps_size={}, len={})", sps_size, payload.len()));
    }
    let sps = &payload[8..8 + sps_size];
    let pps_offset = 8 + sps_size + 1;
    let pps_size = u16::from_be_bytes([payload[pps_offset], payload[pps_offset + 1]]) as usize;
    if pps_offset + 2 + pps_size > payload.len() {
        return Err(format!("Truncated H.264 PPS (pps_size={}, len={})", pps_size, payload.len()));
    }
    let pps = &payload[pps_offset + 2..pps_offset + 2 + pps_size];

    let mut out = Vec::with_capacity(sps_size + pps_size + 8);
    out.extend_from_slice(&NAL_START_CODE);
    out.extend_from_slice(sps);
    out.extend_from_slice(&NAL_START_CODE);
    out.extend_from_slice(pps);
    Ok((false, out))
}

/// Spawns the TCP Mirroring video stream listener.
/// Returns the bound local TCP port and a shutdown sender.
pub async fn spawn_mirror_receiver(
    preferred_port: u16,
    aeskey: [u8; 16],
    stream_connection_id: u64,
    ipc: IpcHub,
) -> std::io::Result<(u16, watch::Sender<bool>)> {
    let listener = match TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], preferred_port))).await {
        Ok(l) => l,
        Err(_) => TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?,
    };
    let actual_port = listener.local_addr()?.port();
    let (stop_tx, mut stop_rx) = watch::channel(false);

    eprintln!(
        "[Mirror] Listening for video stream on TCP port {} (streamID={})",
        actual_port, stream_connection_id
    );

    tokio::spawn(async move {
        let mut cipher = derive_video_cipher(&aeskey, stream_connection_id);
        let mut pending_sps_pps: Vec<u8> = Vec::new();
        let mut sps_pps_ts: u64 = 0;
        let mut is_h265 = false;

        let accept_res = tokio::select! {
            _ = stop_rx.changed() => return,
            res = listener.accept() => res,
        };

        let (mut stream, peer) = match accept_res {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[Mirror] Accept failed: {}", e);
                return;
            }
        };
        let _ = stream.set_nodelay(true);
        eprintln!("[Mirror] Video stream connected from {}", peer);

        let mut header = [0u8; 128];
        let mut stats_start = Instant::now();
        let mut frames_window: u64 = 0;
        let mut bytes_window: u64 = 0;
        let mut current_width: u32 = 0;
        let mut current_height: u32 = 0;
        let mut invalid_count: u64 = 0;
        let mut last_invalid_log = Instant::now();

        loop {
            let read_hdr = tokio::select! {
                _ = stop_rx.changed() => break,
                r = stream.read_exact(&mut header) => r,
            };
            if let Err(e) = read_hdr {
                eprintln!("[Mirror] Video stream ended: {}", e);
                break;
            }

            let payload_size = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
            let payload_type = header[4];
            let payload_opt = header[6];
            let ts_raw = u64::from_le_bytes([
                header[8], header[9], header[10], header[11],
                header[12], header[13], header[14], header[15],
            ]);

            if payload_size > 16 * 1024 * 1024 {
                eprintln!("[Mirror] Abnormal payload size {}, closing", payload_size);
                break;
            }

            let mut payload = vec![0u8; payload_size];
            if payload_size > 0 {
                let read_body = tokio::select! {
                    _ = stop_rx.changed() => break,
                    r = stream.read_exact(&mut payload) => r,
                };
                if let Err(e) = read_body {
                    eprintln!("[Mirror] Video payload read ended: {}", e);
                    break;
                }
            }

            match payload_type {
                0x00 => {
                    // Encrypted VCL NAL units
                    cipher.apply_keystream(&mut payload);

                    // Convert 4-byte big-endian length prefixes to Annex-B start codes
                    let mut pos = 0usize;
                    let mut valid = true;
                    while pos + 4 <= payload.len() {
                        let nal_len = u32::from_be_bytes([
                            payload[pos],
                            payload[pos + 1],
                            payload[pos + 2],
                            payload[pos + 3],
                        ]) as usize;
                        if pos + 4 + nal_len > payload.len() {
                            valid = false;
                            break;
                        }
                        payload[pos..pos + 4].copy_from_slice(&NAL_START_CODE);
                        pos += 4;
                        if nal_len > 0 && (payload[pos] & 0x80) != 0 {
                            valid = false;
                            break;
                        }
                        pos += nal_len;
                    }
                    if pos != payload.len() {
                        valid = false;
                    }

                    if !valid {
                        invalid_count += 1;
                        if invalid_count == 1 || last_invalid_log.elapsed() >= Duration::from_secs(2) {
                            eprintln!(
                                "[Mirror] Warning: invalid decrypted NALU packet (len={}, total invalid={}). \
                                 If this repeats for every packet, the AES key/streamID is wrong.",
                                payload.len(),
                                invalid_count
                            );
                            last_invalid_log = Instant::now();
                        }
                        continue;
                    }

                    let out_data = if !pending_sps_pps.is_empty() {
                        let mut combined = Vec::with_capacity(pending_sps_pps.len() + payload.len());
                        if ts_raw == sps_pps_ts || true {
                            combined.extend_from_slice(&pending_sps_pps);
                        }
                        combined.extend_from_slice(&payload);
                        pending_sps_pps.clear();
                        combined
                    } else {
                        payload
                    };

                    // Convert Q32.32 NTP timestamp to nanoseconds
                    let ts_ns = ((ts_raw >> 32) * 1_000_000_000)
                        + (((ts_raw & 0xFFFF_FFFF) * 1_000_000_000) >> 32);

                    bytes_window += out_data.len() as u64;
                    frames_window += 1;

                    ipc.send_video(is_h265, ts_ns, out_data);

                    let elapsed = stats_start.elapsed();
                    if elapsed >= Duration::from_millis(500) {
                        let secs = elapsed.as_secs_f64();
                        let fps = frames_window as f64 / secs;
                        let mbps = (bytes_window as f64 * 8.0) / (secs * 1_000_000.0);
                        ipc.send_event(&json!({
                            "type": "video_stats",
                            "codec": if is_h265 { "HEVC (H.265)" } else { "AVC (H.264)" },
                            "width": current_width,
                            "height": current_height,
                            "fps": (fps * 10.0).round() / 10.0,
                            "bitrate_mbps": (mbps * 100.0).round() / 100.0,
                        }));
                        stats_start = Instant::now();
                        frames_window = 0;
                        bytes_window = 0;
                    }
                }
                0x01 => {
                    // Unencrypted codec config packet (SPS+PPS or VPS+SPS+PPS) + resolution metadata
                    let w = f32::from_le_bytes([header[56], header[57], header[58], header[59]]) as u32;
                    let h = f32::from_le_bytes([header[60], header[61], header[62], header[63]]) as u32;
                    if w > 0 && h > 0 {
                        current_width = w;
                        current_height = h;
                    }

                    if payload_opt == 0x56 || payload_opt == 0x5e {
                        ipc.send_event(&json!({ "type": "video_paused" }));
                    }

                    if payload_size == 0 {
                        eprintln!("[Mirror] Empty codec packet (client tried HEVC while bit 42 disabled?)");
                        continue;
                    }

                    match parse_codec_packet(&payload) {
                        Ok((h265, annexb)) => {
                            is_h265 = h265;
                            pending_sps_pps = annexb;
                            sps_pps_ts = ts_raw;
                            eprintln!(
                                "[Mirror] Codec config: {} {}x{} ({} bytes)",
                                if is_h265 { "H.265/HEVC" } else { "H.264/AVC" },
                                current_width,
                                current_height,
                                pending_sps_pps.len()
                            );
                            ipc.send_event(&json!({
                                "type": "video_format",
                                "codec": if is_h265 { "HEVC (H.265)" } else { "AVC (H.264)" },
                                "width": current_width,
                                "height": current_height,
                            }));
                        }
                        Err(e) => {
                            eprintln!("[Mirror] Failed to parse codec packet: {}", e);
                        }
                    }
                }
                0x02 | 0x05 => {
                    // Heartbeat / streaming report packet
                }
                other => {
                    eprintln!("[Mirror] Unknown packet type 0x{:02x} len={}", other, payload_size);
                }
            }
        }

        ipc.send_event(&json!({ "type": "video_stopped" }));
    });

    Ok((actual_port, stop_tx))
}
