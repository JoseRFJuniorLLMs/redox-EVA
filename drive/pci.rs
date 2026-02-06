//! PCI Device Discovery and Configuration
//!
//! Handles finding the Intel NPU on the PCI bus, enabling Bus Mastering
//! (required for DMA), and mapping BAR0 (MMIO registers).
//!
//! On Redox OS, PCI devices are accessed via the `pci:` scheme.
//! On other platforms, this provides mock implementations for testing.

use crate::hw_mtl::*;
use crate::mmio::MmioRegion;
use log::{debug, error, info, warn};
use std::io;

/// Discovered NPU device information.
pub struct NpuDevice {
    /// PCI bus:device.function address
    pub bdf: String,
    /// Device ID
    pub device_id: u16,
    /// Device name (human readable)
    pub device_name: &'static str,
    /// BAR0 physical base address
    pub bar0_phys: u64,
    /// BAR0 size
    pub bar0_size: usize,
    /// MMIO region (mapped BAR0)
    pub mmio: MmioRegion,
    /// Mock BAR pointer for proper deallocation (non-Redox only)
    #[cfg(not(target_os = "redox"))]
    mock_bar_ptr: Option<*mut u8>,
}

impl Drop for NpuDevice {
    fn drop(&mut self) {
        #[cfg(not(target_os = "redox"))]
        {
            if let Some(ptr) = self.mock_bar_ptr.take() {
                let layout = std::alloc::Layout::from_size_align(self.bar0_size, 4096)
                    .expect("NpuDevice drop: invalid layout");
                unsafe { std::alloc::dealloc(ptr, layout); }
                log::info!("Mock BAR0 memory freed ({} bytes)", self.bar0_size);
            }
        }
    }
}

/// Scan the PCI bus for a supported Intel NPU.
pub fn discover_npu() -> Result<NpuDevice, PciError> {
    info!("🔍 Scanning PCI bus for Intel NPU...");

    #[cfg(target_os = "redox")]
    {
        discover_redox()
    }

    #[cfg(not(target_os = "redox"))]
    {
        discover_mock()
    }
}

// ================================================================
// Redox OS Implementation
// ================================================================

#[cfg(target_os = "redox")]
fn discover_redox() -> Result<NpuDevice, PciError> {
    use std::fs;
    use std::os::unix::io::AsRawFd;

    // List all PCI devices via the scheme
    let pci_entries = fs::read_dir("pci:").map_err(|e| PciError::SchemeFailed(e))?;

    for entry in pci_entries {
        let entry = entry.map_err(|e| PciError::SchemeFailed(e))?;
        let path = entry.path();
        let bdf = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Read config space to check vendor/device
        let config_path = format!("pci:{}/config", bdf);
        let config = match fs::read(&config_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if config.len() < 4 {
            continue;
        }

        let vendor_id = u16::from_le_bytes([config[0], config[1]]);
        let device_id = u16::from_le_bytes([config[2], config[3]]);

        debug!("  PCI {}: vendor={:#06x} device={:#06x}", bdf, vendor_id, device_id);

        // Check if this is Intel
        if vendor_id != PCI_VENDOR_INTEL {
            continue;
        }

        // Check if it's a supported NPU
        if let Some(name) = is_supported_device(device_id) {
            info!("  ✅ Found: {} at PCI {}", name, bdf);

            // Enable Bus Mastering (CRITICAL for DMA)
            enable_bus_mastering_redox(&bdf, &config)?;

            // Map BAR0
            let (mmio, bar0_phys, bar0_size) = map_bar0_redox(&bdf)?;

            return Ok(NpuDevice {
                bdf,
                device_id,
                device_name: name,
                bar0_phys,
                bar0_size,
                mmio,
            });
        }
    }

    error!("  ❌ No supported Intel NPU found on PCI bus");
    Err(PciError::DeviceNotFound)
}

#[cfg(target_os = "redox")]
fn enable_bus_mastering_redox(bdf: &str, config: &[u8]) -> Result<(), PciError> {
    use std::fs::OpenOptions;
    use std::io::{Seek, Write};

    info!("  Enabling Bus Mastering on {}...", bdf);

    // Bounds check: need at least 6 bytes (vendor[2] + device[2] + command[2])
    if config.len() < 6 {
        error!("  PCI config space too short ({} bytes, need >= 6)", config.len());
        return Err(PciError::ConfigWrite(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("PCI config too short: {} bytes", config.len()),
        )));
    }

    // Read current PCI command register (offset 0x04, 2 bytes)
    let cmd = u16::from_le_bytes([config[4], config[5]]);
    debug!("  Current PCI CMD: {:#06x}", cmd);

    // Set Bus Master + Memory Space Enable
    let new_cmd = cmd | PCI_CMD_BUS_MASTER | PCI_CMD_MEMORY_SPACE;

    if new_cmd != cmd {
        let config_path = format!("pci:{}/config", bdf);
        let mut file = OpenOptions::new()
            .write(true)
            .open(&config_path)
            .map_err(|e| PciError::ConfigWrite(e))?;

        // Seek to offset 4 (PCI command register) before writing
        file.seek(io::SeekFrom::Start(4))
            .map_err(|e| PciError::ConfigWrite(e))?;

        // Write the 2-byte command register value
        let cmd_bytes = new_cmd.to_le_bytes();
        file.write_all(&cmd_bytes)
            .map_err(|e| PciError::ConfigWrite(e))?;

        info!(
            "  ✅ Bus Mastering enabled (CMD: {:#06x} → {:#06x})",
            cmd, new_cmd
        );
    } else {
        info!("  Bus Mastering already enabled");
    }

    Ok(())
}

#[cfg(target_os = "redox")]
fn map_bar0_redox(bdf: &str) -> Result<(MmioRegion, u64, usize), PciError> {
    use std::fs::OpenOptions;
    use std::os::unix::io::AsRawFd;

    let bar_path = format!("pci:{}/bar0", bdf);
    info!("  Mapping BAR0: {}", bar_path);

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&bar_path)
        .map_err(|e| PciError::BarOpen(e))?;

    // Get BAR size from file metadata
    let metadata = file.metadata().map_err(|e| PciError::BarOpen(e))?;
    let bar_size = metadata.len() as usize;

    if bar_size == 0 {
        return Err(PciError::BarZeroSize);
    }

    info!("  BAR0 size: {:#x} ({} KB)", bar_size, bar_size / 1024);

    // Map BAR0 into our address space
    let virt_addr = unsafe {
        syscall::fmap(
            file.as_raw_fd() as usize,
            &syscall::Map {
                offset: 0,
                size: bar_size,
                flags: syscall::MapFlags::MAP_SHARED
                    | syscall::MapFlags::PROT_READ
                    | syscall::MapFlags::PROT_WRITE,
            },
        )
        .map_err(|e| PciError::BarMmap(e))?
    };

    info!("  ✅ BAR0 mapped at virt={:#x}", virt_addr);

    // Resolve physical address of BAR
    let bar0_phys = unsafe {
        syscall::virttophys(virt_addr).unwrap_or(0)
    } as u64;

    let mmio = unsafe { MmioRegion::new(virt_addr as *mut u8, bar_size) };

    // Keep the fd alive without running File's destructor.
    // into_raw_fd() transfers ownership of the fd to us, preventing both
    // the fd leak (mem::forget leaks the File struct) and premature close.
    use std::os::unix::io::IntoRawFd;
    let _raw_fd = file.into_raw_fd();

    Ok((mmio, bar0_phys, bar_size))
}

// ================================================================
// Mock Implementation (for development on Linux/Mac/Windows)
// ================================================================

#[cfg(not(target_os = "redox"))]
fn discover_mock() -> Result<NpuDevice, PciError> {
    warn!("⚠️  Mock PCI discovery (not on Redox OS)");
    warn!("    Simulating Meteor Lake NPU at PCI 0000:00:0b.0");

    // Allocate a fake MMIO region
    let bar_size = 1024 * 1024; // 1MB mock BAR
    let layout = std::alloc::Layout::from_size_align(bar_size, 4096).unwrap();
    let ptr = unsafe { std::alloc::alloc_zeroed(layout) };

    if ptr.is_null() {
        return Err(PciError::MockAllocFailed);
    }

    // Pre-populate some registers for testing
    unsafe {
        // Buttress VPU status: powered on
        let buttress_status = ptr.add(BUTTRESS_VPU_STATUS) as *mut u32;
        std::ptr::write_volatile(buttress_status, 0x0000_0001);

        // FW status: not initialized
        let fw_status = ptr.add(HOST_SS_FW_STATUS) as *mut u32;
        std::ptr::write_volatile(fw_status, 0x0000_0000);
    }

    let mmio = unsafe { MmioRegion::new(ptr, bar_size) };

    // Store layout alongside the device so it can be freed properly.
    // The mock MMIO memory is freed via NpuDevice's Drop impl.
    Ok(NpuDevice {
        bdf: "0000:00:0b.0".to_string(),
        device_id: PCI_DEVICE_MTL_NPU,
        device_name: "Meteor Lake NPU (MOCK)",
        bar0_phys: ptr as u64,
        bar0_size: bar_size,
        mmio,
        #[cfg(not(target_os = "redox"))]
        mock_bar_ptr: Some(ptr),
    })
}

// ================================================================
// Error Types
// ================================================================

#[derive(Debug)]
pub enum PciError {
    SchemeFailed(io::Error),
    DeviceNotFound,
    ConfigWrite(io::Error),
    BarOpen(io::Error),
    BarZeroSize,
    #[cfg(target_os = "redox")]
    BarMmap(syscall::Error),
    MockAllocFailed,
}

impl std::fmt::Display for PciError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemeFailed(e) => write!(f, "PCI scheme access failed: {}", e),
            Self::DeviceNotFound => write!(f, "No supported Intel NPU found on PCI bus"),
            Self::ConfigWrite(e) => write!(f, "PCI config write failed: {}", e),
            Self::BarOpen(e) => write!(f, "Failed to open BAR0: {}", e),
            Self::BarZeroSize => write!(f, "BAR0 has zero size"),
            #[cfg(target_os = "redox")]
            Self::BarMmap(e) => write!(f, "BAR0 mmap failed: {:?}", e),
            Self::MockAllocFailed => write!(f, "Mock MMIO allocation failed"),
        }
    }
}

impl std::error::Error for PciError {}
