//! One-at-a-time job cancellation slot, shared by every cancellable command.
//!
//! Four commands (video compress, audio extract, GIF export, PDF rasterize)
//! previously each hand-rolled the same `Mutex<Option<Arc<AtomicBool>>>`
//! dance: register-or-reject-if-busy, cancel by setting the flag, and a
//! finish that only clears the slot if it still holds *this* job's flag
//! (`Arc::ptr_eq`) so a slow finisher can't clobber a newer job.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct CancelSlot(Mutex<Option<Arc<AtomicBool>>>);

impl CancelSlot {
    pub const fn new() -> Self {
        Self(Mutex::new(None))
    }

    /// Registers a new job. Errors with `busy_msg` if one is already running.
    pub fn start(&self, busy_msg: &str) -> Result<Arc<AtomicBool>, String> {
        let mut slot = self.0.lock().map_err(|e| e.to_string())?;
        if slot.is_some() {
            return Err(busy_msg.to_string());
        }
        let flag = Arc::new(AtomicBool::new(false));
        *slot = Some(flag.clone());
        Ok(flag)
    }

    /// Clears the slot — but only if it still holds `flag`. A job that
    /// finishes late must not clobber the slot a newer job registered.
    pub fn finish(&self, flag: &Arc<AtomicBool>) -> Result<(), String> {
        let mut slot = self.0.lock().map_err(|e| e.to_string())?;
        if slot
            .as_ref()
            .map(|current| Arc::ptr_eq(current, flag))
            .unwrap_or(false)
        {
            *slot = None;
        }
        Ok(())
    }

    /// Signals the running job (if any) to cancel. Best-effort by design.
    pub fn cancel(&self) -> Result<(), String> {
        if let Some(flag) = self.0.lock().map_err(|e| e.to_string())?.as_ref() {
            flag.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_rejects_while_busy_and_frees_after_finish() {
        let slot = CancelSlot::new();
        let flag = slot.start("busy").unwrap();
        assert_eq!(slot.start("busy").unwrap_err(), "busy");
        slot.finish(&flag).unwrap();
        assert!(slot.start("busy").is_ok());
    }

    #[test]
    fn cancel_sets_the_active_flag() {
        let slot = CancelSlot::new();
        let flag = slot.start("busy").unwrap();
        assert!(!flag.load(Ordering::SeqCst));
        slot.cancel().unwrap();
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn cancel_without_active_job_is_a_noop() {
        let slot = CancelSlot::new();
        assert!(slot.cancel().is_ok());
    }

    #[test]
    fn late_finish_does_not_clobber_newer_job() {
        let slot = CancelSlot::new();
        let old = slot.start("busy").unwrap();
        slot.finish(&old).unwrap();
        let newer = slot.start("busy").unwrap();
        // The old job finishing again must not free the newer job's slot.
        slot.finish(&old).unwrap();
        assert_eq!(slot.start("busy").unwrap_err(), "busy");
        slot.finish(&newer).unwrap();
        assert!(slot.start("busy").is_ok());
    }
}
