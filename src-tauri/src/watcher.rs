use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// Watch `dir` and invoke `on_change` (debounced by `settle`) on any filesystem event.
/// Returns the watcher (kept alive by the caller) — dropping it stops watching.
pub fn watch_dir<F>(
    dir: &Path,
    settle: Duration,
    on_change: F,
) -> notify::Result<RecommendedWatcher>
where
    F: Fn() + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        if res.is_ok() {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(dir, RecursiveMode::NonRecursive)?;

    // Debounce: coalesce a burst of events into one callback after `settle`.
    std::thread::spawn(move || {
        while let Ok(()) = rx.recv() {
            while rx.recv_timeout(settle).is_ok() {}
            on_change();
        }
    });
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn fires_callback_on_file_change() {
        let dir = tempdir().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        let _watcher = watch_dir(dir.path(), Duration::from_millis(50), move || {
            c.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();

        std::fs::write(dir.path().join("tasks.json"), b"{}").unwrap();
        // Allow the event to propagate + debounce window to elapse.
        std::thread::sleep(Duration::from_millis(600));
        assert!(
            count.load(Ordering::SeqCst) >= 1,
            "watcher should fire at least once"
        );
    }
}
