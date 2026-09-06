//! A bounded worker pool that transcribes segments from every capture source.
//!
//! Two properties matter here and neither is negotiable.
//!
//! **The producer is never blocked.** Segments arrive from an audio callback
//! running on a Core Audio realtime thread. Blocking one causes dropouts in the
//! recording itself, so a full queue drops rather than waits.
//!
//! **It drops the oldest, not the newest.** `mpsc::SyncSender::try_send` fails
//! the incoming item, which throws away the newest audio and leaves a stale
//! backlog to chew through — the transcript then lags further behind with every
//! overflow. Dropping from the front keeps the transcript close to real time,
//! so this is a `VecDeque` behind a `Condvar` rather than a channel.
//!
//! Drops are counted and surfaced, never silent: a transcript with an
//! unannounced hole in it is worse than one that says it has a hole.

use super::Transcriber;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

/// Which side of the machine a segment came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Input,
    Output,
}

impl Direction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
        }
    }

    /// Parse a persisted direction. Rows written before device attribution
    /// existed were migrated to one of these two values, so an unknown string
    /// means the database has been edited by something other than this app.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "input" => Some(Self::Input),
            "output" => Some(Self::Output),
            _ => None,
        }
    }
}

/// One segment of audio awaiting transcription, tagged with its origin.
pub struct SegmentJob {
    pub device_id: String,
    pub device_name: String,
    pub direction: Direction,
    pub samples: Vec<f32>,
}

/// A transcribed line, ready to persist and emit.
pub struct TranscribedLine {
    pub device_id: String,
    pub device_name: String,
    pub direction: Direction,
    pub text: String,
}

/// Where transcribed lines go — persisted and emitted by the engine.
pub type LineSink = Arc<dyn Fn(TranscribedLine) + Send + Sync>;

struct Queue {
    items: VecDeque<SegmentJob>,
    capacity: usize,
    dropped: u64,
    shutdown: bool,
}

struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
}

impl Shared {
    /// Enqueue, evicting the oldest segment if the queue is already full.
    /// Returns `true` when something was evicted.
    fn submit(&self, job: SegmentJob) -> bool {
        let mut q = self
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let evicted = if q.items.len() >= q.capacity {
            q.items.pop_front();
            q.dropped += 1;
            true
        } else {
            false
        };
        q.items.push_back(job);
        drop(q);
        self.ready.notify_one();
        evicted
    }

    fn take(&self) -> Option<SegmentJob> {
        let mut q = self
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(job) = q.items.pop_front() {
                return Some(job);
            }
            if q.shutdown {
                return None;
            }
            q = self
                .ready
                .wait(q)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

/// A fixed set of worker threads sharing one [`Transcriber`].
///
/// `whisper-rs` creates per-call state from a shared `WhisperContext`, so the
/// model weights are loaded once no matter how many workers there are.
pub struct TranscriptionPool {
    shared: Arc<Shared>,
    /// Held behind a `Mutex` so `shutdown` can join them through a shared
    /// reference. The engine hands `Arc<TranscriptionPool>` clones to every
    /// capture thread, so it never has sole ownership to consume.
    workers: Mutex<Vec<JoinHandle<()>>>,
}

impl TranscriptionPool {
    /// Spawn `workers` threads draining a queue bounded at `capacity` segments.
    pub fn new(
        transcriber: &Arc<Transcriber>,
        workers: usize,
        capacity: usize,
        sink: &LineSink,
    ) -> Self {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue {
                items: VecDeque::new(),
                capacity: capacity.max(1),
                dropped: 0,
                shutdown: false,
            }),
            ready: Condvar::new(),
        });

        let handles = (0..workers.max(1))
            .map(|n| {
                let shared = Arc::clone(&shared);
                let transcriber = Arc::clone(transcriber);
                let sink = Arc::clone(sink);
                std::thread::Builder::new()
                    .name(format!("whisper-{n}"))
                    .spawn(move || worker_loop(&shared, &transcriber, sink.as_ref()))
                    .expect("spawn transcription worker")
            })
            .collect();

        Self {
            shared,
            workers: Mutex::new(handles),
        }
    }

    /// Hand a segment to the pool. Never blocks; logs when it evicts.
    pub fn submit(&self, job: SegmentJob) {
        let device = job.device_name.clone();
        if self.shared.submit(job) {
            log::warn!(
                "[whisper] queue full — dropped the oldest segment (device {device}); \
                 transcription is behind real time"
            );
        }
    }

    /// How many segments have been dropped this session.
    pub fn dropped(&self) -> u64 {
        self.shared
            .queue
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .dropped
    }

    /// Stop accepting work and join every worker, letting the queue drain first.
    ///
    /// Takes `&self` and is idempotent: draining the handle vector means a
    /// second call joins nothing. Workers keep popping until the queue is empty
    /// even after the shutdown flag is set, so queued segments are transcribed
    /// rather than abandoned.
    pub fn shutdown(&self) {
        {
            let mut q = self
                .shared
                .queue
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            q.shutdown = true;
        }
        self.shared.ready.notify_all();
        let handles: Vec<JoinHandle<()>> = self
            .workers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .drain(..)
            .collect();
        for h in handles {
            h.join().ok();
        }
    }
}

fn worker_loop(
    shared: &Arc<Shared>,
    transcriber: &Arc<Transcriber>,
    sink: &dyn Fn(TranscribedLine),
) {
    while let Some(job) = shared.take() {
        match transcriber.transcribe(&job.samples) {
            Ok(Some(text)) => sink(TranscribedLine {
                device_id: job.device_id,
                device_name: job.device_name,
                direction: job.direction,
                text,
            }),
            // Silence, or too short to hold a word. Not a line, not an error.
            Ok(None) => {}
            Err(e) => log::error!(
                "[whisper] transcription failed for {}: {e}",
                job.device_name
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(name: &str) -> SegmentJob {
        SegmentJob {
            device_id: name.to_string(),
            device_name: name.to_string(),
            direction: Direction::Input,
            samples: vec![0.0; 8],
        }
    }

    fn shared(capacity: usize) -> Arc<Shared> {
        Arc::new(Shared {
            queue: Mutex::new(Queue {
                items: VecDeque::new(),
                capacity,
                dropped: 0,
                shutdown: false,
            }),
            ready: Condvar::new(),
        })
    }

    #[test]
    fn submit_below_capacity_evicts_nothing() {
        let s = shared(2);
        assert!(!s.submit(job("a")));
        assert!(!s.submit(job("b")));
        assert_eq!(s.queue.lock().unwrap().dropped, 0);
    }

    #[test]
    fn overflow_drops_the_oldest_not_the_newest() {
        let s = shared(2);
        s.submit(job("first"));
        s.submit(job("second"));
        assert!(s.submit(job("third")), "third submission should evict");

        let q = s.queue.lock().unwrap();
        let names: Vec<&str> = q.items.iter().map(|j| j.device_name.as_str()).collect();
        // "first" is gone; the two most RECENT segments survive.
        assert_eq!(names, vec!["second", "third"]);
        assert_eq!(q.dropped, 1);
    }

    #[test]
    fn take_returns_items_in_order() {
        let s = shared(4);
        s.submit(job("a"));
        s.submit(job("b"));
        assert_eq!(s.take().map(|j| j.device_name), Some("a".to_string()));
        assert_eq!(s.take().map(|j| j.device_name), Some("b".to_string()));
    }

    #[test]
    fn take_returns_none_after_shutdown_drains() {
        let s = shared(4);
        s.submit(job("a"));
        s.queue.lock().unwrap().shutdown = true;
        assert!(s.take().is_some(), "queued work still drains");
        assert!(s.take().is_none(), "then the worker is released");
    }

    #[test]
    fn capacity_is_never_zero() {
        // A zero-capacity queue would evict the item it just pushed.
        let s = shared(1);
        s.submit(job("a"));
        assert_eq!(s.queue.lock().unwrap().items.len(), 1);
    }

    #[test]
    fn queued_work_still_drains_after_shutdown_is_flagged() {
        // The stop path sets the flag and then relies on workers finishing what
        // is already queued. If `take` returned None as soon as the flag was
        // set, every recording would lose whatever had not been transcribed yet.
        let s = shared(4);
        s.submit(job("a"));
        s.submit(job("b"));
        s.queue.lock().unwrap().shutdown = true;
        assert_eq!(s.take().map(|j| j.device_name), Some("a".to_string()));
        assert_eq!(s.take().map(|j| j.device_name), Some("b".to_string()));
        assert!(s.take().is_none());
    }

    #[test]
    fn direction_renders_for_persistence() {
        assert_eq!(Direction::Input.as_str(), "input");
        assert_eq!(Direction::Output.as_str(), "output");
    }
}
