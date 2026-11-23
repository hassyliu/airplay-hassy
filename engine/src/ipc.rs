use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

/// Wire framing sent over localhost TCP IPC from Rust engine -> Python UI:
///   [u8  msg_type]
///   [u8  flags / codec_id]
///   [u64 timestamp_ns (little-endian)]
///   [u32 payload_len (little-endian)]
///   [payload bytes...]
///
/// msg_type:
///   1 = Video Annex-B NALU packet (codec_id: 1 = H.264, 2 = H.265/HEVC)
///   2 = Audio packet (codec_id: 2 = ALAC, 4 = AAC-LC, 8 = AAC-ELD, 1 = LPCM)
///   3 = JSON Event / Telemetry UTF-8 string
#[derive(Clone, Debug)]
pub struct IpcPacket {
    pub msg_type: u8,
    pub codec_id: u8,
    pub timestamp_ns: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone)]
pub struct IpcHub {
    tx: broadcast::Sender<IpcPacket>,
}

impl IpcHub {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(512);
        Self { tx }
    }

    pub fn send_video(&self, is_h265: bool, timestamp_ns: u64, annexb: Vec<u8>) {
        let _ = self.tx.send(IpcPacket {
            msg_type: 1,
            codec_id: if is_h265 { 2 } else { 1 },
            timestamp_ns,
            payload: annexb,
        });
    }

    pub fn send_audio(&self, ct: u8, timestamp_ns: u64, frame: Vec<u8>) {
        let _ = self.tx.send(IpcPacket {
            msg_type: 2,
            codec_id: ct,
            timestamp_ns,
            payload: frame,
        });
    }

    pub fn send_event<T: Serialize>(&self, event: &T) {
        if let Ok(json) = serde_json::to_vec(event) {
            let _ = self.tx.send(IpcPacket {
                msg_type: 3,
                codec_id: 0,
                timestamp_ns: 0,
                payload: json,
            });
        }
    }

    pub async fn run_server(&self, port: u16, ready_event: serde_json::Value) -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", port)).await?;
        eprintln!("[IPC] Listening on 127.0.0.1:{}", port);

        loop {
            let (mut socket, peer) = listener.accept().await?;
            let _ = socket.set_nodelay(true);
            eprintln!("[IPC] UI connected from {}", peer);
            let mut rx = self.tx.subscribe();
            let init_evt = ready_event.clone();

            tokio::spawn(async move {
                // Send immediate ready snapshot to newly connected UI
                if let Ok(json) = serde_json::to_vec(&init_evt) {
                    let pkt = IpcPacket {
                        msg_type: 3,
                        codec_id: 0,
                        timestamp_ns: 0,
                        payload: json,
                    };
                    if write_packet(&mut socket, &pkt).await.is_err() {
                        return;
                    }
                }

                loop {
                    match rx.recv().await {
                        Ok(pkt) => {
                            if write_packet(&mut socket, &pkt).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            eprintln!("[IPC] Client lagged, skipped {} packets", skipped);
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                eprintln!("[IPC] UI disconnected: {}", peer);
            });
        }
    }
}

async fn write_packet(
    socket: &mut tokio::net::TcpStream,
    pkt: &IpcPacket,
) -> std::io::Result<()> {
    let mut hdr = [0u8; 14];
    hdr[0] = pkt.msg_type;
    hdr[1] = pkt.codec_id;
    hdr[2..10].copy_from_slice(&pkt.timestamp_ns.to_le_bytes());
    hdr[10..14].copy_from_slice(&(pkt.payload.len() as u32).to_le_bytes());
    socket.write_all(&hdr).await?;
    socket.write_all(&pkt.payload).await?;
    Ok(())
}
