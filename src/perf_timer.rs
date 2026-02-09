// perf_timer.rs — Performance timing using QueryPerformanceCounter
//
// Port of: PerfTimer.h, PerfTimer.cpp
//
// Stub — full implementation in US-13.

/// High-resolution performance timer using Windows QueryPerformanceCounter.
/// Port of: CPerfTimer
pub struct PerfTimer {
    pub start_count:     i64,
    pub stop_count:      i64,
    pub frequency:       i64,
}

impl Default for PerfTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl PerfTimer {
    pub fn new() -> Self {
        PerfTimer {
            start_count: 0,
            stop_count:  0,
            frequency:   0,
        }
    }

    /// Start the timer. Stub — full implementation in US-13.
    pub fn start(&mut self) {
        // Will call QueryPerformanceCounter
    }

    /// Stop the timer. Stub — full implementation in US-13.
    pub fn stop(&mut self) {
        // Will call QueryPerformanceCounter
    }

    /// Get elapsed time in seconds. Stub — full implementation in US-13.
    pub fn elapsed_seconds(&self) -> f64 {
        0.0
    }
}
