//! Redox NPU Scheme Implementation
//!
//! Exposes the NPU hardware via the `npu:` scheme, allowing other processes
//! to submit inference jobs using simple file operations.
//!
//! Protocol:
//!   - `open("npu:infer", O_RDWR)` -> returns a handle for inference
//!   - `write(handle, cmd_buffer)` -> submits a job
//!   - `read(handle, result_buffer)` -> waits for and reads result
//!   - `fstat(handle)` -> returns job status
//!
//! Note: This module only compiles on Redox OS, as it depends on the
//! `syscall` crate's `Scheme` trait.

use std::collections::HashMap;
use syscall::{Error, Result, Scheme, Stat, EBADF, EINVAL};
use crate::inference::{CommandQueue, CommandDescriptor};
use crate::mmio::MmioRegion;
use crate::status::StatusMonitor;

/// A handle to an open NPU resource
pub enum NpuHandle {
    /// Global status handle (npu:)
    Status,
    /// Active inference session (npu:infer)
    Inference {
        job_id: Option<u32>,
    },
}

pub struct NpuScheme<'a> {
    /// Reference to the hardware MMIO
    mmio: &'a MmioRegion,
    /// Reference to the command queue
    queue: &'a mut CommandQueue,
    /// Reference to status monitor
    monitor: &'a mut StatusMonitor<'a>,
    /// Active handles
    handles: HashMap<usize, NpuHandle>,
    /// Next handle ID
    next_id: usize,
}

impl<'a> NpuScheme<'a> {
    pub fn new(mmio: &'a MmioRegion, queue: &'a mut CommandQueue, monitor: &'a mut StatusMonitor<'a>) -> Self {
        Self {
            mmio,
            queue,
            monitor,
            handles: HashMap::new(),
            next_id: 0,
        }
    }
}

impl<'a> Scheme for NpuScheme<'a> {
    fn open(&mut self, path: &str, _flags: usize, uid: u32, _gid: u32) -> Result<usize> {
        // Security: Only root (uid 0) can access inference operations.
        // Status is readable by anyone for monitoring.
        if path == "infer" && uid != 0 {
            log::warn!("Non-root user (uid={}) denied access to npu:infer", uid);
            return Err(Error::new(syscall::EACCES));
        }

        let handle = match path {
            "" | "status" => NpuHandle::Status,
            "infer" => NpuHandle::Inference { job_id: None },
            _ => return Err(Error::new(syscall::ENOENT)),
        };

        let id = self.next_id;
        self.next_id += 1;
        self.handles.insert(id, handle);
        Ok(id)
    }

    fn read(&mut self, id: usize, buf: &mut [u8]) -> Result<usize> {
        let handle = self.handles.get_mut(&id).ok_or(Error::new(EBADF))?;

        match handle {
            NpuHandle::Status => {
                let status = format!("state: {}\nstats: {}\n", self.monitor.poll(), self.queue.stats());
                let bytes = status.as_bytes();
                let len = std::cmp::min(buf.len(), bytes.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            NpuHandle::Inference { job_id } => {
                if let Some(jid) = job_id {
                    let msg = format!("Job #{} completed (MOCK RESULT)\n", jid);
                    let bytes = msg.as_bytes();
                    let len = std::cmp::min(buf.len(), bytes.len());
                    buf[..len].copy_from_slice(&bytes[..len]);
                    Ok(len)
                } else {
                    Err(Error::new(EINVAL))
                }
            }
        }
    }

    fn write(&mut self, id: usize, buf: &[u8]) -> Result<usize> {
        let handle = self.handles.get_mut(&id).ok_or(Error::new(EBADF))?;

        match handle {
            NpuHandle::Inference { job_id } => {
                if buf.len() < std::mem::size_of::<CommandDescriptor>() {
                    return Err(Error::new(EINVAL));
                }

                // In a real implementation, we'd parse the buffer into a job
                // For now, we simulate submission
                // let job = self.queue.submit(self.mmio, ...)?;
                // *job_id = Some(job);

                Ok(buf.len())
            }
            _ => Err(Error::new(EBADF)),
        }
    }

    fn close(&mut self, id: usize) -> Result<usize> {
        self.handles.remove(&id).ok_or(Error::new(EBADF))?;
        Ok(0)
    }

    fn fstat(&mut self, id: usize, stat: &mut Stat) -> Result<usize> {
        let _handle = self.handles.get(&id).ok_or(Error::new(EBADF))?;
        stat.st_mode = syscall::MODE_FILE | 0o666;
        stat.st_size = 0;
        Ok(0)
    }
}
