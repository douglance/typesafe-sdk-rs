//! Where log lines go.
//!
//! Filtering happens in the wrapper rather than at each call site, so a sink
//! can never be handed a line it was configured not to want. The message is
//! built behind a closure for the same reason: at the default level, formatting
//! a debug line is work nobody asked for.

use std::sync::{Arc, Mutex};

use crate::level::Level;

/// Somewhere log lines are written.
///
/// Implementations must tolerate concurrent calls: one client may have several
/// requests in flight, each numbering its own lines.
pub trait Logger: Send + Sync {
    /// Writes one line, already filtered.
    fn log(&self, level: Level, message: &str);
}

/// Discards everything. The default when a caller wants no output.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl Logger for Silent {
    fn log(&self, _level: Level, _message: &str) {}
}

/// Writes to standard error, prefixed so the source is obvious in a mixed log.
#[derive(Debug, Clone, Copy, Default)]
pub struct Console;

impl Logger for Console {
    fn log(&self, level: Level, message: &str) {
        eprintln!("[typesafe-sdk] {level} {message}");
    }
}

/// A logger that drops anything below its threshold.
///
/// Filtering happens here rather than at each call site so a sink cannot be
/// handed a line it was configured not to want.
pub struct Filtered {
    inner: Arc<dyn Logger>,
    threshold: Level,
}

impl Filtered {
    /// Wraps `inner`, admitting only messages at or above `threshold`.
    #[must_use]
    pub fn new(inner: Arc<dyn Logger>, threshold: Level) -> Self {
        Self { inner, threshold }
    }

    /// The level below which messages are dropped.
    #[must_use]
    pub const fn threshold(&self) -> Level {
        self.threshold
    }

    /// Writes `message` if `level` passes the threshold.
    pub fn log(&self, level: Level, message: impl FnOnce() -> String) {
        if level.passes(self.threshold) {
            self.inner.log(level, &message());
        }
    }
}

impl std::fmt::Debug for Filtered {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filtered")
            .field("threshold", &self.threshold)
            .finish_non_exhaustive()
    }
}

/// One recorded line: the level it was written at, and its text.
pub type Line = (Level, String);

/// Collects lines in memory, for tests.
#[derive(Debug, Default)]
pub struct Recording {
    lines: Mutex<Vec<Line>>,
}

impl Recording {
    /// Every line written so far.
    ///
    /// # Panics
    /// Panics only if a previous writer panicked while holding the lock.
    #[must_use]
    pub fn lines(&self) -> Vec<Line> {
        self.lines
            .lock()
            .map_or_else(|e| e.into_inner().clone(), |l| l.clone())
    }
}

impl Logger for Recording {
    fn log(&self, level: Level, message: &str) {
        if let Ok(mut lines) = self.lines.lock() {
            lines.push((level, message.to_owned()));
        }
    }
}
