//! Hardware register definitions for Intel Meteor Lake NPU (VPU 4.0)
//!
//! Reverse-engineered from Linux kernel driver: drivers/accel/ivpu/
//! Sources: ivpu_hw_40xx.c, ivpu_hw_reg_io.h, ivpu_ipc.h
//!
//! ⚠️  These offsets target Meteor Lake (PCI 0x7D1D).
//!     Arrow Lake uses 0xAD1D with slightly different offsets.

// ============================================================
// PCI Identity
// ============================================================
pub const PCI_VENDOR_INTEL: u16 = 0x8086;

/// Meteor Lake NPU (VPU 4.0)
pub const PCI_DEVICE_MTL_NPU: u16 = 0x7D1D;

/// Arrow Lake NPU (VPU 4.0) — future support
pub const PCI_DEVICE_ARL_NPU: u16 = 0xAD1D;

/// Lunar Lake NPU (VPU 5.0) — future support
pub const PCI_DEVICE_LNL_NPU: u16 = 0x6467;

/// All supported device IDs
pub const SUPPORTED_DEVICES: &[(u16, &str)] = &[
    (PCI_DEVICE_MTL_NPU, "Meteor Lake NPU"),
    (PCI_DEVICE_ARL_NPU, "Arrow Lake NPU"),
    (PCI_DEVICE_LNL_NPU, "Lunar Lake NPU"),
];

// ============================================================
// BAR0 MMIO Register Map
// ============================================================

// --- Buttress (Global Control Subsystem) ---
pub const BUTTRESS_BASE: usize = 0x0000_0000;

/// Global interrupt mask (write 0x0 to unmask all)
pub const BUTTRESS_GLOBAL_INT_MASK: usize = BUTTRESS_BASE + 0x0020;

/// Global interrupt status
pub const BUTTRESS_GLOBAL_INT_STS: usize = BUTTRESS_BASE + 0x0024;

/// Tile fuse register (indicates active tiles)
pub const BUTTRESS_TILE_FUSE: usize = BUTTRESS_BASE + 0x0050;

/// VPU power status (bit 0 = powered on)
pub const BUTTRESS_VPU_STATUS: usize = BUTTRESS_BASE + 0x0114;

/// VPU D0i3 control (power gating)
pub const BUTTRESS_VPU_D0I3_CONTROL: usize = BUTTRESS_BASE + 0x0118;

/// Frequency control (PLL)
pub const BUTTRESS_VPU_IP_RESET: usize = BUTTRESS_BASE + 0x0160;

/// Workpoint frequency
pub const BUTTRESS_WP_REQ_PAYLOAD0: usize = BUTTRESS_BASE + 0x0200;
pub const BUTTRESS_WP_REQ_PAYLOAD1: usize = BUTTRESS_BASE + 0x0204;
pub const BUTTRESS_WP_REQ_CMD: usize = BUTTRESS_BASE + 0x0208;

// --- IPC (Inter-Processor Communication: CPU <-> NPU firmware) ---
pub const IPC_BASE: usize = 0x0007_3000;

/// Doorbell: Host -> Device (bit 31 must be set for NPU to recognize)
pub const IPC_HOST_2_DEVICE_DRBL: usize = IPC_BASE + 0x0000;

/// Doorbell trigger value — bit 31 signals a valid doorbell ring.
/// The Linux ivpu driver uses BIT(31) = 0x80000000; writing just `1` (bit 0)
/// will be ignored by real NPU hardware.
pub const IPC_DRBL_TRIGGER: u32 = 0x8000_0000;

/// Doorbell: Device -> Host (read for FW messages)
pub const IPC_DEVICE_2_HOST_DRBL: usize = IPC_BASE + 0x0004;

/// IPC data payload registers (8x 32-bit)
pub const IPC_HOST_2_DEVICE_DATA0: usize = IPC_BASE + 0x0010;
pub const IPC_HOST_2_DEVICE_DATA1: usize = IPC_BASE + 0x0014;
pub const IPC_HOST_2_DEVICE_DATA2: usize = IPC_BASE + 0x0018;
pub const IPC_HOST_2_DEVICE_DATA3: usize = IPC_BASE + 0x001C;

pub const IPC_DEVICE_2_HOST_DATA0: usize = IPC_BASE + 0x0020;
pub const IPC_DEVICE_2_HOST_DATA1: usize = IPC_BASE + 0x0024;

/// IPC interrupt mask
pub const IPC_INT_MASK: usize = IPC_BASE + 0x0030;

// --- Host Subsystem (Boot, Firmware Loading, Status) ---
pub const HOST_SS_BASE: usize = 0x0008_0000;

/// General control register
pub const HOST_SS_GEN_CTRL: usize = HOST_SS_BASE + 0x0000;

/// Clock enable
pub const HOST_SS_CLK_EN: usize = HOST_SS_BASE + 0x0004;

/// Component power reset SET
pub const HOST_SS_CPR_RST_SET: usize = HOST_SS_BASE + 0x0010;

/// Component power reset CLEAR (write 0x1 to release from reset)
pub const HOST_SS_CPR_RST_CLR: usize = HOST_SS_BASE + 0x0014;

/// Firmware load address (low 32 bits of DMA physical address)
pub const HOST_SS_LOADING_ADDR_LO: usize = HOST_SS_BASE + 0x0040;

/// Firmware load address (high 32 bits of DMA physical address)
pub const HOST_SS_LOADING_ADDR_HI: usize = HOST_SS_BASE + 0x0044;

/// Firmware entry point
pub const HOST_SS_ENTRY_POINT: usize = HOST_SS_BASE + 0x0048;

/// Firmware status — THE key register. Contains Hexspeak codes.
///
/// Known values:
///   0x00000000 — Not initialized / powered off
///   0xF00D0000 — Firmware READY (success!)
///   0xDEAD0000 — Firmware DEAD / fatal error
///   0xCAFE0000 — Waiting / Hesitant (needs nudge)
///   0xBEEF0000 — Boot in progress
///   0x0BAD0000 — Bad firmware image
///   0xFACE0000 — Firmware loaded, initializing
pub const HOST_SS_FW_STATUS: usize = HOST_SS_BASE + 0x0060;

/// Firmware version (read after boot)
pub const HOST_SS_FW_VERSION: usize = HOST_SS_BASE + 0x0064;

/// Boot progress counter
pub const HOST_SS_BOOT_COUNT: usize = HOST_SS_BASE + 0x0068;

// ============================================================
// Firmware Status Codes (Hexspeak)
// ============================================================
pub const FW_STATUS_MASK: u32 = 0xFFFF_0000;
pub const FW_STATUS_READY: u32 = 0xF00D_0000;
pub const FW_STATUS_DEAD: u32 = 0xDEAD_0000;
pub const FW_STATUS_CAFE: u32 = 0xCAFE_0000;
pub const FW_STATUS_BEEF: u32 = 0xBEEF_0000;
pub const FW_STATUS_OBAD: u32 = 0x0BAD_0000;
pub const FW_STATUS_FACE: u32 = 0xFACE_0000;

// ============================================================
// PCI Config Space
// ============================================================
/// PCI Command register offset
pub const PCI_CMD_REG: u16 = 0x04;

/// PCI Command: Bus Master Enable (bit 2)
pub const PCI_CMD_BUS_MASTER: u16 = 0x0004;

/// PCI Command: Memory Space Enable (bit 1)
pub const PCI_CMD_MEMORY_SPACE: u16 = 0x0002;

/// PCI Command: I/O Space Enable (bit 0)
pub const PCI_CMD_IO_SPACE: u16 = 0x0001;

// ============================================================
// DMA / Memory Constants
// ============================================================
/// Firmware maximum size (16 MB)
pub const FW_MAX_SIZE: usize = 16 * 1024 * 1024;

/// DMA alignment required by NPU (4KB page aligned)
pub const DMA_ALIGNMENT: usize = 4096;

/// Command queue ring buffer size (256 entries)
pub const CMD_QUEUE_SIZE: usize = 256;

/// Single command descriptor size (64 bytes)
pub const CMD_DESC_SIZE: usize = 64;

// ============================================================
// Timing Constants
// ============================================================
/// Maximum wait for power-up (milliseconds)
pub const POWER_UP_TIMEOUT_MS: u64 = 2000;

/// Maximum wait for firmware boot (milliseconds)
pub const FW_BOOT_TIMEOUT_MS: u64 = 5000;

/// Polling interval during waits (milliseconds)
pub const POLL_INTERVAL_MS: u64 = 10;

/// Delay before "nudge" retry (milliseconds)
pub const NUDGE_DELAY_MS: u64 = 300;

/// Maximum nudge retries
pub const NUDGE_MAX_RETRIES: u32 = 5;

// ============================================================
// Utility
// ============================================================

/// Decode firmware status to human-readable string
pub fn decode_fw_status(raw: u32) -> &'static str {
    match raw & FW_STATUS_MASK {
        0x0000_0000 => "NOT_INITIALIZED (powered off or no firmware)",
        FW_STATUS_READY => "READY (0xF00D) — Firmware operational! 🎉",
        FW_STATUS_DEAD => "DEAD (0xDEAD) — Fatal firmware error ☠️",
        FW_STATUS_CAFE => "WAITING (0xCAFE) — Needs doorbell nudge",
        FW_STATUS_BEEF => "BOOTING (0xBEEF) — Boot in progress",
        FW_STATUS_OBAD => "BAD_IMAGE (0x0BAD) — Corrupt firmware",
        FW_STATUS_FACE => "LOADING (0xFACE) — Firmware initializing",
        _ => "UNKNOWN",
    }
}

/// Check if a PCI device ID is a supported NPU
pub fn is_supported_device(device_id: u16) -> Option<&'static str> {
    SUPPORTED_DEVICES
        .iter()
        .find(|(id, _)| *id == device_id)
        .map(|(_, name)| *name)
}
