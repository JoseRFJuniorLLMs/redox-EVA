# 🚀 EVA OS - Build Instructions

## Overview

**Redox-EVA OS** is a custom distribution of Redox OS with the EVA AI Voice Assistant pre-installed and configured.

## Features

- ✅ Full Redox OS desktop environment (COSMIC)
- ✅ EVA Daemon pre-installed
- ✅ Audio drivers configured
- ✅ Network stack enabled
- ✅ Auto-start EVA on boot
- ✅ Pre-configured for Gemini API

## Prerequisites

### System Requirements

- **OS:** Linux (Ubuntu 22.04+ recommended)
- **RAM:** 8GB minimum (16GB recommended)
- **Disk:** 20GB free space
- **CPU:** x86_64 with virtualization support

### Required Tools

```bash
# Ubuntu/Debian
sudo apt install -y build-essential curl git qemu-system-x86 \
    qemu-utils libfuse-dev pkg-config libc6-dev-i386 \
    nasm make mtools

# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default nightly
rustup component add rust-src
rustup target add x86_64-unknown-redox
```

## Building Redox-EVA OS

### Step 1: Clone the Repository

```bash
git clone https://github.com/JoseRFJuniorLLMs/redox-EVA.git
cd redox-EVA/redox-EVA
```

### Step 2: Initialize Submodules

```bash
# This downloads all Redox OS components (15-30 minutes)
git submodule update --init --recursive
```

### Step 3: Install Build Tools

```bash
make prefix
```

### Step 4: Configure for Redox-EVA

```bash
# Use the custom Redox-EVA configuration
make config recipe=redox-eva
```

### Step 5: Build the System

```bash
# First build (1-2 hours)
make all

# Subsequent builds (faster)
make rebuild
```

### Step 6: Configure EVA API Key

Before running, edit the configuration:

```bash
nano build/x86_64/redox-eva/filesystem/etc/eva/config.toml
```

Add your Gemini API key:
```toml
[eva]
api_key = "your_actual_api_key_here"
```

### Step 7: Run in QEMU

```bash
make qemu
```

## Using Redox-EVA OS

### First Boot

1. System boots to COSMIC desktop
2. EVA daemon starts automatically
3. Audio drivers load
4. Network connects

### Testing EVA

Open COSMIC Terminal and run:

```bash
# Check EVA status
ps aux | grep eva-daemon

# View EVA logs
cat /var/log/eva.log

# Manually start EVA (if needed)
eva-daemon
```

### Voice Commands

Once EVA is running:
- Speak naturally in Portuguese
- EVA will respond via audio
- Check terminal for text responses

## Configuration

### EVA Settings

Edit `/etc/eva/config.toml`:

```toml
[eva]
model = "gemini-2.0-flash-exp"
voice = "Kore"
language = "pt-PT"
api_key = "your_key"

[audio]
sample_rate = 48000
channels = 1
buffer_size = 4096

[network]
ws_url = "wss://eva-ia.org:8090/ws/pcm"
```

### Network Configuration

```bash
# Configure network (if needed)
sudo ip addr add 10.0.2.15/24 dev net0
sudo ip route add default via 10.0.2.2
```

## Troubleshooting

### EVA Not Starting

```bash
# Check logs
journalctl -u eva-daemon

# Manually start with debug
RUST_LOG=debug eva-daemon
```

### Audio Issues

```bash
# Check audio devices
ls -la /scheme/audio/

# Test audio
cat /dev/urandom | head -c 48000 > audio:play
```

### Network Issues

```bash
# Check network
ping 8.8.8.8

# Restart network
sudo systemctl restart network
```

## Building ISO Image

To create a bootable ISO:

```bash
make iso
```

The ISO will be in `build/x86_64/redox-eva/harddrive.iso`

## Installing to Real Hardware

⚠️ **Warning:** This will erase the target drive!

```bash
# Create bootable USB
sudo dd if=build/x86_64/redox-eva/harddrive.iso of=/dev/sdX bs=4M status=progress
sync
```

Replace `/dev/sdX` with your USB drive.

## Development

### Updating EVA Daemon

```bash
cd recipes/other/eva-daemon
# Edit recipe.toml to point to new version
make rebuild
```

### Adding Packages

Edit `config/redox-eva.toml`:

```toml
[packages]
your-package = {}
```

Then rebuild:

```bash
make rebuild
```

## Architecture

```
Redox-EVA OS
├── Kernel (microkernel)
├── Drivers
│   ├── Audio (audiod)
│   ├── Network (e1000)
│   └── Storage (nvmed)
├── Desktop (COSMIC)
│   ├── cosmic-term
│   ├── cosmic-edit
│   └── cosmic-files
└── EVA Daemon
    ├── WebSocket Client
    ├── Gemini Integration
    └── Audio Processing
```

## Performance

- **Boot Time:** ~5-10 seconds (QEMU)
- **Memory Usage:** ~200MB base + ~50MB EVA
- **Disk Usage:** ~800MB

## Contributing

1. Fork the repository
2. Create feature branch
3. Make changes
4. Test in QEMU
5. Submit pull request

## License

- **Redox OS:** MIT License
- **EVA Daemon:** MIT License
- **Redox-EVA OS:** MIT License

## Support

- **Issues:** https://github.com/JoseRFJuniorLLMs/redox-EVA/issues
- **Discussions:** https://github.com/JoseRFJuniorLLMs/redox-EVA/discussions
- **Redox Chat:** https://matrix.to/#/#redox-join:matrix.org

## Credits

- **Redox OS Team** - Base operating system
- **Google Gemini** - AI model
- **Jose R F Junior** - EVA integration

---

**Version:** 1.0.0  
**Based on:** Redox OS (latest)  
**Last Updated:** 2026-02-04
