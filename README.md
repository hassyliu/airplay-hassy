# AirPlay Hassy — 自定义画质 iOS 无线投屏接收端 (Rust + PySide6)

参考开源项目 [FDH2/UxPlay](https://github.com/FDH2/UxPlay)、[jdkuk/AirGlass](https://github.com/jdkuk/AirGlass) 与 [30jannik06/AirBridge](https://github.com/30jannik06/AirBridge) 协议设计，自研 **Rust 原生低延迟 AirPlay 协议引擎** + **PySide6 桌面控制与硬件解码客户端**，支持从协议源头自定义投屏分辨率、帧率与 H.265/HEVC 编码。

## 核心特性

1. **自定义画质（协议源头协商）**
   - 自由设定向 iOS 声明的分辨率（720p / 1080p / 2K 2560×1440 / 4K 3840×2160 / iPhone 竖屏原生 / 任意自定义宽高）
   - 自定义 `maxFPS` 与 `refreshRate`（30 / 60 / 120 FPS）
   - 一键开关 **H.265 / HEVC**（AirPlay Features Bit 42 `SupportsScreenMultiCodec`），同网络带宽下画质更清晰、支持 4K 输出
   - 实时 HUD 浮层监控：设备名、实际分辨率、编码格式 (AVC/HEVC)、实时 FPS、实时码率 (Mbps) 与解码器状态
2. **Rust 原生超低延迟协议引擎 (`engine/`)**
   - 纯 Rust 实现 mDNS 广播 (`_airplay._tcp` & `_raop._tcp`)，无需安装或启动 Apple Bonjour
   - 完整实现 RTSP/1.0 握手、Ed25519 + X25519 配对 (`pair-setup` / `pair-verify`)、**FairPlay SAP v3 (`playfair`)** 密钥解密、AES-128-CTR 视频流硬件加速解密、AVCC/HVCC 转 Annex-B 与 UDP NTP 时钟同步
   - 同步接收 UDP RTP/RTCP 音频流，支持 **AAC-ELD**、**ALAC**、AAC-LC 与 LPCM 解密及丢包重传请求
3. **硬件解码、音频输出、窗口增强与 MP4 录屏 (`app/`)**
   - 优先调用 NVIDIA **CUVID (NVDEC)** GPU 硬件解码 H.264 / H.265，自动回退低延迟多线程 CPU 解码
   - 实时画质增强：平滑缩放 / 像素锐利切换、实时锐化滤镜
   - `PyAV` + `sounddevice` 低延迟音频播放，支持音量滑块与跟随 iPhone 音量键
   - 窗口增强：窗口置顶、双击全屏、90° 旋转、水平镜像、**无边框纯净模式 (OBS Clean Feed)**
   - 本地一键录制 MP4（优先调用 `h264_nvenc` GPU 编码 + AAC-LC 音频，自动保存至 `recordings/`）

## 快速启动

### 1. 编译 Rust 引擎（如已编译过可跳过）
```powershell
cd engine
cargo build --release
```

### 2. 启动桌面应用
```powershell
python run.py
```

### 3. 连接 iPhone / iPad
1. 确保 iOS 设备与电脑处于**同一局域网 / Wi-Fi**（建议 5GHz Wi-Fi 或电脑网线连接以跑满 2K/4K 60fps 码率）。
2. 在 iOS 控制中心点击 **「屏幕镜像」**，选择 **`AirPlay-Hassy`**（可在左侧面板自定义名称）。
3. 修改分辨率 / 帧率 / HEVC 后，点击 **「应用画质并重启 AirPlay 服务」**，然后在 iOS 端重新连接一次镜像即可按新画质协商推流。

## 快捷键

| 快捷键 | 功能 |
|---|---|
| `Tab` | 收起 / 展开左侧画质控制面板 |
| `B` | 切换无边框纯净模式（OBS 窗口捕获推荐） |
| `F11` / 双击画面 | 进入 / 退出全屏 |
| `Esc` | 退出全屏或纯净模式 |
| `T` | 窗口置顶开关 |
| `R` | 画面顺时针旋转 90° |
| `M` | 水平镜像开关 |
| `I` | 显示 / 隐藏实时 FPS 与码率 HUD |
| `Ctrl + R` | 开始 / 停止录制 MP4 |
| 右键画面 | 快捷菜单（在无边框纯净模式下同样可用） |

## Windows 防火墙提示

如果 iPhone 搜不到设备或点击连接后转圈超时，请确保 Windows 防火墙允许 `engine/target/release/airplay-hassy-engine.exe` 在**专用网络**通过（所需端口：UDP `5353` mDNS、TCP `7000`/`7100`、UDP `6000`/`6001`/`7010`）。可以以管理员身份在 PowerShell 中一键放行：

```powershell
New-NetFirewallRule -DisplayName "AirPlay Hassy Engine" -Direction Inbound -Program "$PWD\engine\target\release\airplay-hassy-engine.exe" -Action Allow
```
