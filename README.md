# 🌟 EVA OS

**The World's First Voice-Controlled Operating System**

[![Rust](https://img.shields.io/badge/rust-nightly-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Redox OS](https://img.shields.io/badge/Redox%20OS-Based-red.svg)](https://www.redox-os.org/)
[![AI Powered](https://img.shields.io/badge/AI-Gemini%202.5-blue.svg)](https://ai.google.dev/gemini-api)

> "Your voice is your interface. No keyboard, no mouse - just natural conversation."

---
## EVA OS - Voice-Controlled Operating System

**EVA OS** (Enhanced Voice Assistant Operating System) is a revolutionary voice-first operating system built on Redox OS, featuring an AI-powered assistant that responds to natural language commands.

## 🎯 Vision

Create a fully voice-controlled operating system where users can interact with their computer naturally through conversation, eliminating the need for traditional keyboard and mouse input for most tasks.

## ✨ Features

### Core Capabilities (v0.7.0)
- 🎤 **Always-On Voice Recognition** - Responds to "Hey EVA"
- 🤖 **AI-Powered Conversations** - Natural language understanding via Gemini
- 🎵 **Audio Responses** - Text-to-speech with emotional context
- 💻 **System Control** - Execute commands through voice
- 🔒 **Secure Sandbox** - Isolated file operations at `~/.eva/sandbox/`
- 👤 **User Profiles** - Personalized preferences and settings

### Advanced Features
- ⚡ **Custom Commands** - Create your own voice shortcuts with triggers
- 🎬 **Voice Macros** - Record and replay command sequences
- 😊 **Emotion Detection** - Understands 8 different user emotions
- 📁 **File Operations** - Create, delete, copy, move, list, read files
- ⚙️ **System Info** - Memory, CPU, disk information by voice
- 🔄 **Session Management** - Maintains conversation context

## 🏗️ Architecture

EVA OS consists of:
- **eva-daemon**: Core voice assistant service (14 Rust modules, ~4,500 lines)
- **Redox OS Integration**: Custom build configuration
- **Gemini API**: AI conversation engine
- **Configuration**: User profiles, custom commands, macros in `~/.eva/`

## 📊 Current Status

**Version:** 0.7.0  
**Progress:** 70% Complete (7/10 phases)  
**Last Updated:** 2026-02-04

### ✅ Completed Phases
1. ✅ **Network Connectivity** - DNS, TCP connections
2. ✅ **TLS/SSL Security** - rustls, HTTPS support
3. ✅ **WebSocket + Gemini API** - Real-time AI communication
4. ✅ **Audio Integration** - Mic capture, wake word, VAD
5. ✅ **Full AI Conversation Loop** - Audio playback, session management
6. ✅ **System Command Integration** - Command parsing, sandboxed execution
7. ✅ **Advanced Voice Features** - Profiles, custom commands, macros, emotions

### 🚧 Upcoming Phases
8. 🚧 **Visual Feedback** - Status indicators, UI, animations
9. 🚧 **Accessibility** - Multi-language, voice customization
10. 🚧 **Advanced AI** - Context learning, predictive suggestions
# Configure
make config recipe=eva-os

# Build (1-2 hours first time)
make all

# Run in QEMU
make qemu
```

---

## 💬 Voice Commands

EVA OS understands natural language. Here are some examples:

### File Operations
```
"EVA, create a folder called projects"
"Copy all PDF files to the backup folder"
"Delete the file test.log"
"Show me what's in the documents folder"
"Rename report.pdf to final_report.pdf"
```

### System Control
```
"EVA, show me the memory usage"
"What processes are running?"
"Open the web browser"
"Close all windows"
"Restart the computer"
```

### Text Input
```
"EVA, type 'Hello World'"
"Select all text"
"Copy this"
"Paste here"
"Save the file"
```

### Network Operations
```
"What's my IP address?"
"Test connection to Google"
"Connect to WiFi 'Home'"
"Download the file from [URL]"
```

---

## 📊 Current Status

### ✅ Completed Phases

| Phase | Feature | Status |
|-------|---------|--------|
| **Phase 1** | Network Connectivity | ✅ Complete |
| **Phase 2** | TLS/SSL Security | ✅ Complete |
| **Phase 3** | WebSocket + Gemini API | ✅ Complete |
| **Phase 4** | Audio Integration | ✅ Complete |

### 🚧 In Development

| Phase | Feature | ETA |
|-------|---------|-----|
| **Phase 5** | Full AI Conversation Loop | 1 week |
| **Phase 6** | System Command Execution | 2 weeks |
| **Phase 7** | Advanced Voice Features | 3 weeks |
| **Phase 8** | Visual Feedback System | 4 weeks |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────┐
│              User Voice Input               │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│         EVA Voice Processing                │
│  - Wake word: "Hey EVA"                     │
│  - Voice Activity Detection                 │
│  - Audio preprocessing                      │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│         Gemini AI Processing                │
│  - Speech-to-Text                           │
│  - Natural Language Understanding           │
│  - Command Execution                        │
│  - Text-to-Speech Response                  │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│         Redox OS (Microkernel)              │
│  - File system                              │
│  - Process management                       │
│  - Memory management                        │
│  - Device drivers                           │
└─────────────────────────────────────────────┘
```

---

## 📚 Documentation

- [**Phase 1 Guide**](./fase1.md) - Network connectivity
- [**Phase 2 Guide**](./fase2.md) - TLS/SSL implementation
- [**Phase 3 Guide**](./fase3.md) - WebSocket + Gemini
- [**Phase 4 Guide**](./fase4.md) - Audio integration
- [**Build Instructions**](./BUILD_EVA_OS.md) - How to build EVA OS
- [**Complete Vision**](./claude.md) - Full project roadmap

---

## 🎓 Why EVA OS?

### For Users
- **Accessibility** - Perfect for users with mobility limitations
- **Productivity** - Faster than typing for many tasks
- **Natural** - Speak naturally, no command memorization
- **Hands-Free** - Work while doing other things

### For Developers
- **Open Source** - MIT License, fully transparent
- **Modern Stack** - Rust, Redox OS, Gemini AI
- **Extensible** - Easy to add new voice commands
- **Educational** - Learn OS development and AI integration

### For the Future
- **Innovation** - Pushing boundaries of human-computer interaction
- **Privacy** - Local processing option available
- **Security** - Microkernel architecture
- **Community** - Growing ecosystem of contributors

---

## 🤝 Contributing

We welcome contributions! Here's how you can help:

1. **Test & Report** - Try EVA OS and report issues
2. **Translate** - Add support for more languages
3. **Develop** - Implement new voice commands
4. **Document** - Improve documentation
5. **Spread the Word** - Share EVA OS with others

### Development Setup

```bash
# Fork the repository
git clone https://github.com/YOUR_USERNAME/EVA-OS.git
cd EVA-OS

# Create feature branch
git checkout -b feature/amazing-feature

# Make changes and test
cd eva-daemon
cargo build --release
cargo test

# Commit and push
git commit -m "Add amazing feature"
git push origin feature/amazing-feature

# Open Pull Request
```

---

## 📄 License

EVA OS is open source software licensed under the [MIT License](./LICENSE).

```
Copyright (c) 2026 Jose R F Junior

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software.
```

---

## 🙏 Acknowledgments

- **Redox OS Team** - For the amazing microkernel OS
- **Google Gemini** - For the powerful AI model
- **Rust Community** - For the incredible language and tools
- **Contributors** - Everyone who helps make EVA OS better

---

## 📞 Contact & Support

- **GitHub:** [@JoseRFJuniorLLMs](https://github.com/JoseRFJuniorLLMs)
- **Project:** [EVA OS](https://github.com/JoseRFJuniorLLMs/EVA-OS)
- **Issues:** [Report a Bug](https://github.com/JoseRFJuniorLLMs/EVA-OS/issues)
- **Discussions:** [Join the Conversation](https://github.com/JoseRFJuniorLLMs/EVA-OS/discussions)

---

## 🌟 Star History

If you like EVA OS, please give us a star! ⭐

---

**Made with ❤️ by the EVA OS Community**

**Version:** 0.4.0 (Phase 4 Complete)  
**Last Updated:** 2026-02-04  
**Status:** 🚧 Active Development

---

## 🎬 See It In Action

> Coming soon: Video demonstrations of EVA OS in action!

---

**EVA OS - The Future of Computing is Voice** 🎤
