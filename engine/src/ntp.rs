use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::sync::watch;

const SECONDS_1900_TO_1970: u64 = 2_208_988_800;

pub fn local_time_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

fn ns_to_ntp_q32_32(ns: u64) -> u64 {
    let secs = (ns / 1_000_000_000) + SECONDS_1900_TO_1970;
    let rem_ns = ns % 1_000_000_000;
    let frac = ((rem_ns as u128) << 32) / 1_000_000_000u128;
    (secs << 32) | (frac as u64 & 0xFFFF_FFFF)
}

/// Spawns a UDP NTP timing sync task matching AirPlay's timing protocol.
/// Returns the bound local port and a shutdown sender.
pub async fn spawn_ntp_sync(
    local_port: u16,
    remote_ip: std::net::IpAddr,
    remote_timing_port: u16,
) -> std::io::Result<(u16, watch::Sender<bool>)> {
    let bind_addr = SocketAddr::from(([0, 0, 0, 0], local_port));
    let sock = match UdpSocket::bind(bind_addr).await {
        Ok(s) => s,
        Err(_) => UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?,
    };
    let actual_port = sock.local_addr()?.port();
    let (stop_tx, mut stop_rx) = watch::channel(false);

    if remote_timing_port != 0 {
        let remote_addr = SocketAddr::new(remote_ip, remote_timing_port);
        tokio::spawn(async move {
            let mut req = [0u8; 32];
            req[0] = 0x80;
            req[1] = 0xd2;
            req[2] = 0x00;
            req[3] = 0x07;
            let mut resp = [0u8; 128];

            loop {
                if *stop_rx.borrow() {
                    break;
                }
                let now_ntp = ns_to_ntp_q32_32(local_time_ns());
                req[24..32].copy_from_slice(&now_ntp.to_be_bytes());
                let _ = sock.send_to(&req, remote_addr).await;

                tokio::select! {
                    _ = stop_rx.changed() => break,
                    res = tokio::time::timeout(Duration::from_millis(400), sock.recv_from(&mut resp)) => {
                        if let Ok(Ok((n, _))) = res {
                            if n >= 32 {
                                // Copy client's t3 into next request's reference field
                                req[8..16].copy_from_slice(&resp[24..32]);
                                let recv_ntp = ns_to_ntp_q32_32(local_time_ns());
                                req[16..24].copy_from_slice(&recv_ntp.to_be_bytes());
                            }
                        }
                    }
                }

                tokio::select! {
                    _ = stop_rx.changed() => break,
                    _ = tokio::time::sleep(Duration::from_secs(3)) => {}
                }
            }
        });
    }

    Ok((actual_port, stop_tx))
}
