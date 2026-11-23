mod audio;
mod config;
mod fairplay;
mod ipc;
mod mdns;
mod mirror;
mod ntp;
mod pairing;
mod playfair;
mod rtsp;

use clap::Parser;
use config::EngineConfig;
use ipc::IpcHub;
use mdns::MdnsAdvertiser;
use pairing::PairingIdentity;
use rtsp::{run_rtsp_server, RtspSharedState};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let cfg = EngineConfig::parse();
    let identity = PairingIdentity::generate();
    let pk_hex = identity.public_key_hex();

    let mdns = match MdnsAdvertiser::start(&cfg, &pk_hex) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[FATAL] {}", e);
            std::process::exit(1);
        }
    };

    let ipc = IpcHub::new();

    let ready_event = json!({
        "type": "engine_ready",
        "name": cfg.name,
        "local_ip": mdns.local_ip,
        "rtsp_port": cfg.rtsp_port,
        "width": cfg.width,
        "height": cfg.height,
        "max_fps": cfg.max_fps,
        "refresh_rate": cfg.refresh_rate,
        "hevc": cfg.hevc,
        "overscanned": cfg.overscanned,
    });

    let ipc_srv = ipc.clone();
    let ipc_port = cfg.ipc_port;
    tokio::spawn(async move {
        if let Err(e) = ipc_srv.run_server(ipc_port, ready_event).await {
            eprintln!("[IPC] Server error: {}", e);
        }
    });

    let rtsp_state = Arc::new(RtspSharedState {
        config: cfg.clone(),
        identity,
        airplay_txt_raw: mdns.airplay_txt_raw.clone(),
        raop_txt_raw: mdns.raop_txt_raw.clone(),
        ipc: ipc.clone(),
    });

    let rtsp_handle = tokio::spawn(async move {
        if let Err(e) = run_rtsp_server(rtsp_state).await {
            eprintln!("[RTSP] Server error: {}", e);
        }
    });

    // Wait for Ctrl+C to cleanly unregister mDNS before exiting
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("[Engine] Shutting down...");
    rtsp_handle.abort();
    mdns.shutdown();
}
