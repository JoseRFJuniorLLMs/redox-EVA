//! Inference Engine — Command Queue and Job Submission
//!
//! After the NPU boots and reports 0xF00D, we can submit inference jobs.
//! The NPU uses a ring buffer command queue in DMA memory:
//!
//! ```text
//!  ┌─────────────────────────────────────────┐
//!  │           DMA Command Queue              │
//!  │  ┌─────────┬─────────┬─────────┬─────┐  │
//!  │  │  Cmd 0  │  Cmd 1  │  Cmd 2  │ ... │  │
//!  │  └─────────┴─────────┴─────────┴─────┘  │
//!  │  write_ptr ──▲                           │
//!  │  read_ptr  ──▲ (NPU advances this)       │
//!  └─────────────────────────────────────────┘
//! ```
//!
//! Each command descriptor tells the NPU:
//! - What operation to run (inference, profiling, etc.)
//! - Where the model weights are (DMA address)
//! - Where the input data is (DMA address)
//! - Where to write the output (DMA address)

use crate::dma::{DmaBuffer, DmaError};
use crate::hw_mtl::*;
use crate::mmio::MmioRegion;
use log::{debug, error, info};

/// Type of inference operation.
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum InferenceOp {
    /// Standard inference (forward pass)
    Infer = 0x0001,
    /// Profiling run (with timing data)
    Profile = 0x0002,
    /// Model validation (check weights)
    Validate = 0x0003,
    /// Power state change
    PowerCtl = 0x00F0,
}

/// A command descriptor (64 bytes, matching CMD_DESC_SIZE).
///
/// Debug is implemented manually to avoid potential UB from creating
/// references to misaligned packed fields on older Rust toolchains.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct CommandDescriptor {
    /// Operation type
    pub opcode: u32,
    /// Flags (reserved)
    pub flags: u32,
    /// Physical address of model weights
    pub model_addr_lo: u32,
    pub model_addr_hi: u32,
    /// Model size in bytes
    pub model_size: u32,
    /// Physical address of input data
    pub input_addr_lo: u32,
    pub input_addr_hi: u32,
    /// Input size in bytes
    pub input_size: u32,
    /// Physical address for output buffer
    pub output_addr_lo: u32,
    pub output_addr_hi: u32,
    /// Output buffer size in bytes
    pub output_size: u32,
    /// Job ID (for tracking completion)
    pub job_id: u32,
    /// Padding to 64 bytes
    pub _reserved: [u32; 4],
}

impl CommandDescriptor {
    /// Create a new inference command descriptor.
    ///
    /// Returns `None` if any buffer size exceeds `u32::MAX` (4 GB), since
    /// the NPU command descriptor uses 32-bit size fields.
    pub fn new_inference(
        job_id: u32,
        model: &DmaBuffer,
        input: &DmaBuffer,
        output: &DmaBuffer,
    ) -> Option<Self> {
        // Guard against silent truncation of sizes > 4 GB
        let model_size = u32::try_from(model.size).ok()?;
        let input_size = u32::try_from(input.size).ok()?;
        let output_size = u32::try_from(output.size).ok()?;

        Some(Self {
            opcode: InferenceOp::Infer as u32,
            flags: 0,
            model_addr_lo: model.phys_lo(),
            model_addr_hi: model.phys_hi(),
            model_size,
            input_addr_lo: input.phys_lo(),
            input_addr_hi: input.phys_hi(),
            input_size,
            output_addr_lo: output.phys_lo(),
            output_addr_hi: output.phys_hi(),
            output_size,
            job_id,
            _reserved: [0; 4],
        })
    }

    /// Serialize to bytes for writing into DMA command queue.
    pub fn to_bytes(&self) -> [u8; CMD_DESC_SIZE] {
        // Compile-time guarantee: struct size must match descriptor size
        const _: () = assert!(
            std::mem::size_of::<CommandDescriptor>() == CMD_DESC_SIZE,
            "CommandDescriptor size does not match CMD_DESC_SIZE"
        );
        unsafe { std::mem::transmute_copy(self) }
    }
}

impl std::fmt::Debug for CommandDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Copy fields to aligned locals to avoid UB from packed references
        let opcode = self.opcode;
        let flags = self.flags;
        let job_id = self.job_id;
        let model_lo = self.model_addr_lo;
        let model_hi = self.model_addr_hi;
        let model_sz = self.model_size;
        let input_lo = self.input_addr_lo;
        let input_hi = self.input_addr_hi;
        let input_sz = self.input_size;
        let output_lo = self.output_addr_lo;
        let output_hi = self.output_addr_hi;
        let output_sz = self.output_size;
        f.debug_struct("CommandDescriptor")
            .field("opcode", &format_args!("{:#06x}", opcode))
            .field("flags", &flags)
            .field("job_id", &job_id)
            .field("model_addr", &format_args!("{:#010x}_{:08x}", model_hi, model_lo))
            .field("model_size", &model_sz)
            .field("input_addr", &format_args!("{:#010x}_{:08x}", input_hi, input_lo))
            .field("input_size", &input_sz)
            .field("output_addr", &format_args!("{:#010x}_{:08x}", output_hi, output_lo))
            .field("output_size", &output_sz)
            .finish()
    }
}

/// The command queue ring buffer in DMA memory.
pub struct CommandQueue {
    /// DMA buffer holding the ring of command descriptors
    ring: DmaBuffer,
    /// Current write position (index into ring)
    write_idx: usize,
    /// Maximum number of entries
    capacity: usize,
    /// Next job ID to assign
    next_job_id: u32,
}

impl CommandQueue {
    /// Create a new command queue with the given capacity.
    ///
    /// Capacity must be > 0 to avoid divide-by-zero in ring buffer wrapping.
    pub fn new(capacity: usize) -> Result<Self, DmaError> {
        if capacity == 0 {
            return Err(DmaError::ZeroSize);
        }

        let total_size = capacity.checked_mul(CMD_DESC_SIZE)
            .ok_or(DmaError::OutOfBounds { offset: 0, len: capacity, capacity: CMD_DESC_SIZE })?;
        info!(
            "Creating command queue: {} entries × {} bytes = {} bytes",
            capacity, CMD_DESC_SIZE, total_size
        );

        let ring = DmaBuffer::new(total_size)?;

        info!(
            "Command queue at phys={:#010x}",
            ring.phys_addr
        );

        Ok(Self {
            ring,
            write_idx: 0,
            capacity,
            next_job_id: 1,
        })
    }

    /// Submit an inference job to the queue.
    ///
    /// Returns the job_id that can be used to track completion.
    pub fn submit(
        &mut self,
        mmio: &MmioRegion,
        model: &DmaBuffer,
        input: &DmaBuffer,
        output: &DmaBuffer,
    ) -> Result<u32, InferenceError> {
        let job_id = self.next_job_id;
        self.next_job_id += 1;

        info!("Submitting inference job #{}", job_id);

        // Build the command descriptor
        let cmd = CommandDescriptor::new_inference(job_id, model, input, output)
            .ok_or(InferenceError::BufferTooLarge)?;
        let cmd_bytes = cmd.to_bytes();

        // Write to the next slot in the ring (checked_mul prevents overflow)
        let offset = self.write_idx.checked_mul(CMD_DESC_SIZE)
            .ok_or(InferenceError::QueueFull)?;
        self.ring
            .write_bytes(offset, &cmd_bytes)
            .map_err(|e| InferenceError::QueueWrite(e))?;

        debug!(
            "  Written CMD at ring offset {:#x} (slot {})",
            offset, self.write_idx
        );

        // Advance write pointer (wrap around)
        self.write_idx = (self.write_idx + 1) % self.capacity;

        // Ring the doorbell to notify NPU — bit 31 must be set
        mmio.write32(IPC_HOST_2_DEVICE_DRBL, IPC_DRBL_TRIGGER);
        debug!("  Doorbell rung for job #{}", job_id);

        Ok(job_id)
    }

    /// Get the physical address of the command queue (for NPU registration).
    pub fn phys_addr(&self) -> u64 {
        self.ring.phys_addr
    }

    /// Get queue statistics.
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            capacity: self.capacity,
            write_idx: self.write_idx,
            total_submitted: self.next_job_id as usize - 1,
        }
    }
}

/// Queue statistics for monitoring.
#[derive(Debug)]
pub struct QueueStats {
    pub capacity: usize,
    pub write_idx: usize,
    pub total_submitted: usize,
}

impl std::fmt::Display for QueueStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Queue: write_idx={}, capacity={}, total_submitted={}",
            self.write_idx, self.capacity, self.total_submitted
        )
    }
}

// ============================================================
// Inference Job Helpers
// ============================================================

/// Prepare an input buffer from raw data (e.g., audio samples, image pixels).
pub fn prepare_input(data: &[u8]) -> Result<DmaBuffer, DmaError> {
    let buf = DmaBuffer::new(data.len())?;
    buf.write_bytes(0, data)?;
    debug!("Input buffer: {} bytes at phys={:#x}", data.len(), buf.phys_addr);
    Ok(buf)
}

/// Allocate an output buffer of the given size.
pub fn prepare_output(size: usize) -> Result<DmaBuffer, DmaError> {
    let buf = DmaBuffer::new(size)?;
    debug!("Output buffer: {} bytes at phys={:#x}", size, buf.phys_addr);
    Ok(buf)
}

/// Read inference results from an output buffer.
///
/// Uses volatile reads to ensure hardware-written DMA data is read correctly.
pub fn read_output(output: &DmaBuffer) -> Vec<u8> {
    output.read_all()
}

// ============================================================
// Error Types
// ============================================================

#[derive(Debug)]
pub enum InferenceError {
    QueueWrite(DmaError),
    QueueFull,
    BufferTooLarge,
    Timeout { job_id: u32 },
    NpuError { job_id: u32, status: u32 },
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QueueWrite(e) => write!(f, "Failed to write command to queue: {}", e),
            Self::QueueFull => write!(f, "Command queue is full"),
            Self::BufferTooLarge => write!(f, "DMA buffer exceeds u32::MAX (4 GB limit for NPU descriptors)"),
            Self::Timeout { job_id } => write!(f, "Inference job #{} timed out", job_id),
            Self::NpuError { job_id, status } => {
                write!(f, "NPU error on job #{}: status={:#010x}", job_id, status)
            }
        }
    }
}

impl std::error::Error for InferenceError {}
