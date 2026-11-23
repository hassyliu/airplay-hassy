use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(name = "airplay-hassy-engine", about = "High-performance AirPlay Mirroring & Audio Receiver Engine")]
pub struct EngineConfig {
    /// Device display name shown in iOS Screen Mirroring list
    #[arg(long, default_value = "AirPlay-Hassy")]
    pub name: String,

    /// Advertised display width in pixels
    #[arg(long, default_value_t = 1920)]
    pub width: u32,

    /// Advertised display height in pixels
    #[arg(long, default_value_t = 1080)]
    pub height: u32,

    /// Maximum frame rate advertised to iOS (e.g. 30, 60, 120)
    #[arg(long, default_value_t = 60)]
    pub max_fps: u32,

    /// Display refresh rate in Hz (sent as 1.0 / refresh_rate in /info plist)
    #[arg(long, default_value_t = 60)]
    pub refresh_rate: u32,

    /// Enable H.265 / HEVC (AirPlay Features Bit 42: SupportsScreenMultiCodec)
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub hevc: bool,

    /// Advertise overscanned display flag
    #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
    pub overscanned: bool,

    /// RTSP / AirPlay control TCP port
    #[arg(long, default_value_t = 7000)]
    pub rtsp_port: u16,

    /// Mirror video stream TCP port
    #[arg(long, default_value_t = 7100)]
    pub mirror_port: u16,

    /// Audio RTP UDP data port
    #[arg(long, default_value_t = 6000)]
    pub audio_data_port: u16,

    /// Audio RTCP UDP control port
    #[arg(long, default_value_t = 6001)]
    pub audio_control_port: u16,

    /// NTP UDP timing port
    #[arg(long, default_value_t = 7010)]
    pub timing_port: u16,

    /// Localhost IPC TCP port to stream decoded media & telemetry to UI
    #[arg(long, default_value_t = 17890)]
    pub ipc_port: u16,

    /// Optional custom 6-byte hardware MAC address (e.g. AA:BB:CC:DD:EE:FF)
    #[arg(long, default_value = "48:5D:60:7C:EE:22")]
    pub mac: String,
}

impl EngineConfig {
    pub fn features1(&self) -> u32 {
        // UxPlay default FEATURES_1 with bit 27 (legacy pairing) ON: 0x5A7FFEE6
        0x5A7F_FEE6
    }

    pub fn features2(&self) -> u32 {
        // Bit 42 (42 - 32 = 10 => 0x400) enables SupportsScreenMultiCodec (H.265 / HEVC)
        if self.hevc {
            1u32 << (42 - 32)
        } else {
            0
        }
    }

    pub fn features_u64(&self) -> u64 {
        ((self.features2() as u64) << 32) | (self.features1() as u64)
    }

    pub fn features_txt(&self) -> String {
        format!("0x{:X},0x{:X}", self.features1(), self.features2())
    }

    pub fn mac_bytes(&self) -> [u8; 6] {
        let parts: Vec<u8> = self
            .mac
            .split(':')
            .filter_map(|p| u8::from_str_radix(p.trim(), 16).ok())
            .collect();
        if parts.len() == 6 {
            [parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]]
        } else {
            [0x48, 0x5D, 0x60, 0x7C, 0xEE, 0x22]
        }
    }

    pub fn mac_colon_upper(&self) -> String {
        let b = self.mac_bytes();
        format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        )
    }

    pub fn mac_compact_upper(&self) -> String {
        let b = self.mac_bytes();
        format!(
            "{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            b[0], b[1], b[2], b[3], b[4], b[5]
        )
    }
}
