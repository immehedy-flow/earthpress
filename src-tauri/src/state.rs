//! Shared conversion state: one running batch at a time, plus its cancel flag.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct ConversionState {
    cancel: Mutex<Option<Arc<AtomicBool>>>,
    busy: AtomicBool,
}

impl ConversionState {
    /// Claims the single conversion slot.
    ///
    /// Returns the cancel flag for the batch. The flag is shared with every
    /// worker thread, so setting it stops the whole batch.
    pub fn begin(&self) -> Result<Arc<AtomicBool>, String> {
        if self.busy.swap(true, Ordering::SeqCst) {
            return Err("A conversion is already running.".to_string());
        }

        let flag = Arc::new(AtomicBool::new(false));
        let mut slot = match self.cancel.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        *slot = Some(Arc::clone(&flag));

        Ok(flag)
    }

    /// Releases the slot. Safe to call even if `begin` was never called.
    pub fn finish(&self) {
        let mut slot = match self.cancel.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        *slot = None;
        self.busy.store(false, Ordering::SeqCst);
    }

    /// Asks the running batch to stop. No-op when nothing is running.
    pub fn cancel(&self) {
        let slot = match self.cancel.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(flag) = slot.as_ref() {
            flag.store(true, Ordering::SeqCst);
        }
    }

    /// True when a batch currently holds the slot.
    #[cfg(test)]
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_batch_can_run_at_a_time() {
        let state = ConversionState::default();

        state.begin().expect("first batch");
        assert!(state.begin().is_err(), "second batch must be rejected");

        state.finish();
        assert!(!state.is_busy());
        assert!(state.begin().is_ok());
    }

    #[test]
    fn cancel_sets_the_shared_flag() {
        let state = ConversionState::default();
        let flag = state.begin().expect("batch");

        assert!(!flag.load(Ordering::SeqCst));
        state.cancel();
        assert!(flag.load(Ordering::SeqCst));

        state.finish();
    }

    #[test]
    fn cancelling_without_a_batch_is_a_no_op() {
        let state = ConversionState::default();
        state.cancel();
        assert!(!state.is_busy());
    }

    #[test]
    fn finish_clears_the_previous_flag() {
        let state = ConversionState::default();
        let first = state.begin().expect("batch");
        state.finish();

        // Cancelling after the batch ended must not touch the old flag.
        state.cancel();
        assert!(!first.load(Ordering::SeqCst));
    }
}
