//! DMA Buffer Management — Userspace Strategy
//!
//! This is the heart of the "Elan's Pivot": instead of modifying the Redox
//! kernel with new syscalls, we use the existing `memory:phys_contiguous`
//! scheme to allocate physically contiguous, uncacheable memory from userspace.
//!
//! The NPU requires physical addresses for DMA transfers (firmware loading,
//! command queues, inference data). This module provides that bridge.
//!
//! # Architecture
//!
//! ```text
//!   CPU (Rust code)                    NPU Hardware
//!   ┌──────────────┐                  ┌──────────────┐
//!   │ virt_addr    │──── writes ────▶│              │
//!   │ (mmap'd)     │                  │  DMA Engine  │
//!   └──────────────┘                  │              │
//!         │                           │  reads from  │
//!         │ virttophys()              │  phys_addr   │
//!         ▼                           └──────┬───────┘
//!   ┌──────────────┐                         │
//!   │ phys_addr    │◀────────────────────────┘
//!   │ (real RAM)   │
//!   └──────────────┘
//! ```

use crate::hw_mtl::DMA_ALIGNMENT;
use log::{debug, error, info};
use std::io;

/// A physically contiguous DMA buffer accessible by both CPU and NPU.
///
/// Fields are `pub(crate)` to prevent external crates from modifying
/// safety-critical invariants (phys_addr, virt_addr) while allowing
/// internal driver modules full access.
pub struct DmaBuffer {
    /// Virtual address for CPU access (Rust can read/write here)
    pub(crate) virt_addr: usize,
    /// Physical address for NPU hardware (written to MMIO registers)
    pub(crate) phys_addr: u64,
    /// Buffer size in bytes
    pub(crate) size: usize,
    /// Keeps the backing file alive — dropping this unmaps the memory
    #[cfg(target_os = "redox")]
    _file: std::fs::File,
    /// On non-Redox (dev/test), we just use a heap allocation
    #[cfg(not(target_os = "redox"))]
    _backing: Vec<u8>,
}

impl DmaBuffer {
    /// Allocate a new DMA buffer of at least `size` bytes.
    ///
    /// On Redox OS:
    ///   - Opens `memory:phys_contiguous?size=N&uncacheable`
    ///   - Maps it via `fmap` (mmap equivalent)
    ///   - Resolves physical address via `virttophys`
    ///
    /// On other OS (development):
    ///   - Allocates page-aligned heap memory
    ///   - Uses virtual address as fake "physical" address
    pub fn new(size: usize) -> Result<Self, DmaError> {
        if size == 0 {
            return Err(DmaError::ZeroSize);
        }

        // Round up to page alignment
        let aligned_size = (size + DMA_ALIGNMENT - 1) & !(DMA_ALIGNMENT - 1);

        info!(
            "Allocating DMA buffer: requested={} bytes, aligned={} bytes",
            size, aligned_size
        );

        #[cfg(target_os = "redox")]
        {
            Self::alloc_redox(aligned_size)
        }

        #[cfg(not(target_os = "redox"))]
        {
            Self::alloc_mock(aligned_size)
        }
    }

    /// Redox-specific allocation using the phys_contiguous scheme.
    #[cfg(target_os = "redox")]
    fn alloc_redox(size: usize) -> Result<Self, DmaError> {
        use std::fs::OpenOptions;
        use std::os::unix::io::AsRawFd;

        // 1. Open the physical contiguous memory scheme
        //    The `uncacheable` flag is CRITICAL: it ensures the NPU sees
        //    our writes immediately (no CPU cache coherence issues).
        let scheme_path = format!("memory:phys_contiguous?size={}&uncacheable", size);
        debug!("Opening scheme: {}", scheme_path);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&scheme_path)
            .map_err(|e| DmaError::SchemeOpen(e))?;

        // 2. Map into our virtual address space
        let virt_addr = unsafe {
            syscall::fmap(
                file.as_raw_fd() as usize,
                &syscall::Map {
                    offset: 0,
                    size,
                    flags: syscall::MapFlags::MAP_SHARED
                        | syscall::MapFlags::PROT_READ
                        | syscall::MapFlags::PROT_WRITE,
                },
            )
            .map_err(|e| DmaError::Mmap(e))?
        };

        info!("DMA buffer mapped at virt_addr={:#x}", virt_addr);

        // 3. Resolve the physical address
        //    This requires CAP_SYS_PHYS in recipe.toml
        let phys_addr = unsafe {
            syscall::virttophys(virt_addr).map_err(|e| DmaError::VirtToPhys(e))?
        } as u64;

        info!(
            "DMA buffer physical address: {:#x} (virt={:#x}, size={:#x})",
            phys_addr, virt_addr, size
        );

        // 4. Zero the buffer (clean slate for firmware/commands)
        unsafe {
            std::ptr::write_bytes(virt_addr as *mut u8, 0, size);
        }

        Ok(Self {
            virt_addr,
            phys_addr,
            size,
            _file: file,
        })
    }

    /// Mock allocation for development on Linux/macOS/Windows.
    #[cfg(not(target_os = "redox"))]
    fn alloc_mock(size: usize) -> Result<Self, DmaError> {
        info!("⚠️  Mock DMA allocation (not on Redox — using heap)");

        // Allocate with alignment guarantee
        let mut backing = vec![0u8; size + DMA_ALIGNMENT];
        let raw_ptr = backing.as_mut_ptr() as usize;

        // Align to DMA_ALIGNMENT boundary
        let aligned_ptr = (raw_ptr + DMA_ALIGNMENT - 1) & !(DMA_ALIGNMENT - 1);

        info!(
            "Mock DMA: virt={:#x}, fake_phys={:#x}, size={:#x}",
            aligned_ptr, aligned_ptr, size
        );

        Ok(Self {
            virt_addr: aligned_ptr,
            phys_addr: aligned_ptr as u64, // In mock mode, virt == "phys"
            size,
            _backing: backing,
        })
    }

    /// Get the low 32 bits of the physical address (for LOADING_ADDR_LO register).
    pub fn phys_lo(&self) -> u32 {
        self.phys_addr as u32
    }

    /// Get the high 32 bits of the physical address (for LOADING_ADDR_HI register).
    pub fn phys_hi(&self) -> u32 {
        (self.phys_addr >> 32) as u32
    }

    /// Write raw bytes into the DMA buffer at the given offset.
    ///
    /// Uses volatile writes to ensure the compiler does not elide, reorder,
    /// or merge stores. This is critical for DMA buffers that hardware reads.
    pub fn write_bytes(&self, offset: usize, data: &[u8]) -> Result<(), DmaError> {
        let end = offset.checked_add(data.len()).ok_or(DmaError::OutOfBounds {
            offset,
            len: data.len(),
            capacity: self.size,
        })?;
        if end > self.size {
            return Err(DmaError::OutOfBounds {
                offset,
                len: data.len(),
                capacity: self.size,
            });
        }

        unsafe {
            let dst = (self.virt_addr + offset) as *mut u8;
            for i in 0..data.len() {
                std::ptr::write_volatile(dst.add(i), data[i]);
            }
        }

        // Memory fence to ensure all volatile writes are visible to hardware
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);

        Ok(())
    }

    /// Read raw bytes from the DMA buffer.
    ///
    /// Uses volatile reads to ensure the compiler does not cache or eliminate
    /// reads from memory that hardware may have written via DMA.
    pub fn read_bytes(&self, offset: usize, len: usize) -> Result<Vec<u8>, DmaError> {
        let end = offset.checked_add(len).ok_or(DmaError::OutOfBounds {
            offset,
            len,
            capacity: self.size,
        })?;
        if end > self.size {
            return Err(DmaError::OutOfBounds {
                offset,
                len,
                capacity: self.size,
            });
        }

        let mut result = vec![0u8; len];
        unsafe {
            let src = (self.virt_addr + offset) as *const u8;
            for i in 0..len {
                result[i] = std::ptr::read_volatile(src.add(i));
            }
        }

        Ok(result)
    }

    /// Write a 32-bit value at offset within the buffer.
    ///
    /// Returns an error on out-of-bounds instead of panicking.
    pub fn write_u32(&self, offset: usize, value: u32) -> Result<(), DmaError> {
        let bytes = value.to_le_bytes();
        self.write_bytes(offset, &bytes)
    }

    /// Read a 32-bit value from offset within the buffer.
    ///
    /// Returns an error on out-of-bounds instead of panicking.
    pub fn read_u32(&self, offset: usize) -> Result<u32, DmaError> {
        let bytes = self.read_bytes(offset, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Get a mutable slice view of the entire buffer.
    ///
    /// # Safety
    /// Caller must ensure no hardware DMA is in progress when mutating.
    /// The returned slice uses volatile-unaware access — for DMA-visible
    /// writes, prefer `write_bytes()` which uses volatile semantics.
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        std::slice::from_raw_parts_mut(self.virt_addr as *mut u8, self.size)
    }

    /// Read the entire buffer using volatile reads.
    ///
    /// Returns a copy of the buffer contents. This is safe to call even when
    /// hardware may have written to the buffer via DMA, as each byte is read
    /// with volatile semantics.
    pub fn read_all(&self) -> Vec<u8> {
        self.read_bytes(0, self.size)
            .expect("read_all with offset=0 and len=size cannot be OOB")
    }

    /// Zero the entire buffer using volatile writes.
    ///
    /// Ensures the compiler cannot optimize away the zeroing, which is
    /// critical when the buffer will be read by DMA hardware.
    pub fn zero(&self) {
        unsafe {
            let dst = self.virt_addr as *mut u8;
            for i in 0..self.size {
                std::ptr::write_volatile(dst.add(i), 0);
            }
        }
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    }
}

impl Drop for DmaBuffer {
    fn drop(&mut self) {
        info!(
            "Releasing DMA buffer: virt={:#x}, phys={:#x}, size={:#x}",
            self.virt_addr, self.phys_addr, self.size
        );

        #[cfg(target_os = "redox")]
        {
            // On Redox, unmapping happens when _file is dropped,
            // but we explicitly unmap for safety.
            let _ = unsafe { syscall::funmap(self.virt_addr, self.size) };
        }
    }
}

// ============================================================
// Firmware Loader
// ============================================================

/// Expected firmware magic bytes: "VPU!" (0x56, 0x50, 0x55, 0x21)
const FW_MAGIC: [u8; 4] = [0x56, 0x50, 0x55, 0x21];

/// Load firmware binary from disk into a DMA buffer.
///
/// Validates the firmware magic header before loading to prevent
/// loading corrupted or non-firmware files into DMA.
pub fn load_firmware(path: &str) -> Result<DmaBuffer, DmaError> {
    info!("Loading firmware from: {}", path);

    let fw_data = std::fs::read(path).map_err(|e| DmaError::FirmwareRead(e))?;

    if fw_data.is_empty() {
        return Err(DmaError::FirmwareEmpty);
    }

    if fw_data.len() > crate::hw_mtl::FW_MAX_SIZE {
        return Err(DmaError::FirmwareTooLarge {
            actual: fw_data.len(),
            max: crate::hw_mtl::FW_MAX_SIZE,
        });
    }

    // Validate firmware magic header
    if fw_data.len() < 4 || fw_data[0..4] != FW_MAGIC {
        let got = if fw_data.len() >= 4 {
            format!("{:02x} {:02x} {:02x} {:02x}", fw_data[0], fw_data[1], fw_data[2], fw_data[3])
        } else {
            format!("(only {} bytes)", fw_data.len())
        };
        error!("Firmware magic mismatch: expected 'VPU!' (56 50 55 21), got {}", got);
        return Err(DmaError::FirmwareBadMagic);
    }

    info!(
        "Firmware loaded: {} bytes ({:.2} MB), magic OK",
        fw_data.len(),
        fw_data.len() as f64 / (1024.0 * 1024.0)
    );

    // Allocate DMA buffer sized to firmware
    let buf = DmaBuffer::new(fw_data.len())?;

    // Copy firmware into DMA buffer
    buf.write_bytes(0, &fw_data)?;

    info!(
        "Firmware written to DMA at phys={:#010x}",
        buf.phys_addr
    );

    Ok(buf)
}

// ============================================================
// Error Types
// ============================================================

#[derive(Debug)]
pub enum DmaError {
    SchemeOpen(io::Error),
    #[cfg(target_os = "redox")]
    Mmap(syscall::Error),
    #[cfg(target_os = "redox")]
    VirtToPhys(syscall::Error),
    OutOfBounds {
        offset: usize,
        len: usize,
        capacity: usize,
    },
    ZeroSize,
    FirmwareRead(io::Error),
    FirmwareEmpty,
    FirmwareBadMagic,
    FirmwareTooLarge {
        actual: usize,
        max: usize,
    },
}

impl std::fmt::Display for DmaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemeOpen(e) => write!(f, "Failed to open phys_contiguous scheme: {}", e),
            #[cfg(target_os = "redox")]
            Self::Mmap(e) => write!(f, "mmap (fmap) failed: {:?}", e),
            #[cfg(target_os = "redox")]
            Self::VirtToPhys(e) => write!(f, "virttophys failed (missing CAP_SYS_PHYS?): {:?}", e),
            Self::OutOfBounds { offset, len, capacity } => {
                write!(f, "DMA access out of bounds: offset={:#x} + len={:#x} > capacity={:#x}", offset, len, capacity)
            }
            Self::ZeroSize => write!(f, "Cannot allocate zero-size DMA buffer"),
            Self::FirmwareRead(e) => write!(f, "Failed to read firmware file: {}", e),
            Self::FirmwareEmpty => write!(f, "Firmware file is empty"),
            Self::FirmwareBadMagic => write!(f, "Firmware magic header invalid (expected 'VPU!')"),
            Self::FirmwareTooLarge { actual, max } => {
                write!(f, "Firmware too large: {} bytes (max {})", actual, max)
            }
        }
    }
}

impl std::error::Error for DmaError {}
