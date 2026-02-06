//! Intel NPU Driver Daemon for Redox OS (EVA OS)
//!
//! This is the world's first NPU driver for a microkernel OS.
//!
//! Architecture:
//!   - Runs entirely in userspace (no kernel modifications)
//!   - Uses Redox `memory:phys_contiguous` for DMA
//!   - Communicates with NPU hardware via MMIO (BAR0)
//!   - Loads Intel VPU firmware and monitors health
//!
//! Usage:
//!   intel-npu [--firmware PATH] [--test] [--diagnostics]
//!
//! On Redox OS, this runs as a daemon via redox-daemon.
//! On other OS, it runs in mock mode for development/testing.

mod boot;
mod dma;
mod hw_mtl;
mod inference;
mod mmio;
mod pci;
#[cfg(target_os = "redox")]
mod scheme;
mod status;

use boot::BootSequence;
use hw_mtl::*;
use inference::CommandQueue;
use log::{error, info, warn};
use status::StatusMonitor;

/// Default firmware paths to search
const FW_SEARCH_PATHS: &[&str] = &[
    "/lib/firmware/intel/vpu/vpu_40xx_v0.0.bin",
    "/lib/firmware/intel/vpu_40xx.bin",
    "firmware/vpu_40xx.bin",
    "./vpu_40xx.bin",
];

/// Driver version
const VERSION: &str = "0.1.0";

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    let test_mode = args.iter().any(|a| a == "--test");
    let diag_mode = args.iter().any(|a| a == "--diagnostics");
    let fw_path = args
        .iter()
        .position(|a| a == "--firmware")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    // === Banner ===
    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║                                                  ║");
    println!("║   🧠 Intel NPU Driver for EVA OS               ║");
    println!("║   Version: {}                              ║", VERSION);
    println!("║   Target:  Intel Meteor Lake NPU (VPU 4.0)      ║");
    println!("║   Mode:    Userspace (Zero-Kernel-Crash)         ║");
    println!("║                                                  ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    #[cfg(not(target_os = "redox"))]
    {
        warn!("⚠️  Running in MOCK MODE (not on Redox OS)");
        warn!("   Hardware access is simulated for development.");
        println!();
    }

    // === Run the driver ===
    // We use a separate scope so that all resources (DMA buffers, MMIO mappings)
    // are properly dropped BEFORE process exit, preventing resource leaks.
    let exit_code = match run_driver(fw_path, test_mode, diag_mode) {
        Ok(()) => {
            info!("Driver shut down cleanly.");
            0
        }
        Err(e) => {
            error!("❌ Driver failed: {}", e);
            1
        }
    };

    // Exit AFTER all destructors have run (unlike process::exit which skips them)
    std::process::exit(exit_code);
}

fn run_driver(
    fw_path_override: Option<&str>,
    test_mode: bool,
    diag_mode: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // ================================================================
    // Step 1: PCI Discovery
    // ================================================================
    info!("━━━ Phase 1: PCI Discovery ━━━");

    let npu = pci::discover_npu()?;

    println!("🔍 NPU Found:");
    println!("   Device : {} (ID: {:#06x})", npu.device_name, npu.device_id);
    println!("   PCI BDF: {}", npu.bdf);
    println!("   BAR0   : {:#x} ({} KB)", npu.bar0_phys, npu.bar0_size / 1024);
    println!();

    // ================================================================
    // Step 2: Initial Status Check
    // ================================================================
    info!("━━━ Phase 2: Initial Status ━━━");

    let mut monitor = StatusMonitor::new(&npu.mmio);
    let initial_state = monitor.poll();

    println!("📊 Initial NPU State: {}", initial_state);
    println!("   Raw FW_STATUS : {:#010x}", monitor.raw_status());
    println!("   Buttress      : {:#010x}", monitor.buttress_status());
    println!();

    // If diagnostics only, print and exit
    if diag_mode {
        monitor.print_diagnostics();
        return Ok(());
    }

    // If test mode, just verify PCI discovery works and exit
    if test_mode {
        println!("✅ Test mode: PCI discovery and register read successful!");
        println!("   If you see a raw status value above (even 0x00000000),");
        println!("   the hardware barrier has been broken. 🎉");
        return Ok(());
    }

    // ================================================================
    // Step 3: Find Firmware
    // ================================================================
    info!("━━━ Phase 3: Firmware Location ━━━");

    let fw_path = if let Some(path) = fw_path_override {
        // Validate firmware path: reject path traversal attempts
        if path.contains("..") {
            return Err("Firmware path contains '..' (path traversal rejected)".into());
        }
        info!("Using firmware path from --firmware: {}", path);
        path.to_string()
    } else {
        find_firmware()?
    };

    println!("📦 Firmware: {}", fw_path);
    println!();

    // ================================================================
    // Step 4: Boot Sequence
    // ================================================================
    info!("━━━ Phase 4: Boot Sequence ━━━");

    let boot = BootSequence::new(&npu.mmio);
    let (boot_result, _fw_buffer) = boot.execute(&fw_path)?;

    // IMPORTANT: _fw_buffer must remain alive for the entire driver lifetime.
    // The NPU references the firmware at its physical DMA address.
    // Dropping it would cause a use-after-free on the hardware DMA path.

    match &boot_result {
        boot::BootResult::Ready { fw_version } => {
            println!("🎉 NPU BOOT SUCCESSFUL!");
            println!("   Firmware Version: {:#010x}", fw_version);
        }
        boot::BootResult::Ambiguous { status } => {
            println!("⚠️  NPU boot ambiguous: {:#010x}", status);
        }
    }
    println!();

    // ================================================================
    // Step 5: Initialize Command Queue
    // ================================================================
    info!("━━━ Phase 5: Command Queue Init ━━━");

    let mut cmd_queue = CommandQueue::new(CMD_QUEUE_SIZE)?;
    println!("📋 Command Queue ready ({} slots)", CMD_QUEUE_SIZE);
    println!("   Physical Address: {:#010x}", cmd_queue.phys_addr());

    // Register the command queue physical address with the NPU hardware.
    // The NPU reads commands from this DMA address when the doorbell is rung.
    let queue_phys = cmd_queue.phys_addr();
    npu.mmio.write32(IPC_HOST_2_DEVICE_DATA0, queue_phys as u32);
    npu.mmio.write32(IPC_HOST_2_DEVICE_DATA1, (queue_phys >> 32) as u32);
    info!(
        "Command queue registered with NPU: DATA0={:#010x}, DATA1={:#010x}",
        queue_phys as u32,
        (queue_phys >> 32) as u32
    );
    println!();

    // ================================================================
    // Step 6: Scheme Support (npu:)
    // ================================================================
    info!("━━━ Phase 6: Initializing NPU Scheme ━━━");

    #[cfg(target_os = "redox")]
    {
        use syscall::Scheme;
        let mut scheme = scheme::NpuScheme::new(&npu.mmio, &mut cmd_queue, &mut monitor);
        
        // Open the scheme file to register 'npu:'
        let mut socket = syscall::open(":npu", syscall::O_CREAT | syscall::O_RDWR | syscall::O_CLOEXEC)
            .map_err(|e| format!("Failed to create npu: scheme: {:?}", e))?;

        info!("🚀 Scheme 'npu:' registered. Listening for requests...");

        loop {
            let mut packet = syscall::Packet::default();
            if syscall::read(socket, &mut packet).map_err(|e| format!("Failed to read scheme packet: {:?}", e))? == 0 {
                break;
            }

            scheme.handle(&mut packet);

            syscall::write(socket, &packet).map_err(|e| format!("Failed to write scheme packet: {:?}", e))?;
        }
    }

    #[cfg(not(target_os = "redox"))]
    {
        println!("╔══════════════════════════════════════════════════╗");
        println!("║   🟢 NPU Driver Active (Mock Loop)             ║");
        println!("╚══════════════════════════════════════════════════╝");

        let mut loop_count: u64 = 0;
        loop {
            let state = monitor.poll();
            if loop_count % 12 == 0 {
                info!("Heartbeat: state={}, uptime={:.0}s", state, monitor.uptime().as_secs_f64());
            }
            if state == status::NpuState::Dead { return Err("NPU died".into()); }
            loop_count += 1;
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    Ok(())
}

/// Search for firmware binary in standard locations.
fn find_firmware() -> Result<String, Box<dyn std::error::Error>> {
    for path in FW_SEARCH_PATHS {
        if std::path::Path::new(path).exists() {
            info!("Found firmware at: {}", path);
            return Ok(path.to_string());
        }
    }

    // Create a mock firmware for testing
    #[cfg(not(target_os = "redox"))]
    {
        let mock_path = "firmware/vpu_40xx.bin";
        warn!("⚠️  No firmware found. Creating mock firmware for testing...");

        std::fs::create_dir_all("firmware")?;

        // Create a minimal fake firmware (just a header for testing)
        let mut mock_fw = vec![0u8; 4096];
        // Magic bytes that the NPU expects at firmware start
        mock_fw[0..4].copy_from_slice(&[0x56, 0x50, 0x55, 0x21]); // "VPU!"
        mock_fw[4..8].copy_from_slice(&0x0001_0000u32.to_le_bytes()); // Version 1.0

        std::fs::write(mock_path, &mock_fw)?;
        info!("Created mock firmware: {} ({} bytes)", mock_path, mock_fw.len());

        return Ok(mock_path.to_string());
    }

    #[cfg(target_os = "redox")]
    {
        Err(format!(
            "Firmware not found. Searched: {:?}\n\
             Copy the Intel VPU firmware to one of these locations.\n\
             On Linux: find it in linux-firmware.git as intel/vpu/vpu_40xx_v*.bin",
            FW_SEARCH_PATHS
        )
        .into())
    }
}
