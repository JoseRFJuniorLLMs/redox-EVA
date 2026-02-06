//! NPU Status Monitor
//!
//! Provides continuous monitoring of the NPU's health after boot.
//! Watches the FW_STATUS register for state changes, detects crashes,
//! and provides an interface for querying NPU readiness.

use crate::hw_mtl::*;
use crate::mmio::MmioRegion;
use log::{debug, error, info, warn};
use std::time::{Duration, Instant};

/// Current NPU state, derived from hardware registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpuState {
    /// Not powered / not initialized
    PoweredOff,
    /// Firmware is booting
    Booting,
    /// Firmware is ready for inference
    Ready,
    /// Firmware encountered a fatal error
    Dead,
    /// NPU is executing an inference job
    Busy,
    /// Unknown state
    Unknown(u32),
}

impl std::fmt::Display for NpuState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NpuState::PoweredOff => write!(f, "💤 Powered Off"),
            NpuState::Booting => write!(f, "🔄 Booting"),
            NpuState::Ready => write!(f, "✅ Ready"),
            NpuState::Dead => write!(f, "☠️  Dead"),
            NpuState::Busy => write!(f, "⚡ Busy"),
            NpuState::Unknown(v) => write!(f, "❓ Unknown ({:#010x})", v),
        }
    }
}

/// Status monitor that reads hardware state.
pub struct StatusMonitor<'a> {
    mmio: &'a MmioRegion,
    last_state: NpuState,
    last_check: Instant,
    state_changes: Vec<(Instant, NpuState)>,
    total_inferences: u64,
    uptime_start: Instant,
}

impl<'a> StatusMonitor<'a> {
    pub fn new(mmio: &'a MmioRegion) -> Self {
        let now = Instant::now();
        Self {
            mmio,
            last_state: NpuState::PoweredOff,
            last_check: now,
            state_changes: vec![(now, NpuState::PoweredOff)],
            total_inferences: 0,
            uptime_start: now,
        }
    }

    /// Read the current NPU state from hardware.
    pub fn poll(&mut self) -> NpuState {
        let raw = self.mmio.read32(HOST_SS_FW_STATUS);
        let state = self.decode_state(raw);

        if state != self.last_state {
            let now = Instant::now();
            info!(
                "NPU state change: {} → {} (raw={:#010x})",
                self.last_state, state, raw
            );
            self.state_changes.push((now, state));
            self.last_state = state;
        }

        self.last_check = Instant::now();
        state
    }

    /// Get the last known state without hitting hardware.
    pub fn last_state(&self) -> NpuState {
        self.last_state
    }

    /// Check if the NPU is ready for inference.
    pub fn is_ready(&mut self) -> bool {
        self.poll() == NpuState::Ready
    }

    /// Get raw firmware status register value.
    pub fn raw_status(&self) -> u32 {
        self.mmio.read32(HOST_SS_FW_STATUS)
    }

    /// Get firmware version (valid only after successful boot).
    pub fn fw_version(&self) -> u32 {
        self.mmio.read32(HOST_SS_FW_VERSION)
    }

    /// Get Buttress power status.
    pub fn buttress_status(&self) -> u32 {
        self.mmio.read32(BUTTRESS_VPU_STATUS)
    }

    /// Get interrupt status.
    pub fn interrupt_status(&self) -> u32 {
        self.mmio.read32(BUTTRESS_GLOBAL_INT_STS)
    }

    /// Get uptime since monitor creation.
    pub fn uptime(&self) -> Duration {
        self.uptime_start.elapsed()
    }

    /// Get number of state changes observed.
    pub fn state_change_count(&self) -> usize {
        self.state_changes.len()
    }

    /// Record a completed inference.
    pub fn record_inference(&mut self) {
        self.total_inferences += 1;
    }

    /// Get total inference count.
    pub fn total_inferences(&self) -> u64 {
        self.total_inferences
    }

    /// Print a full diagnostic report.
    pub fn print_diagnostics(&self) {
        let raw = self.mmio.read32(HOST_SS_FW_STATUS);
        let fw_ver = self.mmio.read32(HOST_SS_FW_VERSION);
        let buttress = self.mmio.read32(BUTTRESS_VPU_STATUS);
        let int_sts = self.mmio.read32(BUTTRESS_GLOBAL_INT_STS);
        let boot_count = self.mmio.read32(HOST_SS_BOOT_COUNT);
        let gen_ctrl = self.mmio.read32(HOST_SS_GEN_CTRL);

        println!("╔══════════════════════════════════════════╗");
        println!("║       Intel NPU Diagnostic Report        ║");
        println!("╠══════════════════════════════════════════╣");
        println!("║ State       : {:26} ║", format!("{}", self.last_state));
        println!("║ FW Status   : {:#010x} {:16} ║", raw, decode_fw_status(raw));
        println!("║ FW Version  : {:#010x}                    ║", fw_ver);
        println!("║ Buttress    : {:#010x}                    ║", buttress);
        println!("║ Interrupts  : {:#010x}                    ║", int_sts);
        println!("║ Boot Count  : {:10}                    ║", boot_count);
        println!("║ Gen Control : {:#010x}                    ║", gen_ctrl);
        println!("║ Uptime      : {:10.1}s                   ║", self.uptime().as_secs_f64());
        println!("║ Inferences  : {:10}                    ║", self.total_inferences);
        println!("║ State Chgs  : {:10}                    ║", self.state_changes.len());
        println!("╚══════════════════════════════════════════╝");
    }

    // ================================================================
    // Internal
    // ================================================================

    fn decode_state(&self, raw: u32) -> NpuState {
        match raw & FW_STATUS_MASK {
            0x0000_0000 => NpuState::PoweredOff,
            FW_STATUS_READY => NpuState::Ready,
            FW_STATUS_DEAD => NpuState::Dead,
            FW_STATUS_BEEF | FW_STATUS_FACE | FW_STATUS_CAFE => NpuState::Booting,
            _ => NpuState::Unknown(raw),
        }
    }
}
