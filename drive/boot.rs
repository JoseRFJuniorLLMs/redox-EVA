//! NPU Boot Sequence — Power Up, Firmware Load, Handshake
//!
//! Implements the full startup sequence for the Meteor Lake NPU:
//!
//! 1. Power Up: Release from reset, verify power via Buttress
//! 2. Firmware Load: Copy firmware to DMA buffer, write address to NPU
//! 3. Boot Trigger: Ring the doorbell, wait for 0xF00D
//! 4. Nudge Strategy: If NPU hesitates (0xCAFE), retry the doorbell
//!
//! Based on reverse engineering of Linux ivpu driver boot path:
//!   ivpu_hw_40xx.c → ivpu_boot_fw(), ivpu_hw_40xx_run_boot_fw()

use crate::dma::{self, DmaBuffer};
use crate::hw_mtl::*;
use crate::mmio::MmioRegion;
use log::{debug, error, info, warn};
use std::thread;
use std::time::Duration;

/// Result of the boot sequence.
#[derive(Debug)]
pub enum BootResult {
    /// Firmware is ready (0xF00D). NPU is operational.
    Ready { fw_version: u32 },
    /// Firmware loaded but status is ambiguous.
    Ambiguous { status: u32 },
}

/// Full boot orchestrator.
pub struct BootSequence<'a> {
    mmio: &'a MmioRegion,
}

impl<'a> BootSequence<'a> {
    pub fn new(mmio: &'a MmioRegion) -> Self {
        Self { mmio }
    }

    /// Execute the complete boot sequence.
    ///
    /// Returns both the boot result and the firmware DMA buffer.
    /// The caller MUST keep the returned `DmaBuffer` alive for the entire
    /// lifetime of the driver — the NPU continues to reference the firmware
    /// at its physical address after boot.
    pub fn execute(&self, fw_path: &str) -> Result<(BootResult, DmaBuffer), BootError> {
        info!("╔══════════════════════════════════════════╗");
        info!("║   Intel NPU Boot Sequence Starting...    ║");
        info!("╚══════════════════════════════════════════╝");

        // Step 1: Power up the NPU
        self.power_up()?;

        // Step 2: Load firmware into DMA buffer
        let fw_buffer = self.load_firmware(fw_path)?;

        // Step 3: Tell NPU where the firmware lives
        self.set_firmware_address(&fw_buffer)?;

        // Step 4: Trigger boot and wait for handshake
        let result = self.trigger_and_wait()?;

        match &result {
            BootResult::Ready { fw_version } => {
                info!("╔══════════════════════════════════════════╗");
                info!("║   ✅ NPU BOOT SUCCESSFUL!                ║");
                info!("║   Firmware Version: {:#010x}          ║", fw_version);
                info!("╚══════════════════════════════════════════╝");
            }
            BootResult::Ambiguous { status } => {
                warn!("⚠️  NPU boot completed with ambiguous status: {:#010x}", status);
                warn!("    Decoded: {}", decode_fw_status(*status));
            }
        }

        // Return fw_buffer to caller — it MUST stay alive while NPU is running.
        // Dropping it would free the physical memory the NPU is still reading.
        Ok((result, fw_buffer))
    }

    // ================================================================
    // Step 1: Power Up
    // ================================================================

    fn power_up(&self) -> Result<(), BootError> {
        info!("🔌 [1/4] Power-up sequence...");

        // Read initial status
        let initial = self.mmio.read32(HOST_SS_FW_STATUS);
        debug!("  Initial FW_STATUS: {:#010x} ({})", initial, decode_fw_status(initial));

        // Exit D0i3 power gating state (must happen before any other power ops)
        info!("  Exiting D0i3 power state...");
        self.mmio.write32(BUTTRESS_VPU_D0I3_CONTROL, 0x0);
        thread::sleep(Duration::from_millis(10));

        // Enable clocks FIRST (Linux ivpu driver: clocks before reset release)
        info!("  Enabling clocks...");
        self.mmio.write32(HOST_SS_CLK_EN, 0x1);
        thread::sleep(Duration::from_millis(10));

        // THEN release NPU from reset
        info!("  Clearing reset...");
        self.mmio.write32(HOST_SS_CPR_RST_CLR, 0x1);

        // Delay for hardware to stabilize after reset release
        thread::sleep(Duration::from_millis(50));

        // Poll Buttress for power confirmation
        info!("  Polling Buttress for power status...");
        let buttress_result = self.mmio.poll_until(
            BUTTRESS_VPU_STATUS,
            |val| val & 0x1 != 0, // Bit 0 = powered
            POLL_INTERVAL_MS,
            POWER_UP_TIMEOUT_MS,
        );

        match buttress_result {
            Ok(val) => {
                info!("  ✅ Buttress confirms power ON (status={:#010x})", val);
            }
            Err(last_val) => {
                // Don't fail hard — some revisions report differently
                warn!(
                    "  ⚠️  Buttress power bit not set after {}ms (last={:#010x})",
                    POWER_UP_TIMEOUT_MS, last_val
                );
                warn!("  Continuing anyway (Buttress check is advisory)...");
            }
        }

        // Read tile fuse to know what we're working with
        let tile_fuse = self.mmio.read32(BUTTRESS_TILE_FUSE);
        debug!("  Tile fuse: {:#010x}", tile_fuse);

        // NOTE: Interrupts are unmasked later in trigger_and_wait(), just before
        // ringing the doorbell. Unmasking too early can cause spurious IRQs
        // before the firmware is loaded.

        info!("  ✅ Power-up complete.");
        Ok(())
    }

    // ================================================================
    // Step 2: Load Firmware
    // ================================================================

    fn load_firmware(&self, fw_path: &str) -> Result<DmaBuffer, BootError> {
        info!("📦 [2/4] Loading firmware: {}", fw_path);

        let fw_buffer = dma::load_firmware(fw_path).map_err(|e| BootError::FirmwareLoad(e))?;

        info!(
            "  ✅ Firmware in DMA: phys={:#010x}, size={} bytes",
            fw_buffer.phys_addr, fw_buffer.size
        );

        Ok(fw_buffer)
    }

    // ================================================================
    // Step 3: Set Firmware Address
    // ================================================================

    fn set_firmware_address(&self, fw_buffer: &DmaBuffer) -> Result<(), BootError> {
        info!("📍 [3/4] Writing firmware address to NPU registers...");

        // Write the 64-bit physical address where firmware lives
        self.mmio
            .write32(HOST_SS_LOADING_ADDR_LO, fw_buffer.phys_lo());
        self.mmio
            .write32(HOST_SS_LOADING_ADDR_HI, fw_buffer.phys_hi());

        // Verify the write (read back)
        let readback_lo = self.mmio.read32(HOST_SS_LOADING_ADDR_LO);
        let readback_hi = self.mmio.read32(HOST_SS_LOADING_ADDR_HI);

        debug!(
            "  Readback: LO={:#010x} (expected {:#010x})",
            readback_lo,
            fw_buffer.phys_lo()
        );
        debug!(
            "  Readback: HI={:#010x} (expected {:#010x})",
            readback_hi,
            fw_buffer.phys_hi()
        );

        if readback_lo != fw_buffer.phys_lo() || readback_hi != fw_buffer.phys_hi() {
            error!("  ❌ Address readback mismatch! MMIO write may have failed.");
            return Err(BootError::AddressReadbackMismatch);
        }

        info!("  ✅ Firmware address set: {:#018x}", fw_buffer.phys_addr);
        Ok(())
    }

    // ================================================================
    // Step 4: Trigger Boot + Nudge Strategy
    // ================================================================

    fn trigger_and_wait(&self) -> Result<BootResult, BootError> {
        info!("🔔 [4/4] Triggering NPU boot (doorbell)...");

        // Unmask interrupts NOW — firmware is loaded and address is set,
        // so the NPU can signal us back via IPC after we ring the doorbell.
        info!("  Unmasking global + IPC interrupts...");
        self.mmio.write32(BUTTRESS_GLOBAL_INT_MASK, 0x0);
        self.mmio.write32(IPC_INT_MASK, 0x0);

        // Ring the doorbell — bit 31 must be set (IPC_DRBL_TRIGGER)
        self.mmio.write32(IPC_HOST_2_DEVICE_DRBL, IPC_DRBL_TRIGGER);

        // Initial delay — let the NPU start processing
        thread::sleep(Duration::from_millis(NUDGE_DELAY_MS));

        // Poll for firmware status with nudge retries
        let mut nudge_count = 0u32;
        let boot_start = std::time::Instant::now();
        let boot_timeout = Duration::from_millis(FW_BOOT_TIMEOUT_MS);

        loop {
            // Hard global timeout — prevents infinite loop on unknown status
            if boot_start.elapsed() >= boot_timeout {
                let last = self.mmio.read32(HOST_SS_FW_STATUS);
                error!(
                    "  ❌ Boot timed out after {}ms (last status: {:#010x} = {})",
                    FW_BOOT_TIMEOUT_MS, last, decode_fw_status(last)
                );
                self.dump_diagnostics();
                return Err(BootError::Timeout { last_status: last });
            }

            let raw_status = self.mmio.read32(HOST_SS_FW_STATUS);
            let status_code = raw_status & FW_STATUS_MASK;

            debug!(
                "  Status poll: {:#010x} → {} (elapsed={:.1}s)",
                raw_status,
                decode_fw_status(raw_status),
                boot_start.elapsed().as_secs_f64()
            );

            match status_code {
                // ===== SUCCESS =====
                FW_STATUS_READY => {
                    info!("  🎉 Firmware reports READY (0xF00D)!");
                    let fw_version = self.mmio.read32(HOST_SS_FW_VERSION);
                    return Ok(BootResult::Ready { fw_version });
                }

                // ===== FATAL ERRORS =====
                FW_STATUS_DEAD => {
                    error!("  ☠️  Firmware reports DEAD (0xDEAD)!");
                    self.dump_diagnostics();
                    return Err(BootError::FirmwareDead);
                }

                FW_STATUS_OBAD => {
                    error!("  ❌ Firmware reports BAD IMAGE (0x0BAD)!");
                    return Err(BootError::FirmwareBadImage);
                }

                // ===== NEEDS NUDGE =====
                FW_STATUS_CAFE => {
                    nudge_count += 1;
                    if nudge_count > NUDGE_MAX_RETRIES {
                        error!("  ❌ NPU stuck in CAFE state after {} nudges", nudge_count);
                        self.dump_diagnostics();
                        return Err(BootError::NudgeExhausted { attempts: nudge_count });
                    }

                    warn!(
                        "  ⚠️  NPU hesitant (0xCAFE). Nudge #{}/{}...",
                        nudge_count, NUDGE_MAX_RETRIES
                    );

                    // Re-ring the doorbell (bit 31 = trigger)
                    self.mmio.write32(IPC_HOST_2_DEVICE_DRBL, IPC_DRBL_TRIGGER);
                    thread::sleep(Duration::from_millis(NUDGE_DELAY_MS * (nudge_count as u64 + 1)));
                }

                // ===== IN PROGRESS =====
                FW_STATUS_BEEF | FW_STATUS_FACE => {
                    debug!("  ⏳ Boot in progress...");
                    thread::sleep(Duration::from_millis(POLL_INTERVAL_MS * 10));
                }

                // ===== NOT INITIALIZED / UNKNOWN =====
                _ => {
                    if raw_status == 0x0000_0000 {
                        // Still waiting to wake up
                        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS * 5));
                    } else {
                        debug!(
                            "  Unknown status {:#010x}, continuing to poll...",
                            raw_status
                        );
                        thread::sleep(Duration::from_millis(POLL_INTERVAL_MS * 10));
                    }
                }
            }

            // Boot count sanity check
            let boot_count = self.mmio.read32(HOST_SS_BOOT_COUNT);
            if boot_count > 100 {
                warn!("  Boot count high ({}), NPU may be in a loop", boot_count);
            }
        }
    }

    // ================================================================
    // Diagnostics
    // ================================================================

    fn dump_diagnostics(&self) {
        error!("=== NPU Diagnostic Dump ===");
        error!(
            "  FW_STATUS    : {:#010x} ({})",
            self.mmio.read32(HOST_SS_FW_STATUS),
            decode_fw_status(self.mmio.read32(HOST_SS_FW_STATUS))
        );
        error!(
            "  FW_VERSION   : {:#010x}",
            self.mmio.read32(HOST_SS_FW_VERSION)
        );
        error!(
            "  BOOT_COUNT   : {}",
            self.mmio.read32(HOST_SS_BOOT_COUNT)
        );
        error!(
            "  BUTTRESS     : {:#010x}",
            self.mmio.read32(BUTTRESS_VPU_STATUS)
        );
        error!(
            "  GEN_CTRL     : {:#010x}",
            self.mmio.read32(HOST_SS_GEN_CTRL)
        );
        error!(
            "  GLOBAL_INT   : {:#010x}",
            self.mmio.read32(BUTTRESS_GLOBAL_INT_STS)
        );
        error!("=== End Diagnostic Dump ===");
    }
}

// ============================================================
// Error Types
// ============================================================

#[derive(Debug)]
pub enum BootError {
    PowerUpTimeout,
    FirmwareLoad(dma::DmaError),
    AddressReadbackMismatch,
    FirmwareDead,
    FirmwareBadImage,
    NudgeExhausted { attempts: u32 },
    Timeout { last_status: u32 },
}

impl std::fmt::Display for BootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PowerUpTimeout => write!(f, "NPU power-up timed out"),
            Self::FirmwareLoad(e) => write!(f, "Firmware load failed: {}", e),
            Self::AddressReadbackMismatch => {
                write!(f, "Firmware address readback mismatch (MMIO write failure)")
            }
            Self::FirmwareDead => write!(f, "NPU firmware reported DEAD (0xDEAD)"),
            Self::FirmwareBadImage => write!(f, "NPU rejected firmware image (0x0BAD)"),
            Self::NudgeExhausted { attempts } => {
                write!(f, "NPU stuck in CAFE state after {} nudge attempts", attempts)
            }
            Self::Timeout { last_status } => {
                write!(
                    f,
                    "Boot timed out. Last status: {:#010x} ({})",
                    last_status,
                    decode_fw_status(*last_status)
                )
            }
        }
    }
}

impl std::error::Error for BootError {}
