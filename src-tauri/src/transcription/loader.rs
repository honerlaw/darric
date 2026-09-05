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

/// Whether [`get_or_init`] reused the stored value or had to produce one.
///
/// Returned rather than logged in place because `get_or_init` is generic and has
/// no name for what it is loading. The distinction is the single most useful
/// line when diagnosing this area: it says whether a slow start was waiting on a
/// download or was served instantly from the slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    Cached,
    Loaded,
}

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
) -> Result<(Arc<T>, Origin)>
where
    T: Send + Sync,
    Fut: Future<Output = Result<T>> + Send,
{
    let mut guard = slot.lock().await;
    if let Some(existing) = guard.as_ref() {
        return Ok((Arc::clone(existing), Origin::Cached));
    }
    let value = Arc::new(init().await?);
    *guard = Some(Arc::clone(&value));
    Ok((value, Origin::Loaded))
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
    // Probed rather than logged unconditionally: a line emitted on every call
    // says nothing, whereas a failed `try_lock` means someone really is mid-load
    // and this caller is about to wait minutes for them. Advisory only — losing
    // the race between the probe and the real lock costs a log line, nothing more.
    if slot.try_lock().is_err() {
        log::info!("[transcriber] waiting for an in-flight model load");
    }

    let (transcriber, origin) = get_or_init(slot, || async {
        let path = match custom_model_path(db) {
            // A configured path that no longer exists used to be irrelevant,
            // because the startup pre-load ignored the setting entirely. Now that
            // both paths consult it, honouring a stale one would fail every
            // `Transcriber::new` forever with no way to clear it from the UI —
            // the setting is written to the database and read nowhere in `src/`.
            Some(p) if std::path::Path::new(&p).exists() => PathBuf::from(p),
            Some(p) => {
                log::warn!(
                    "[transcriber] configured whisper_model_path {p:?} does not exist — \
                     falling back to the downloaded model"
                );
                crate::model::ensure_model(app).await?
            }
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

        // Note that on the startup path this reaches no webview at all — see
        // `2026-09-05-constraint-tauri-events-from-setup-reach-no-webview`. It is
        // kept for the session path and for parity with the previous behaviour.
        app.emit("model_ready", ()).ok();
        Ok(transcriber)
    })
    .await?;

    match origin {
        Origin::Cached => log::info!("[transcriber] served from the already-loaded model"),
        Origin::Loaded => log::info!("[transcriber] ready"),
    }
    Ok(transcriber)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Stands in for `Transcriber` — `get_or_init` is generic precisely so the
    /// single-flight property can be tested without a 1.6 GB model download.
    #[derive(Debug, PartialEq, Eq)]
    struct Fake(usize);

    /// Multi-threaded so the callers genuinely contend rather than merely
    /// interleaving at await points inside one task.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_callers_load_once_and_share_the_result() {
        // The bug this module exists to prevent: two callers each ran their own
        // load. Here `runs` must end at 1 no matter how many ask.
        let slot: Arc<Mutex<Option<Arc<Fake>>>> = Arc::new(Mutex::new(None));
        let runs = Arc::new(AtomicUsize::new(0));

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let slot = Arc::clone(&slot);
                let runs = Arc::clone(&runs);
                tokio::spawn(async move {
                    get_or_init(&slot, || async {
                        let n = runs.fetch_add(1, Ordering::SeqCst);
                        // Long enough that a missing lock lets the others in.
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        Ok(Fake(n))
                    })
                    .await
                })
            })
            .collect();

        let mut values = Vec::new();
        for h in handles {
            values.push(h.await.expect("task").expect("load"));
        }

        assert_eq!(runs.load(Ordering::SeqCst), 1, "init must run exactly once");
        assert_eq!(
            values.iter().filter(|(_, o)| *o == Origin::Loaded).count(),
            1,
            "exactly one caller reports having loaded it"
        );
        for (v, _) in &values[1..] {
            assert!(
                Arc::ptr_eq(&values[0].0, v),
                "callers share one transcriber"
            );
        }
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

        let (recovered, origin) = get_or_init(&slot, || async { Ok(Fake(7)) })
            .await
            .expect("the next caller retries");
        assert_eq!(*recovered, Fake(7));
        assert_eq!(origin, Origin::Loaded);
    }

    /// The reported scenario's shape: a caller is already queued behind a load
    /// that then fails. It must run its own `init` rather than inherit the
    /// failure or observe a half-stored value.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_caller_queued_behind_a_failing_load_retries_it() {
        let slot: Arc<Mutex<Option<Arc<Fake>>>> = Arc::new(Mutex::new(None));

        let first = {
            let slot = Arc::clone(&slot);
            tokio::spawn(async move {
                get_or_init(&slot, || async {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    Err(AppError::Transcription("download failed".into()))
                })
                .await
            })
        };
        // Give the first caller time to take the lock before the second queues.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let second = {
            let slot = Arc::clone(&slot);
            tokio::spawn(async move { get_or_init(&slot, || async { Ok(Fake(2)) }).await })
        };

        assert!(first.await.expect("task").is_err(), "the first load fails");
        let (value, origin) = second
            .await
            .expect("task")
            .expect("the queued caller recovers");
        assert_eq!(*value, Fake(2));
        assert_eq!(
            origin,
            Origin::Loaded,
            "it ran its own init, not a cached one"
        );
    }

    #[tokio::test]
    async fn an_already_loaded_value_is_reused() {
        let slot: Mutex<Option<Arc<Fake>>> = Mutex::new(None);
        let (first, first_origin) = get_or_init(&slot, || async { Ok(Fake(1)) })
            .await
            .expect("first load");
        assert_eq!(first_origin, Origin::Loaded);

        let (second, second_origin) = get_or_init(&slot, || async {
            panic!("must not reload an already-loaded transcriber")
        })
        .await
        .expect("second call");

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(second_origin, Origin::Cached);
    }
}
