//! Dependency-free parallel map used by file inspection and batch conversion.
//!
//! Replaces the `Promise.allSettled` + fixed-size batching of the Electron
//! build with a worker pool that keeps input order and never spawns more than
//! `workers` threads.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Runs `task` over every item with at most `workers` concurrent threads,
/// returning results in the same order as `items`.
pub fn map_ordered<T, R, F>(items: &[T], workers: usize, task: F) -> Vec<R>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> R + Sync,
{
    if items.is_empty() {
        return Vec::new();
    }

    let workers = workers.clamp(1, items.len());
    let slots: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    let cursor = AtomicUsize::new(0);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let index = cursor.fetch_add(1, Ordering::SeqCst);
                if index >= items.len() {
                    break;
                }

                let value = task(&items[index]);

                match slots.lock() {
                    Ok(mut guard) => guard[index] = Some(value),
                    Err(poisoned) => poisoned.into_inner()[index] = Some(value),
                }
            });
        }
    });

    let slots = match slots.into_inner() {
        Ok(slots) => slots,
        Err(poisoned) => poisoned.into_inner(),
    };

    slots.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn keeps_input_order_while_running_concurrently() {
        let items: Vec<u32> = (0..64).collect();
        let peak = AtomicUsize::new(0);
        let active = AtomicUsize::new(0);

        let results = map_ordered(&items, 4, |item| {
            let now = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(1));
            active.fetch_sub(1, Ordering::SeqCst);
            item * 2
        });

        assert_eq!(
            results,
            items.iter().map(|item| item * 2).collect::<Vec<_>>()
        );
        assert!(peak.load(Ordering::SeqCst) > 1, "expected real concurrency");
    }

    #[test]
    fn empty_input_returns_empty_output() {
        let items: Vec<u32> = Vec::new();
        let results: Vec<u32> = map_ordered(&items, 4, |item| *item);
        assert!(results.is_empty());
    }

    #[test]
    fn single_item_and_single_worker_both_work() {
        let items = vec![7u32];
        assert_eq!(map_ordered(&items, 8, |item| *item + 1), vec![8]);
        assert_eq!(map_ordered(&items, 0, |item| *item + 1), vec![8]);
    }
}
