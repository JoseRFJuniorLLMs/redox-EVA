# 🧠 EVA Daemon - AI Voice Assistant for Redox OS

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Redox OS](https://img.shields.io/badge/Redox%20OS-Compatible-red.svg)](https://www.redox-os.org/)

Native voice AI integration for Redox OS using Google Gemini 2.5 Flash with real-time audio streaming.

## ✨ Features

- 🎤 **Real-time Voice Capture** - Direct audio input via Redox `audio:` scheme
- 🔊 **Audio Playback** - Ring buffer implementation for smooth playback
- 🌐 **WebSocket Streaming** - Bidirectional communication with Gemini API
- 🔐 **Secure TLS 1.3** - Pure Rust implementation with `rustls`
- 🎯 **Voice Activity Detection** - Smart audio processing
- 🤖 **Gemini Integration** - Native WebSocket protocol support

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust nightly
rustup default nightly
rustup component add rust-src
rustup target add x86_64-unknown-redox
```

### Build

```bash
# For Linux/Windows (testing)
cargo build --release

# For Redox OS
cargo build --target x86_64-unknown-redox --release
```

### Run

```bash
# Set your Gemini API key
export GOOGLE_API_KEY="your_api_key_here"

# Run the daemon
./target/release/eva-daemon
```

## 📦 Project Structure

```
eva-daemon/
├── src/
│   ├── main.rs          # Main entry point
│   ├── tls.rs           # TLS manager with rustls
│   ├── websocket.rs     # WebSocket client
│   ├── gemini.rs        # Gemini API client
│   └── audio.rs         # Audio capture/playback (Phase 4)
├── Cargo.toml           # Dependencies
├── README.md            # This file
└── QUICKSTART.md        # Quick start guide
```

## 🎯 Implementation Phases

### ✅ Phase 1: Network Connectivity
- [x] DNS resolution
- [x] TCP connections
- [x] Basic error handling

### ✅ Phase 2: TLS/SSL
- [x] rustls integration
- [x] Certificate validation
- [x] HTTPS requests
- [x] Secure connections

### ✅ Phase 3: WebSocket + Gemini
- [x] WebSocket client (WSS)
- [x] Gemini API integration
- [x] Message streaming
- [x] Protocol implementation

### 🚧 Phase 4: Audio Integration
- [ ] Microphone capture
- [ ] Ring buffer
- [ ] Voice Activity Detection
- [ ] Redox `audio:` scheme

### 🚧 Phase 5: Full Integration
- [ ] Real-time conversation loop
- [ ] Session management
- [ ] Error recovery
- [ ] Production ready

## 🔧 Configuration

### Environment Variables

```bash
# Required
GOOGLE_API_KEY=your_gemini_api_key

# Optional
MODEL_ID=gemini-2.0-flash-exp
WS_URL=wss://eva-ia.org:8090/ws/pcm
```

### Redox OS Integration

Add to your Redox build configuration:

```toml
# config/x86_64/desktop.toml
[packages]
eva-daemon = "recipe"
```

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo run

# Test specific phase
cargo run --bin eva-daemon
```

## 📚 Documentation

- [Phase 1 Guide](../fase1.md) - Network connectivity
- [Phase 2 Guide](../fase2.md) - TLS/SSL implementation
- [Phase 3 Guide](../fase3.md) - WebSocket + Gemini
- [Quick Start](./QUICKSTART.md) - Get started quickly

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│         EVA Daemon (Rust)               │
├─────────────────────────────────────────┤
│  ┌──────────┐  ┌──────────┐            │
│  │  Audio   │  │ Gemini   │            │
│  │ Manager  │  │  Client  │            │
│  └────┬─────┘  └────┬─────┘            │
│       │             │                   │
│  ┌────┴─────────────┴─────┐            │
│  │   WebSocket Client     │            │
│  └────────┬───────────────┘            │
│           │                             │
│  ┌────────┴───────────────┐            │
│  │    TLS Manager         │            │
│  └────────────────────────┘            │
└─────────────────────────────────────────┘
           │
           ▼
    ┌─────────────┐
    │  Gemini API │
    └─────────────┘
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Redox OS](https://www.redox-os.org/) - The operating system
- [Google Gemini](https://ai.google.dev/gemini-api) - AI model
- [rustls](https://github.com/rustls/rustls) - TLS implementation
- [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) - WebSocket client

## 📞 Contact

- **Author:** Jose R F Junior
- **GitHub:** [@JoseRFJuniorLLMs](https://github.com/JoseRFJuniorLLMs)
- **Project:** [eva-daemon](https://github.com/JoseRFJuniorLLMs/eva-daemon)

## 🔗 Related Projects

- [EVA Mind Backend](https://github.com/JoseRFJuniorLLMs/EVA-Mind-FZPN) - Go backend for EVA
- [Redox OS](https://gitlab.redox-os.org/redox-os/redox) - The operating system

---

**Status:** ✅ Phase 3 Complete | 🚧 Phase 4 In Progress  
**Version:** 0.3.0  
**Last Updated:** 2026-02-04
