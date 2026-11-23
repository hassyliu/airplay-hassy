use crate::audio::spawn_audio_receiver;
use crate::config::EngineConfig;
use crate::fairplay::FairPlaySession;
use crate::ipc::IpcHub;
use crate::mdns::{AIRPLAY_PI, GLOBAL_MODEL, GLOBAL_VERSION};
use crate::mirror::spawn_mirror_receiver;
use crate::ntp::spawn_ntp_sync;
use crate::pairing::{PairingIdentity, PairingSession};
use plist::{Dictionary, Value};
use serde_json::json;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

pub struct RtspSharedState {
    pub config: EngineConfig,
    pub identity: PairingIdentity,
    pub airplay_txt_raw: Vec<u8>,
    pub raop_txt_raw: Vec<u8>,
    pub ipc: IpcHub,
}

struct RtspRequest {
    method: String,
    url: String,
    protocol: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct ActiveSession {
    pairing: PairingSession,
    fairplay: FairPlaySession,
    aeskey: Option<[u8; 16]>,
    aesiv: Option<[u8; 16]>,
    ntp_stop: Option<watch::Sender<bool>>,
    mirror_stop: Option<watch::Sender<bool>>,
    audio_stop: Option<watch::Sender<bool>>,
    client_name: String,
    client_model: String,
    volume: f64,
}

impl ActiveSession {
    fn new(identity: &PairingIdentity) -> Self {
        Self {
            pairing: PairingSession::new(identity),
            fairplay: FairPlaySession::new(),
            aeskey: None,
            aesiv: None,
            ntp_stop: None,
            mirror_stop: None,
            audio_stop: None,
            client_name: String::new(),
            client_model: String::new(),
            volume: -15.0,
        }
    }

    fn stop_all(&mut self) {
        if let Some(tx) = self.mirror_stop.take() {
            let _ = tx.send(true);
        }
        if let Some(tx) = self.audio_stop.take() {
            let _ = tx.send(true);
        }
        if let Some(tx) = self.ntp_stop.take() {
            let _ = tx.send(true);
        }
    }
}

impl Drop for ActiveSession {
    fn drop(&mut self) {
        self.stop_all();
    }
}

pub async fn run_rtsp_server(state: Arc<RtspSharedState>) -> std::io::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], state.config.rtsp_port));
    let listener = TcpListener::bind(addr).await?;
    eprintln!("[RTSP] Listening on 0.0.0.0:{}", state.config.rtsp_port);

    loop {
        let (stream, peer) = listener.accept().await?;
        let _ = stream.set_nodelay(true);
        let st = Arc::clone(&state);
        tokio::spawn(async move {
            eprintln!("[RTSP] Client connected: {}", peer);
            if let Err(e) = handle_connection(stream, peer, st).await {
                eprintln!("[RTSP] Connection {} closed: {}", peer, e);
            }
        });
    }
}

async fn read_rtsp_request(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> std::io::Result<Option<RtspRequest>> {
    let mut first_line = String::new();
    let n = reader.read_line(&mut first_line).await?;
    if n == 0 {
        return Ok(None);
    }
    let trimmed = first_line.trim_end_matches(&['\r', '\n'][..]);
    if trimmed.is_empty() {
        return Ok(None);
    }

    let mut parts = trimmed.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let url = parts.next().unwrap_or("").to_string();
    let protocol = parts.next().unwrap_or("RTSP/1.0").to_string();

    let mut headers = HashMap::new();
    let mut content_length = 0usize;

    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }
        let t = line.trim_end_matches(&['\r', '\n'][..]);
        if t.is_empty() {
            break;
        }
        if let Some((k, v)) = t.split_once(':') {
            let key_lower = k.trim().to_ascii_lowercase();
            let val = v.trim().to_string();
            if key_lower == "content-length" {
                content_length = val.parse().unwrap_or(0);
            }
            headers.insert(key_lower, val);
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await?;
    }

    Ok(Some(RtspRequest {
        method,
        url,
        protocol,
        headers,
        body,
    }))
}

fn plist_to_binary(dict: Dictionary) -> Vec<u8> {
    let mut buf = Vec::new();
    let val = Value::Dictionary(dict);
    let _ = plist::to_writer_binary(&mut buf, &val);
    buf
}

fn build_info_response(state: &RtspSharedState, req: &RtspRequest) -> Vec<u8> {
    let mut res = Dictionary::new();
    let content_type = req.headers.get("content-type").map(|s| s.as_str());
    let cseq = req.headers.get("cseq");

    let mut add_txt_airplay = false;
    let mut add_txt_raop = false;

    if let Some(ct) = content_type {
        if ct.contains("apple-binary-plist") && !req.body.is_empty() {
            if let Ok(Value::Dictionary(d)) = plist::from_reader(Cursor::new(&req.body)) {
                if let Some(Value::Array(arr)) = d.get("qualifier") {
                    for item in arr {
                        if let Value::String(s) = item {
                            if s == "txtAirPlay" {
                                add_txt_airplay = true;
                            } else if s == "txtRAOP" {
                                add_txt_raop = true;
                            }
                        }
                    }
                }
            }
        }
    }

    if cseq.is_none() {
        if req.url.contains("txtAirPlay") {
            add_txt_airplay = true;
        }
        if req.url.contains("txtRAOP") {
            add_txt_raop = true;
        }
    }

    if add_txt_airplay {
        res.insert("txtAirPlay".into(), Value::Data(state.airplay_txt_raw.clone()));
    }
    if add_txt_raop {
        res.insert("txtRAOP".into(), Value::Data(state.raop_txt_raw.clone()));
    }
    if content_type.is_some() {
        return plist_to_binary(res);
    }

    let cfg = &state.config;
    let mac = cfg.mac_colon_upper();
    res.insert("deviceID".into(), Value::String(mac.clone()));
    res.insert("macAddress".into(), Value::String(mac));
    res.insert("pk".into(), Value::Data(state.identity.public_key_bytes().to_vec()));
    res.insert("features".into(), Value::Integer(cfg.features_u64().into()));
    res.insert("name".into(), Value::String(cfg.name.clone()));
    res.insert("pi".into(), Value::String(AIRPLAY_PI.into()));
    res.insert("vv".into(), Value::Integer(2u64.into()));
    res.insert("statusFlags".into(), Value::Integer(68u64.into()));
    res.insert("keepAliveLowPower".into(), Value::Boolean(true));
    res.insert("sourceVersion".into(), Value::String(GLOBAL_VERSION.into()));
    res.insert("keepAliveSendStatsAsBody".into(), Value::Boolean(true));
    res.insert("model".into(), Value::String(GLOBAL_MODEL.into()));

    if cseq.is_none() {
        return plist_to_binary(res);
    }

    res.insert("initialVolume".into(), Value::Real(-15.0));

    // audioLatencies
    let mut audio_latencies = Vec::new();
    for t in [100u64, 101u64] {
        let mut d = Dictionary::new();
        d.insert("type".into(), Value::Integer(t.into()));
        d.insert("audioType".into(), Value::String("default".into()));
        d.insert("inputLatencyMicros".into(), Value::Integer(0u64.into()));
        d.insert("outputLatencyMicros".into(), Value::Boolean(false));
        audio_latencies.push(Value::Dictionary(d));
    }
    res.insert("audioLatencies".into(), Value::Array(audio_latencies));

    // audioFormats
    let mut audio_formats = Vec::new();
    for t in [100u64, 101u64] {
        let mut d = Dictionary::new();
        d.insert("type".into(), Value::Integer(t.into()));
        d.insert("audioInputFormats".into(), Value::Integer(0x3fffffcu64.into()));
        d.insert("audioOutputFormats".into(), Value::Integer(0x3fffffcu64.into()));
        audio_formats.push(Value::Dictionary(d));
    }
    res.insert("audioFormats".into(), Value::Array(audio_formats));

    // Custom display capabilities (controls iOS output resolution, FPS & refresh rate!)
    let mut disp = Dictionary::new();
    disp.insert("uuid".into(), Value::String("e0ff8a27-6738-3d56-8a16-cc53aacee925".into()));
    disp.insert("widthPhysical".into(), Value::Integer(0u64.into()));
    disp.insert("heightPhysical".into(), Value::Integer(0u64.into()));
    disp.insert("width".into(), Value::Integer((cfg.width as u64).into()));
    disp.insert("height".into(), Value::Integer((cfg.height as u64).into()));
    disp.insert("widthPixels".into(), Value::Integer((cfg.width as u64).into()));
    disp.insert("heightPixels".into(), Value::Integer((cfg.height as u64).into()));
    disp.insert("rotation".into(), Value::Boolean(false));
    let rr = if cfg.refresh_rate > 0 {
        1.0 / (cfg.refresh_rate as f64)
    } else {
        1.0 / 60.0
    };
    disp.insert("refreshRate".into(), Value::Real(rr));
    disp.insert("maxFPS".into(), Value::Integer((cfg.max_fps as u64).into()));
    disp.insert("overscanned".into(), Value::Boolean(cfg.overscanned));
    disp.insert("features".into(), Value::Integer(14u64.into()));
    res.insert("displays".into(), Value::Array(vec![Value::Dictionary(disp)]));

    plist_to_binary(res)
}

async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    state: Arc<RtspSharedState>,
) -> std::io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut session = ActiveSession::new(&state.identity);

    loop {
        let req = match read_rtsp_request(&mut reader).await? {
            Some(r) => r,
            None => break,
        };

        let cseq = req.headers.get("cseq").cloned();
        let mut status_code = 200u16;
        let mut status_text = "OK";
        let mut extra_headers: Vec<(String, String)> = Vec::new();
        let mut resp_body: Vec<u8> = Vec::new();
        let mut should_close = false;

        if cseq.is_some() && req.method != "RECORD" {
            extra_headers.push(("Audio-Jack-Status".into(), "connected; type=digital".into()));
        }

        match (req.method.as_str(), req.url.as_str()) {
            ("GET", u) if u.contains("/info") => {
                resp_body = build_info_response(&state, &req);
                extra_headers.push(("Content-Type".into(), "application/x-apple-binary-plist".into()));
            }
            ("POST", "/pair-setup") => {
                let pk = state.identity.public_key_bytes().to_vec();
                resp_body = pk;
                extra_headers.push(("Content-Type".into(), "application/octet-stream".into()));
            }
            ("POST", "/pair-verify") => {
                if req.body.len() >= 4 {
                    match req.body[0] {
                        1 => match session.pairing.verify_step1(&req.body) {
                            Ok(reply) => {
                                resp_body = reply;
                                extra_headers.push(("Content-Type".into(), "application/octet-stream".into()));
                            }
                            Err(e) => {
                                eprintln!("[RTSP] pair-verify step1 error: {}", e);
                                status_code = 400;
                                status_text = "Bad Request";
                            }
                        },
                        0 => match session.pairing.verify_step2(&req.body) {
                            Ok(()) => {
                                extra_headers.push(("Content-Type".into(), "application/octet-stream".into()));
                            }
                            Err(e) => {
                                eprintln!("[RTSP] pair-verify step2 error: {}", e);
                                status_code = 401;
                                status_text = "Unauthorized";
                                should_close = true;
                            }
                        },
                        _ => {
                            status_code = 400;
                            status_text = "Bad Request";
                        }
                    }
                }
            }
            ("POST", "/fp-setup") => match session.fairplay.handle_fp_setup(&req.body) {
                Ok(reply) => {
                    resp_body = reply;
                    extra_headers.push(("Content-Type".into(), "application/octet-stream".into()));
                }
                Err(e) => {
                    eprintln!("[RTSP] fp-setup error: {}", e);
                    status_code = 400;
                    status_text = "Bad Request";
                }
            },
            ("OPTIONS", _) => {
                extra_headers.push((
                    "Public".into(),
                    "SETUP, RECORD, FLUSH, TEARDOWN, OPTIONS, GET_PARAMETER, SET_PARAMETER".into(),
                ));
            }
            ("SETUP", _) => {
                if let Ok(Value::Dictionary(req_dict)) = plist::from_reader(Cursor::new(&req.body)) {
                    let mut res_dict = Dictionary::new();

                    // Stage 1 SETUP: ekey + eiv + timingPort
                    if let (Some(Value::Data(ekey)), Some(Value::Data(eiv))) =
                        (req_dict.get("ekey"), req_dict.get("eiv"))
                    {
                        if let Some(Value::String(name)) = req_dict.get("name") {
                            session.client_name = name.clone();
                        }
                        if let Some(Value::String(model)) = req_dict.get("model") {
                            session.client_model = model.clone();
                        }

                        eprintln!(
                            "[RTSP] SETUP 1 from '{}' ({}) @ {}",
                            session.client_name, session.client_model, peer
                        );

                        state.ipc.send_event(&json!({
                            "type": "client_connected",
                            "name": session.client_name,
                            "model": session.client_model,
                            "ip": peer.ip().to_string(),
                        }));

                        match session.fairplay.decrypt_ekey(ekey) {
                            Ok(mut aeskey) => {
                                // Hash aeskey with shared ECDH secret using SHA-512 (UxPlay raop_handlers.h:889)
                                if let Some(ecdh_secret) = session.pairing.ecdh_secret() {
                                    let mut hasher = Sha512::new();
                                    hasher.update(aeskey);
                                    hasher.update(ecdh_secret);
                                    let digest = hasher.finalize();
                                    aeskey.copy_from_slice(&digest[..16]);
                                }
                                let mut iv_arr = [0u8; 16];
                                if eiv.len() == 16 {
                                    iv_arr.copy_from_slice(eiv);
                                }
                                session.aeskey = Some(aeskey);
                                session.aesiv = Some(iv_arr);

                                let timing_rport = req_dict
                                    .get("timingPort")
                                    .and_then(|v| v.as_unsigned_integer())
                                    .unwrap_or(0) as u16;

                                if let Ok((tport, stop_tx)) = spawn_ntp_sync(
                                    state.config.timing_port,
                                    peer.ip(),
                                    timing_rport,
                                )
                                .await
                                {
                                    session.ntp_stop = Some(stop_tx);
                                    res_dict.insert("timingPort".into(), Value::Integer((tport as u64).into()));
                                    res_dict.insert("eventPort".into(), Value::Integer(0u64.into()));
                                }
                            }
                            Err(e) => {
                                eprintln!("[RTSP] FairPlay decrypt failed: {}", e);
                                status_code = 400;
                                status_text = "Bad Request";
                            }
                        }
                    }

                    // Stage 2 SETUP: streams (type 110 = video mirroring, type 96 = audio)
                    if let Some(Value::Array(streams)) = req_dict.get("streams") {
                        let mut res_streams = Vec::new();
                        for s in streams {
                            if let Value::Dictionary(sd) = s {
                                let stype = sd
                                    .get("type")
                                    .and_then(|v| v.as_unsigned_integer())
                                    .unwrap_or(0);
                                match stype {
                                    110 => {
                                        // iOS often sends streamConnectionID as a *negative* int64 in the plist.
                                        // `as_unsigned_integer()` returns None for negatives, which silently fell
                                        // back to 0 and derived a wrong AES-CTR key (every NALU failed to decrypt).
                                        // UxPlay reads it as uint64 and formats with %llu, so reinterpret the bits.
                                        let stream_id = sd
                                            .get("streamConnectionID")
                                            .and_then(|v| {
                                                v.as_unsigned_integer()
                                                    .or_else(|| v.as_signed_integer().map(|i| i as u64))
                                            })
                                            .unwrap_or_else(|| {
                                                eprintln!("[RTSP] Warning: SETUP stream 110 missing streamConnectionID");
                                                0
                                            });
                                        if let Some(aeskey) = session.aeskey {
                                            if let Some(old) = session.mirror_stop.take() {
                                                let _ = old.send(true);
                                            }
                                            if let Ok((dport, stop_tx)) = spawn_mirror_receiver(
                                                state.config.mirror_port,
                                                aeskey,
                                                stream_id,
                                                state.ipc.clone(),
                                            )
                                            .await
                                            {
                                                session.mirror_stop = Some(stop_tx);
                                                let mut rs = Dictionary::new();
                                                rs.insert("dataPort".into(), Value::Integer((dport as u64).into()));
                                                rs.insert("type".into(), Value::Integer(110u64.into()));
                                                res_streams.push(Value::Dictionary(rs));
                                            }
                                        }
                                    }
                                    96 => {
                                        let remote_cport = sd
                                            .get("controlPort")
                                            .and_then(|v| v.as_unsigned_integer())
                                            .unwrap_or(0) as u16;
                                        let ct = sd
                                            .get("ct")
                                            .and_then(|v| v.as_unsigned_integer())
                                            .unwrap_or(8) as u8;
                                        if let (Some(aeskey), Some(aesiv)) = (session.aeskey, session.aesiv) {
                                            if let Some(old) = session.audio_stop.take() {
                                                let _ = old.send(true);
                                            }
                                            if let Ok((dport, cport, stop_tx)) = spawn_audio_receiver(
                                                state.config.audio_data_port,
                                                state.config.audio_control_port,
                                                peer.ip(),
                                                remote_cport,
                                                aeskey,
                                                aesiv,
                                                ct,
                                                state.ipc.clone(),
                                            )
                                            .await
                                            {
                                                session.audio_stop = Some(stop_tx);
                                                let mut rs = Dictionary::new();
                                                rs.insert("dataPort".into(), Value::Integer((dport as u64).into()));
                                                rs.insert("controlPort".into(), Value::Integer((cport as u64).into()));
                                                rs.insert("type".into(), Value::Integer(96u64.into()));
                                                res_streams.push(Value::Dictionary(rs));
                                            }
                                        }
                                    }
                                    _ => {
                                        eprintln!("[RTSP] Unknown stream type {}", stype);
                                    }
                                }
                            }
                        }
                        res_dict.insert("streams".into(), Value::Array(res_streams));
                    }

                    resp_body = plist_to_binary(res_dict);
                    extra_headers.push(("Content-Type".into(), "application/x-apple-binary-plist".into()));
                }
            }
            ("RECORD", _) => {
                extra_headers.push(("Audio-Latency".into(), "0".into()));
                extra_headers.push(("Audio-Jack-Status".into(), "connected; type=analog".into()));
            }
            ("GET_PARAMETER", _) => {
                let text = String::from_utf8_lossy(&req.body);
                if text.contains("volume") {
                    resp_body = format!("volume: {:.6}\r\n", session.volume).into_bytes();
                    extra_headers.push(("Content-Type".into(), "text/parameters".into()));
                }
            }
            ("SET_PARAMETER", _) => {
                if let Some(ct) = req.headers.get("content-type") {
                    if ct == "text/parameters" {
                        let text = String::from_utf8_lossy(&req.body);
                        if let Some(rest) = text.strip_prefix("volume: ") {
                            if let Ok(v) = rest.trim().parse::<f64>() {
                                session.volume = v;
                                // Convert AirPlay dB volume (-30..0, -144=mute) to linear 0..1
                                let linear = if v <= -140.0 {
                                    0.0
                                } else {
                                    ((v + 30.0) / 30.0).clamp(0.0, 1.0)
                                };
                                state.ipc.send_event(&json!({
                                    "type": "volume",
                                    "db": v,
                                    "linear": linear,
                                }));
                            }
                        }
                    }
                }
            }
            ("FLUSH", _) => {
                state.ipc.send_event(&json!({ "type": "audio_flush" }));
            }
            ("POST", "/feedback") => {
                // Keepalive heartbeat
            }
            ("POST", "/audioMode") => {
                // Acknowledge audioMode
            }
            ("TEARDOWN", _) => {
                let mut td_96 = false;
                let mut td_110 = false;
                if let Ok(Value::Dictionary(d)) = plist::from_reader(Cursor::new(&req.body)) {
                    if let Some(Value::Array(streams)) = d.get("streams") {
                        for s in streams {
                            if let Value::Dictionary(sd) = s {
                                match sd.get("type").and_then(|v| v.as_unsigned_integer()) {
                                    Some(96) => td_96 = true,
                                    Some(110) => td_110 = true,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                if td_96 {
                    if let Some(tx) = session.audio_stop.take() {
                        let _ = tx.send(true);
                    }
                } else if td_110 {
                    if let Some(tx) = session.mirror_stop.take() {
                        let _ = tx.send(true);
                    }
                } else {
                    session.stop_all();
                    should_close = true;
                }
            }
            _ => {
                eprintln!("[RTSP] Unhandled: {} {} {}", req.method, req.url, req.protocol);
            }
        }

        let mut resp = format!("{} {} {}\r\n", req.protocol, status_code, status_text);
        resp.push_str(&format!("Server: AirTunes/{}\r\n", GLOBAL_VERSION));
        if let Some(cs) = cseq {
            resp.push_str(&format!("CSeq: {}\r\n", cs));
        }
        for (k, v) in extra_headers {
            resp.push_str(&format!("{}: {}\r\n", k, v));
        }
        if !resp_body.is_empty() {
            resp.push_str(&format!("Content-Length: {}\r\n", resp_body.len()));
        }
        if should_close {
            resp.push_str("Connection: close\r\n");
        }
        resp.push_str("\r\n");

        write_half.write_all(resp.as_bytes()).await?;
        if !resp_body.is_empty() {
            write_half.write_all(&resp_body).await?;
        }
        write_half.flush().await?;

        if should_close {
            break;
        }
    }

    session.stop_all();
    state.ipc.send_event(&json!({
        "type": "client_disconnected",
        "ip": peer.ip().to_string(),
    }));
    Ok(())
}
