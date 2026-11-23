use crate::config::EngineConfig;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;
use std::net::IpAddr;

pub const GLOBAL_MODEL: &str = "AppleTV3,2";
pub const GLOBAL_VERSION: &str = "220.68";
pub const AIRPLAY_PI: &str = "2e388006-13ba-4041-9a67-25dd4a43d536";

pub struct MdnsAdvertiser {
    daemon: ServiceDaemon,
    airplay_fullname: String,
    raop_fullname: String,
    pub airplay_txt_raw: Vec<u8>,
    pub raop_txt_raw: Vec<u8>,
    pub local_ip: String,
}

fn encode_dns_txt(entries: &[(&str, String)]) -> Vec<u8> {
    let mut buf = Vec::new();
    for (k, v) in entries {
        let item = format!("{}={}", k, v);
        let bytes = item.as_bytes();
        if bytes.len() <= 255 {
            buf.push(bytes.len() as u8);
            buf.extend_from_slice(bytes);
        }
    }
    buf
}

impl MdnsAdvertiser {
    pub fn start(cfg: &EngineConfig, pk_hex: &str) -> Result<Self, String> {
        let daemon = ServiceDaemon::new().map_err(|e| format!("mDNS daemon init failed: {}", e))?;

        let local_ip = local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let features_str = cfg.features_txt();
        let device_id = cfg.mac_colon_upper();
        let raop_name = format!("{}@{}", cfg.mac_compact_upper(), cfg.name);

        let airplay_entries: Vec<(&str, String)> = vec![
            ("deviceid", device_id.clone()),
            ("features", features_str.clone()),
            ("flags", "0x4".to_string()),
            ("model", GLOBAL_MODEL.to_string()),
            ("pk", pk_hex.to_string()),
            ("pi", AIRPLAY_PI.to_string()),
            ("pw", "false".to_string()),
            ("srcvers", GLOBAL_VERSION.to_string()),
            ("vv", "2".to_string()),
        ];

        let raop_entries: Vec<(&str, String)> = vec![
            ("ch", "2".to_string()),
            ("cn", "0,1,2,3".to_string()),
            ("da", "true".to_string()),
            ("et", "0,3,5".to_string()),
            ("vv", "2".to_string()),
            ("ft", features_str.clone()),
            ("am", GLOBAL_MODEL.to_string()),
            ("md", "0,1,2".to_string()),
            ("rhd", "5.6.0.0".to_string()),
            ("pw", "false".to_string()),
            ("sf", "0x4".to_string()),
            ("sr", "44100".to_string()),
            ("ss", "16".to_string()),
            ("sv", "false".to_string()),
            ("tp", "UDP".to_string()),
            ("txtvers", "1".to_string()),
            ("vs", GLOBAL_VERSION.to_string()),
            ("vn", "65537".to_string()),
            ("pk", pk_hex.to_string()),
        ];

        let airplay_txt_raw = encode_dns_txt(&airplay_entries);
        let raop_txt_raw = encode_dns_txt(&raop_entries);

        let airplay_props: HashMap<String, String> = airplay_entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let raop_props: HashMap<String, String> = raop_entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        // Build clean hostname (alphanumeric + hyphen)
        let host_label: String = cfg
            .name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let host_name = format!("{}.local.", if host_label.is_empty() { "airplay-hassy" } else { &host_label });

        let ip_addr: IpAddr = local_ip.parse().unwrap_or(IpAddr::from([127, 0, 0, 1]));

        let airplay_info = ServiceInfo::new(
            "_airplay._tcp.local.",
            &cfg.name,
            &host_name,
            ip_addr,
            cfg.rtsp_port,
            Some(airplay_props),
        )
        .map_err(|e| format!("Failed creating _airplay._tcp ServiceInfo: {}", e))?
        .enable_addr_auto();

        let raop_info = ServiceInfo::new(
            "_raop._tcp.local.",
            &raop_name,
            &host_name,
            ip_addr,
            cfg.rtsp_port,
            Some(raop_props),
        )
        .map_err(|e| format!("Failed creating _raop._tcp ServiceInfo: {}", e))?
        .enable_addr_auto();

        let airplay_fullname = airplay_info.get_fullname().to_string();
        let raop_fullname = raop_info.get_fullname().to_string();

        daemon
            .register(airplay_info)
            .map_err(|e| format!("mDNS register _airplay._tcp failed: {}", e))?;
        daemon
            .register(raop_info)
            .map_err(|e| format!("mDNS register _raop._tcp failed: {}", e))?;

        eprintln!(
            "[mDNS] Broadcasting '{}' on {} port {} (HEVC={}, features={})",
            cfg.name, local_ip, cfg.rtsp_port, cfg.hevc, features_str
        );

        Ok(Self {
            daemon,
            airplay_fullname,
            raop_fullname,
            airplay_txt_raw,
            raop_txt_raw,
            local_ip,
        })
    }

    pub fn shutdown(self) {
        let _ = self.daemon.unregister(&self.airplay_fullname);
        let _ = self.daemon.unregister(&self.raop_fullname);
        let _ = self.daemon.shutdown();
    }
}
