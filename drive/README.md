# Intel NPU Driver for Redox OS

The **world's first userspace NPU driver for a microkernel operating system**, specifically designed for the **EVA OS** (Enhanced Voice Assistant) project and the **Redox OS** ecosystem.

## 🌟 Overview

This driver enables hardware-accelerated AI inference on Intel Meteor Lake (VPU 4.0) and future architectures (Arrow Lake) from within a microkernel's userspace. It avoids the safety risks of monolithic kernels by implementing a high-performance DMA and MMIO bridge entirely in unprivileged memory.

### Key Achievements
- **Zero-Kernel-Modifications**: Operates without a single line of kernel-level change.
- **Microkernel DMA**: Utilizes Redox's `phys_contiguous` scheme for uncacheable, physically contiguous memory.
- **NPU Scheme (`npu:`)**: Exposes the NPU as a standard Redox scheme, allowing any process to request inference jobs via file operations.
- **Meteor Lake Native**: Full support for PCI ID `0x7D1D`, including firmware loading and ring-buffer command queues.

## 🏗️ Architecture

```text
[ Process ] -> [ npu: scheme ] -> [ Intel NPU Daemon ] -> [ Hardware ]
                                         |
                                  [ DMA Ring Buffer ]
```

- **`pci.rs`**: Discovery and Bus Mastering activation.
- **`dma.rs`**: Safe, userspace-managed physical memory allocation.
- **`boot.rs`**: Handshake and firmware loading logic (Hexspeak status monitoring).
- **`inference.rs`**: Job submission and ring-buffer management.
- **`scheme.rs`**: Communication layer for system-wide access.

## 🚀 Status

- [x] PCI Device Detection
- [x] Userspace DMA via `memory:phys_contiguous`
- [x] Intel VPU Firmware Loading
- [x] Hexspeak Handshake (0xF00D)
- [x] Command Queue (Ring Buffer) Implementation
- [x] Redox `npu:` Scheme Implementation

## 🛠️ Build and Integration

To integrate this driver into your Redox build, add it to your `recipe.toml`:

```toml
[package]
name = "intel-npu"
permissions = [
    "CAP_SYS_PHYS",    # Physical memory access
    "CAP_IO_PORT",     # PCI configuration
    "CAP_MMAP_PHYS",   # Hardware mapping
]
dependencies = ["pcid"]
```

## 📜 License

MIT License. Developed as part of the **EVA OS** project by Jose R F Junior and the EVA OS Team.

---

*“Dedicated to the vision of a voice-controlled, AI-first operating system.”*
