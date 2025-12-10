//! Job execution module
//!
//! This module handles PTY allocation, process spawning, and job lifecycle management.

pub mod process;
pub mod pty;

pub use process::{JobExecutor, JobHandle, ProcessInfo, ProcessManager};
pub use pty::{PtyAllocation, PtyMaster};
