//! Frame timing: the delta time each frame simulates, and frame-rate statistics.

use std::time::{Duration, Instant};

/// The longest delta time a frame simulates. A longer gap (a stall, a debugger break, a
/// window drag on some platforms) is cut to this, so the game doesn't jump ahead.
pub const MAX_DELTA: Duration = Duration::from_millis(250);

/// The span that [`FrameStats`] cover.
pub const STATS_WINDOW: Duration = Duration::from_micros(STATS_WINDOW_US);

/// [`STATS_WINDOW`] in microseconds, the unit of the samples.
const STATS_WINDOW_US: u64 = 1_000_000;

/// How many frame times the ring buffer holds. Above this many frames per second, the
/// statistics cover the last `SAMPLES` frames, a little less than [`STATS_WINDOW`].
const SAMPLES: usize = 8192;

/// Frame-rate statistics over the last [`STATS_WINDOW`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    /// Frames per second.
    pub fps: f32,
    /// Mean frame time.
    pub avg: Duration,
    /// Longest frame time.
    pub max: Duration,
}

/// Measures the time between frames.
///
/// Call [`tick`](Self::tick) once per frame. Frame times are kept in a fixed-size ring
/// buffer, allocated once in [`new`](Self::new), so ticking never allocates.
pub struct FrameTimer {
    /// When the previous frame ticked; `None` before the first tick and after a pause.
    last: Option<Instant>,
    /// Frame times in microseconds, oldest at `head`.
    samples: Box<[u32]>,
    head: usize,
    len: usize,
    /// Sum of the `len` samples, in microseconds.
    sum_us: u64,
}

impl FrameTimer {
    /// A timer with no frames yet.
    pub fn new() -> Self {
        Self {
            last: None,
            samples: vec![0; SAMPLES].into_boxed_slice(),
            head: 0,
            len: 0,
            sum_us: 0,
        }
    }

    /// Starts a frame at `now` and returns the time to simulate: the time since the
    /// previous tick, at most [`MAX_DELTA`]. The first tick, and the first after
    /// [`pause`](Self::pause), returns zero and records no frame time.
    ///
    /// The statistics record the real, unclamped frame time, so a stall shows up there.
    pub fn tick(&mut self, now: Instant) -> Duration {
        let Some(last) = self.last.replace(now) else {
            return Duration::ZERO;
        };
        let elapsed = now.saturating_duration_since(last);
        self.record(u32::try_from(elapsed.as_micros()).unwrap_or(u32::MAX));
        elapsed.min(MAX_DELTA)
    }

    /// Forgets when the last frame ticked, so the gap until the next tick isn't counted
    /// as a frame. For when the loop stops drawing on purpose (a hidden window).
    pub fn pause(&mut self) {
        self.last = None;
    }

    /// Statistics over the last [`STATS_WINDOW`], or `None` before the first frame time.
    pub fn stats(&self) -> Option<FrameStats> {
        if self.len == 0 {
            return None;
        }
        let max_us = (0..self.len)
            .map(|i| self.samples[(self.head + i) % SAMPLES])
            .max()
            .unwrap_or(0);
        let count = self.len as u64;
        let fps = if self.sum_us == 0 {
            0.0
        } else {
            count as f32 * 1e6 / self.sum_us as f32
        };
        Some(FrameStats {
            fps,
            avg: Duration::from_micros(self.sum_us / count),
            max: Duration::from_micros(u64::from(max_us)),
        })
    }

    /// Adds a frame time, then drops the oldest ones until the rest fit in
    /// [`STATS_WINDOW`] (always keeping the newest).
    fn record(&mut self, us: u32) {
        if self.len == SAMPLES {
            self.pop_oldest();
        }
        self.samples[(self.head + self.len) % SAMPLES] = us;
        self.len += 1;
        self.sum_us += u64::from(us);
        while self.len > 1 && self.sum_us > STATS_WINDOW_US {
            self.pop_oldest();
        }
    }

    fn pop_oldest(&mut self) {
        self.sum_us -= u64::from(self.samples[self.head]);
        self.head = (self.head + 1) % SAMPLES;
        self.len -= 1;
    }
}

impl Default for FrameTimer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{FrameTimer, MAX_DELTA, SAMPLES};

    /// Ticks `timer` `frames` times, `step` apart, starting one `step` after `start`.
    /// Returns the time of the last tick.
    fn run(timer: &mut FrameTimer, start: Instant, step: Duration, frames: u32) -> Instant {
        let mut now = start;
        for _ in 0..frames {
            now += step;
            timer.tick(now);
        }
        now
    }

    #[test]
    fn first_tick_is_zero_and_records_nothing() {
        let mut timer = FrameTimer::new();
        assert_eq!(timer.tick(Instant::now()), Duration::ZERO);
        assert_eq!(timer.stats(), None);
    }

    #[test]
    fn delta_is_time_since_previous_tick() {
        let mut timer = FrameTimer::new();
        let start = Instant::now();
        timer.tick(start);
        assert_eq!(
            timer.tick(start + Duration::from_millis(16)),
            Duration::from_millis(16)
        );
    }

    #[test]
    fn steady_60_fps() {
        let mut timer = FrameTimer::new();
        let start = Instant::now();
        timer.tick(start);
        run(&mut timer, start, Duration::from_micros(16_667), 120);
        let stats = timer.stats().expect("frames were recorded");
        assert!((stats.fps - 60.0).abs() < 0.01, "fps {}", stats.fps);
        assert_eq!(stats.avg, Duration::from_micros(16_667));
        assert_eq!(stats.max, Duration::from_micros(16_667));
    }

    #[test]
    fn stats_cover_only_the_last_second() {
        let mut timer = FrameTimer::new();
        let start = Instant::now();
        timer.tick(start);
        // A slow second (10 fps, one 300 ms hitch), then a fast one (100 fps).
        let now = run(&mut timer, start, Duration::from_millis(100), 7);
        let now = run(&mut timer, now, Duration::from_millis(300), 1);
        run(&mut timer, now, Duration::from_millis(10), 100);
        let stats = timer.stats().expect("frames were recorded");
        assert!((stats.fps - 100.0).abs() < 0.01, "fps {}", stats.fps);
        assert_eq!(stats.avg, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(10));
    }

    #[test]
    fn mixed_frame_times_give_mean_and_max() {
        let mut timer = FrameTimer::new();
        let start = Instant::now();
        timer.tick(start);
        let now = run(&mut timer, start, Duration::from_millis(10), 3);
        run(&mut timer, now, Duration::from_millis(30), 1);
        let stats = timer.stats().expect("frames were recorded");
        // 4 frames in 60 ms.
        assert!((stats.fps - 66.667).abs() < 0.01, "fps {}", stats.fps);
        assert_eq!(stats.avg, Duration::from_millis(15));
        assert_eq!(stats.max, Duration::from_millis(30));
    }

    #[test]
    fn a_stall_is_clamped_for_simulation_but_not_in_stats() {
        let mut timer = FrameTimer::new();
        let start = Instant::now();
        timer.tick(start);
        assert_eq!(timer.tick(start + Duration::from_secs(3)), MAX_DELTA);
        let stats = timer.stats().expect("frames were recorded");
        // Longer than the window, but the newest frame is always kept.
        assert_eq!(stats.max, Duration::from_secs(3));
        assert!((stats.fps - 1.0 / 3.0).abs() < 0.001, "fps {}", stats.fps);
    }

    #[test]
    fn pause_skips_the_gap() {
        let mut timer = FrameTimer::new();
        let start = Instant::now();
        timer.tick(start);
        let now = run(&mut timer, start, Duration::from_millis(10), 5);
        timer.pause();
        let resumed = now + Duration::from_secs(5);
        assert_eq!(timer.tick(resumed), Duration::ZERO);
        assert_eq!(
            timer.tick(resumed + Duration::from_millis(10)),
            Duration::from_millis(10)
        );
        let stats = timer.stats().expect("frames were recorded");
        assert_eq!(stats.max, Duration::from_millis(10));
    }

    #[test]
    fn more_frames_than_samples_keeps_the_newest() {
        let mut timer = FrameTimer::new();
        let start = Instant::now();
        timer.tick(start);
        // 50 µs frames: 20 000 fps, more than the buffer holds in a second.
        let now = run(&mut timer, start, Duration::from_micros(50), 20_000);
        run(&mut timer, now, Duration::from_micros(60), 1);
        let stats = timer.stats().expect("frames were recorded");
        assert_eq!(stats.max, Duration::from_micros(60));
        let expected = SAMPLES as f32 * 1e6 / ((SAMPLES as f32 - 1.0) * 50.0 + 60.0);
        assert!((stats.fps - expected).abs() < 1.0, "fps {}", stats.fps);
    }
}
