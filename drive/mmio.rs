//! Memory-Mapped I/O Interface
//!
//! Provides safe abstractions for reading/writing hardware registers
//! via memory-mapped BAR0 region. On Redox, this is obtained by
//! opening the PCI BAR file and mmap'ing it.

use std::sync::atomic::{fence, Ordering};

/// Raw MMIO region mapped into our virtual address space.
///
/// # Safety
/// The caller must ensure `base` points to a valid mmap'd BAR region
/// and that `size` does not exceed the actual BAR size.
pub struct MmioRegion {
    base: *mut u8,
    size: usize,
}

// Safety: MmioRegion can be sent to another thread (ownership transfer).
// We intentionally do NOT implement Sync — concurrent &MmioRegion access
// from multiple threads would cause data races on hardware registers.
// The driver is single-threaded by design (Redox scheme loop).
unsafe impl Send for MmioRegion {}

impl MmioRegion {
    /// Create a new MMIO region from a raw pointer and size.
    ///
    /// # Safety
    /// - `base` must be a valid pointer to an mmap'd region
    /// - `size` must not exceed the mapped region
    /// - The region must remain mapped for the lifetime of this struct
    pub unsafe fn new(base: *mut u8, size: usize) -> Self {
        Self { base, size }
    }

    /// Read a 32-bit register at `offset` bytes from base.
    ///
    /// Returns `0xFFFF_FFFF` on out-of-bounds access (same behavior as PCI
    /// for non-existent registers), preventing driver panics.
    pub fn read32(&self, offset: usize) -> u32 {
        let end = match offset.checked_add(4) {
            Some(e) => e,
            None => {
                log::error!("MMIO read32 overflow: offset {:#x}", offset);
                return 0xFFFF_FFFF;
            }
        };
        if end > self.size {
            log::error!(
                "MMIO read32 out of bounds: offset {:#x}, region size {:#x}",
                offset, self.size
            );
            return 0xFFFF_FFFF;
        }

        // Memory fence before read to ensure ordering
        fence(Ordering::SeqCst);

        unsafe {
            let ptr = self.base.add(offset) as *const u32;
            // Volatile read: compiler cannot optimize this away
            std::ptr::read_volatile(ptr)
        }
    }

    /// Write a 32-bit value to register at `offset` bytes from base.
    ///
    /// Silently drops the write on out-of-bounds access (preventing driver
    /// panics). Logs an error for debugging.
    pub fn write32(&self, offset: usize, value: u32) {
        let end = match offset.checked_add(4) {
            Some(e) => e,
            None => {
                log::error!("MMIO write32 overflow: offset {:#x}", offset);
                return;
            }
        };
        if end > self.size {
            log::error!(
                "MMIO write32 out of bounds: offset {:#x}, region size {:#x}",
                offset, self.size
            );
            return;
        }

        unsafe {
            let ptr = self.base.add(offset) as *mut u32;
            // Volatile write: ensures the write hits the hardware
            std::ptr::write_volatile(ptr, value);
        }

        // Memory fence after write to ensure it propagates
        fence(Ordering::SeqCst);
    }

    /// Read a 64-bit register at `offset` (two consecutive 32-bit reads).
    pub fn read64(&self, offset: usize) -> u64 {
        let lo = self.read32(offset) as u64;
        let hi = self.read32(offset + 4) as u64;
        (hi << 32) | lo
    }

    /// Write a 64-bit value as two 32-bit writes (low then high).
    pub fn write64(&self, offset: usize, value: u64) {
        self.write32(offset, value as u32);
        self.write32(offset + 4, (value >> 32) as u32);
    }

    /// Set specific bits in a register (read-modify-write).
    ///
    /// # Warning: TOCTOU
    /// This is NOT atomic. The register value can change between the read
    /// and write. Only use for registers that are not concurrently modified
    /// by hardware or firmware. For status registers that hardware updates,
    /// use explicit read32/write32 sequences with appropriate checks.
    pub fn set_bits32(&self, offset: usize, bits: u32) {
        let current = self.read32(offset);
        self.write32(offset, current | bits);
    }

    /// Clear specific bits in a register (read-modify-write).
    ///
    /// # Warning: TOCTOU
    /// This is NOT atomic. See `set_bits32` for details.
    pub fn clear_bits32(&self, offset: usize, bits: u32) {
        let current = self.read32(offset);
        self.write32(offset, current & !bits);
    }

    /// Poll a register until a condition is met or timeout expires.
    ///
    /// Returns `Ok(final_value)` if condition met, `Err(last_value)` on timeout.
    pub fn poll_until<F>(
        &self,
        offset: usize,
        condition: F,
        poll_interval_ms: u64,
        timeout_ms: u64,
    ) -> Result<u32, u32>
    where
        F: Fn(u32) -> bool,
    {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let interval = std::time::Duration::from_millis(poll_interval_ms);

        loop {
            let value = self.read32(offset);
            if condition(value) {
                return Ok(value);
            }
            if start.elapsed() >= timeout {
                return Err(value);
            }
            std::thread::sleep(interval);
        }
    }

    /// Get the base pointer (for advanced/unsafe operations).
    pub fn base_ptr(&self) -> *mut u8 {
        self.base
    }

    /// Get the region size.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Dump a range of registers for debugging.
    pub fn dump_range(&self, start_offset: usize, count: usize) {
        log::debug!("=== MMIO Dump: {:#x} to {:#x} ===", start_offset, start_offset + count * 4);
        for i in 0..count {
            let offset = start_offset + i * 4;
            if offset + 4 <= self.size {
                let val = self.read32(offset);
                if val != 0 {
                    log::debug!("  [{:#06x}] = {:#010x}", offset, val);
                }
            }
        }
    }
}

impl Drop for MmioRegion {
    fn drop(&mut self) {
        log::info!("MMIO region dropped (base={:p}, size={:#x})", self.base, self.size);
        // Note: actual munmap is handled by the file descriptor owner (DmaBuffer / BAR file)
    }
}
