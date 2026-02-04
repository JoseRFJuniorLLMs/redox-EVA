# 🌟 EVA OS (v0.13.0)

**The World's First Privacy-First, AI-Native Operating System.**

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Redox OS](https://img.shields.io/badge/Redox%20OS-Based-red.svg)](https://www.redox-os.org/)
[![Status](https://img.shields.io/badge/Status-Stable-green.svg)]()
[![AI](https://img.shields.io/badge/AI-Hybrid%20(Local+Cloud)-purple.svg)]()

> "Your voice is your interface. Your privacy is paramount. Your past is secure."

---

## 🚀 What is EVA OS?

**EVA OS** is a next-generation operating system built on **Redox OS** (Rust microkernel), designed from the ground up for **voice interaction** and **AI integration**. 

Unlike traditional OSs that bolt AI on top, EVA OS puts AI at the kernel level of user interaction, acting as a "living" shell that understands natural language, manages your system, and enables a "Time Machine" memory of your digital life—all whilst respecting your privacy through strict **Local-First** processing.

---

## ✨ Key Features (v0.13.0)

### 🧠 Hybrid AI Architecture
- **Online Intelligence**: Connects to **Gemini Pro/Flash** for complex reasoning and world knowledge.
- **Offline Privacy**: Uses local **NPU (Neural Processing Unit)** for sensitive tasks (OCR, Screen Analysis, Wake Word).
- **Zero-Latency Voice**: Local VAD (Voice Activity Detection) ensures instant response.

### 🕰️ Time Machine AI (New in v0.13!)
A privacy-focused implementation of "Photographic Memory" for your PC:
- **Continuous Recording**: Captures encrypted snapshots of your workflow.
- **Semantic Search**: Ask *"What was I working on yesterday morning?"* instead of searching files.
- **100% Local & Encrypted**: Uses **AES-256-GCM**. No data ever leaves your device.
- **Smart Filtering**: Automatically blocks banking apps, incognito mode, and sensitive windows.

### 🖥️ Visual & Voice Experience
- **Terminal UI**: Beautiful, lag-free TUI with real-time audio visualization.
- **Emotion Engine**: EVA detects your mood and adapts its voice/responses.
- **System Control**: Open apps, kill processes, and manage files naturally. *("EVA, kill the frozen browser", "Create a backup of my project")*.
- **Audio Playback**: Full TTS (Text-to-Speech) responses.

---

## 📊 Project Status

**Current Version:** 0.13.0  
**Progress:** Phase 13/16 Complete  

| Phase | Feature | Status | Technology |
|:---:|:---|:---:|:---|
| 1-4 | **Core Network & SSL** | ✅ Done | `tokio`, `rustls` |
| 5-7 | **Conversation Loop** | ✅ Done | `google-generative-ai`, `cpal` |
| 8 | **Visual Feedback** | ✅ Done | `ratatui`, `termion` |
| 9 | **Long-term Memory** | ✅ Done | `serde_json` |
| 10 | **System Control** | ✅ Done | `sysinfo`, `std::process` |
| 13 | **Time Machine AI** | ✅ Done | **`ort` (ONNX), `faiss`, `AES-256`** |
| 14 | **Offline Commands** | 🚧 Next | `vosk`, `regex` |
| 15 | **Local Voice (TTS)** | 🗓️ Planned | `piper-rs` |

---

## 🛠️ Usage

### Prerequisites
- **Rust (Nightly)**
- **Windows / Linux / Redox OS**
- Optional: **NPU** (Intel/AMD/Qualcomm) for accelerated Time Machine.

### Running EVA
```bash
# Clone the repository
git clone https://github.com/JoseRFJuniorLLMs/EVA-OS.git
cd EVA-OS/eva-daemon

# Run with Time Machine enabled
cargo run --release --features timemachine
```

### Voice Commands
- **System**: *"Open Calculator"*, *"What is my RAM usage?"*, *"Close Firefox"*
- **Time Machine**: *"What code was I writing at 10 AM?"*, *"Show me the email from John"*
- **General**: *"Tell me a joke"*, *"Explain Quantum Physics"*
- **Privacy**: *"Pause recording"*, *"Delete history from today"*

---

## 🏗️ Architecture

```mermaid
graph TD
    User[User Voice] --> Audio[Audio Capture (cpal)]
    Audio --> VAD[VAD & Wake Word]
    
    subgraph "EVA Daemon (Local)"
        VAD --> |"Hey EVA"| Core[Core Loop]
        Core --> NPU[NPU Delegate (ort)]
        NPU --> OCR[OCR Engine]
        NPU --> Embed[Embedding Engine]
        Core --> TUI[Terminal UI]
        Core --> Sys[System Executor]
    end
    
    subgraph "Secure Storage"
        OCR --> DB[(Encrypted SQLite)]
        Embed --> Vector[(FAISS Index)]
        DB --> |AES-256| Disk[Local Disk]
    end
    
    subgraph "Cloud (Optional)"
        Core --> |JSON| Gemini[Gemini API]
    end
```

---

## 🤝 Contributing
Contributions are welcome! We are especially looking for help with:
1. **Redox OS Drivers**: Improving audio driver stability on bare metal.
2. **Local LLMs**: Integrating `llama.rs` for full offline chat.
3. **UI/UX**: Improving the TUI animations.

## 📄 License
MIT License - Copyright (c) 2026 Jose R F Junior
