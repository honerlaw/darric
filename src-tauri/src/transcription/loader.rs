//! Single-flight acquisition of the process-wide whisper transcriber.
//!
//! Two callers want the model: the startup pre-load in `lib.rs`, and
//! [`crate::commands::sessions`] when a recording begins. Each used to call
//! [`crate::model::ensure_model`] independently, so a session started while the
//! startup download was still running raced it — both streamed into the same
//! `.tmp` path and both then tried to rename it. Exactly one rename can succeed;
//! the loser got `ENOENT`, turned it into a `None` transcriber, and the
//! recording captured and metered audio that nothing ever transcribed.
//!
//! The fix is mutual exclusion over the whole acquire-and-load sequence rather
//! than a cleverer temp filename: the lock is held across `ensure_model` *and*
//! [`Transcriber::new`], so a second caller blocks and then finds the finished
//! transcriber instead of starting a competing download.

use super::Transcriber;
use crate::error::{AppError, Result};
use crate::state::DbConn;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

/// The shared transcriber, once loaded.
///
/// A `tokio::sync::Mutex<Option<_>>` rather than a `OnceCell`: a `OnceCell`
/// resolved to an error caches that error for the life of the process, so one
/// transient network failure at launch would disable transcription until the app
/// restarted. Re-checking under a lock leaves the next caller free to retry.
pub type TranscriberSlot = Arc<Mutex<Option<Arc<Transcriber>>>>;

/// Return the value in `slot`, running `init` to produce it if it is empty.
///
/// The lock is held across `init`, which is what makes this single-flight:
/// concurrent callers do not each run `init`, they queue and then observe the
/// value the first one stored. A failing `init` stores nothing, so the next
/// caller retries rather than inheriting the failure.
///
/// The `Send`/`Sync` bounds are required, not incidental: `lib.rs` spawns this
/// onto the Tauri runtime, so the returned future has to be `Send`.
async fn get_or_init<T, Fut>(
    slot: &Mutex<Option<Arc<T>>>,
    init: impl FnOnce() -> Fut + Send,
) -> Result<Arc<T>>
where
    T: Send + Sync,
    Fut: Future<Output = Result<T>> + Send,
{
    let mut guard = slot.lock().await;
    if let Some(existing) = guard.as_ref() {
        return Ok(Arc::clone(existing));
    }
    let value = Arc::new(init().await?);
    *guard = Some(Arc::clone(&value));
    Ok(value)
}

/// The user's overriding model path, if they have set one.
fn custom_model_path(db: &DbConn) -> Option<String> {
    db.0.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .query_row(
            "SELECT value FROM settings WHERE key = 'whisper_model_path'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .filter(|s| !s.is_empty())
}

/// Get the shared transcriber, downloading and loading the model on first use.
///
/// Concurrent callers never race: the second blocks until the first finishes and
/// then receives the same `Arc`.
pub async fn get_or_load(
    app: &AppHandle,
    slot: &TranscriberSlot,
    db: &DbConn,
) -> Result<Arc<Transcriber>> {
    // Logged before the lock is taken, so a start that is waiting on someone
    // else's in-flight download is visible in the log rather than looking hung.
    log::info!("[transcriber] requesting the shared whisper model");
    get_or_init(slot, || async {
        let path = match custom_model_path(db) {
            Some(p) => PathBuf::from(p),
            None => crate::model::ensure_model(app).await?,
        };

        log::info!(
            "[transcriber] loading whisper model from {}",
            path.display()
        );
        let path_str = path.to_string_lossy().into_owned();
        let transcriber = tokio::task::spawn_blocking(move || Transcriber::new(&path_str))
            .await
            .map_err(|e| {
                AppError::Transcription(format!("loading the whisper model panicked: {e}"))
            })??;

        log::info!("[transcriber] ready");
        app.emit("model_ready", ()).ok();
        Ok(transcriber)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Stands in for `Transcriber` — `get_or_init` is generic precisely so the
    /// single-flight property can be tested without a 1.6 GB model download.
    #[derive(Debug, PartialEq, Eq)]
    struct Fake(usize);

    #[tokio::test]
    async fn concurrent_callers_load_once_and_share_the_result() {
        // The bug this module exists to prevent: two callers each ran their own
        // load. Here `runs` must end at 1 no matter how many ask.
        let slot: Mutex<Option<Arc<Fake>>> = Mutex::new(None);
        let runs = AtomicUsize::new(0);

        let init = || async {
            let n = runs.fetch_add(1, Ordering::SeqCst);
            // Yield so the other callers are polled mid-load; without the lock
            // they would each proceed to run `init` themselves.
            tokio::task::yield_now().await;
            Ok(Fake(n))
        };

        let (a, b, c) = tokio::join!(
            get_or_init(&slot, init),
            get_or_init(&slot, init),
            get_or_init(&slot, init),
        );

        assert_eq!(runs.load(Ordering::SeqCst), 1, "init must run exactly once");
        let (a, b, c) = (
            a.expect("first caller"),
            b.expect("second caller"),
            c.expect("third caller"),
        );
        assert!(Arc::ptr_eq(&a, &b), "callers share one transcriber");
        assert!(Arc::ptr_eq(&b, &c), "callers share one transcriber");
    }

    #[tokio::test]
    async fn a_failed_load_is_not_cached() {
        // A OnceCell would poison the process here: one offline launch and
        // transcription never works again until restart.
        let slot: Mutex<Option<Arc<Fake>>> = Mutex::new(None);

        let failed = get_or_init(&slot, || async {
            Err(AppError::Transcription("network is down".into()))
        })
        .await;
        assert!(failed.is_err(), "the first load fails");
        assert!(slot.lock().await.is_none(), "nothing is cached");

        let recovered = get_or_init(&slot, || async { Ok(Fake(7)) })
            .await
            .expect("the next caller retries");
        assert_eq!(*recovered, Fake(7));
    }

    #[tokio::test]
    async fn an_already_loaded_value_is_reused() {
        let slot: Mutex<Option<Arc<Fake>>> = Mutex::new(None);
        let first = get_or_init(&slot, || async { Ok(Fake(1)) })
            .await
            .expect("first load");

        let second = get_or_init(&slot, || async {
            panic!("must not reload an already-loaded transcriber")
        })
        .await
        .expect("second call");

        assert!(Arc::ptr_eq(&first, &second));
    }
}
