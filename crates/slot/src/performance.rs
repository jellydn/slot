use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// Opt-in cumulative worker measurements. No file I/O or allocation on the worker.
#[derive(Default)]
pub struct Counters {
    pub enabled: AtomicBool,
    pub steps: AtomicU64,
    pub core_ns: AtomicU64,
    pub rewind_ns: AtomicU64,
    pub copy_ns: AtomicU64,
    pub audio_ns: AtomicU64,
}

impl Counters {
    pub fn read(&self) -> Totals {
        Totals {
            steps: self.steps.load(Ordering::Relaxed),
            core_ns: self.core_ns.load(Ordering::Relaxed),
            rewind_ns: self.rewind_ns.load(Ordering::Relaxed),
            copy_ns: self.copy_ns.load(Ordering::Relaxed),
            audio_ns: self.audio_ns.load(Ordering::Relaxed),
            ..Totals::default()
        }
    }
}

/// Audio faults count interleaved i16 samples, not callbacks or stereo frames.
#[derive(Clone, Copy, Default)]
pub struct Totals {
    pub steps: u64,
    pub core_ns: u64,
    pub rewind_ns: u64,
    pub copy_ns: u64,
    pub audio_ns: u64,
    pub swaps: u64,
    pub fresh: u64,
    pub missed: u64,
    pub late_available: bool,
    pub work_ns: u64,
    pub swap_ns: u64,
    pub starved: u64,
    pub dropped: u64,
}

/// Rates use wall time, not a presumed 60 Hz refresh or the core's advertised FPS.
pub fn lines(before: Totals, after: Totals, elapsed: Duration) -> [String; 6] {
    let seconds = elapsed.as_secs_f64().max(f64::EPSILON);
    let delta = |a: u64, b: u64| b.saturating_sub(a) as f64;
    let steps = delta(before.steps, after.steps);
    let swaps = delta(before.swaps, after.swaps);
    let per_step = |a, b| delta(a, b) / steps.max(1.0) / 1e6;
    let per_swap = |a, b| delta(a, b) / swaps.max(1.0) / 1e6;
    [
        format!(
            "Emu {:.1} fps | fresh {:.1} fps",
            steps / seconds,
            delta(before.fresh, after.fresh) / seconds
        ),
        format!(
            "Swaps {:.1} fps | late {}",
            swaps / seconds,
            if after.late_available {
                format!("{:.1}/s", delta(before.missed, after.missed) / seconds)
            } else {
                "n/a".into()
            }
        ),
        format!(
            "Core {:.2} ms | rewind {:.2} ms/step",
            per_step(before.core_ns, after.core_ns),
            per_step(before.rewind_ns, after.rewind_ns)
        ),
        format!(
            "Copy {:.2} ms | audio {:.2} ms/step",
            per_step(before.copy_ns, after.copy_ns),
            per_step(before.audio_ns, after.audio_ns)
        ),
        format!(
            "Work {:.2} ms | swap {:.2} ms/swap",
            per_swap(before.work_ns, after.work_ns),
            per_swap(before.swap_ns, after.swap_ns)
        ),
        format!(
            "Audio gaps {:.0} | drop {:.0} samples/s",
            delta(before.starved, after.starved) / seconds,
            delta(before.dropped, after.dropped) / seconds
        ),
    ]
}
