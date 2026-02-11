// perf_timer.rs — Performance timing
//
// Port of: PerfTimer.h, PerfTimer.cpp
// Uses std::time::Instant for cross-platform high-resolution timing
// (Instant uses QueryPerformanceCounter internally on Windows).

use std::time::Instant;





/// High-resolution performance timer.
/// Port of: CPerfTimer
///
/// Uses `std::time::Instant` which delegates to `QueryPerformanceCounter` on Windows,
/// so this is functionally equivalent to the C++ original.
pub struct PerfTimer {
    start: Option<Instant>,
    stop:  Option<Instant>,
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl Default for PerfTimer
//
//  Default constructor — delegates to new().
//
////////////////////////////////////////////////////////////////////////////////

impl Default for PerfTimer {
    fn default() -> Self {
        Self::new()
    }
}





////////////////////////////////////////////////////////////////////////////////
//
//  impl PerfTimer
//
//  Performance timer for elapsed-time measurement.
//
////////////////////////////////////////////////////////////////////////////////

impl PerfTimer {
    ////////////////////////////////////////////////////////////////////////////
    //
    //  new
    //
    //  Create a new PerfTimer in the stopped state.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn new() -> Self {
        PerfTimer {
            start: None,
            stop:  None,
        }
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  start
    //
    //  Start (or restart) the timer.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn start(&mut self) {
        self.start = Some(Instant::now());
        self.stop  = None;
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  stop
    //
    //  Stop the timer.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn stop(&mut self) {
        self.stop = Some(Instant::now());
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  elapsed_ms
    //
    //  Get elapsed time in milliseconds with fractional precision.
    //  Returns 0.0 if not started.
    //
    ////////////////////////////////////////////////////////////////////////////

    pub fn elapsed_ms(&self) -> f64 {
        match (self.start, self.stop) {
            (Some(s), Some(e)) => e.duration_since(s).as_secs_f64() * 1000.0,
            (Some(s), None)    => s.elapsed().as_secs_f64() * 1000.0,
            _                  => 0.0,
        }
    }
}





#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    ////////////////////////////////////////////////////////////////////////////
    //
    //  timer_measures_elapsed
    //
    //  Verify the timer measures at least ~50ms of elapsed time.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn timer_measures_elapsed() {
        let mut t = PerfTimer::new();
        t.start();
        thread::sleep(Duration::from_millis(50));
        t.stop();
        let ms = t.elapsed_ms();
        // Should be at least 40ms (allowing for scheduler variance)
        assert!(ms >= 40.0, "elapsed_ms was {}", ms);
        // Should be less than 500ms (generous upper bound)
        assert!(ms < 500.0, "elapsed_ms was {}", ms);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  timer_not_started_returns_zero
    //
    //  An unstarted timer should return 0.0.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn timer_not_started_returns_zero() {
        let t = PerfTimer::new();
        assert_eq!(t.elapsed_ms(), 0.0);
    }





    ////////////////////////////////////////////////////////////////////////////
    //
    //  timer_still_running_returns_positive
    //
    //  A running timer should return a positive elapsed time.
    //
    ////////////////////////////////////////////////////////////////////////////

    #[test]
    fn timer_still_running_returns_positive() {
        let mut t = PerfTimer::new();
        t.start();
        thread::sleep(Duration::from_millis(10));
        let ms = t.elapsed_ms();
        assert!(ms > 0.0);
    }
}
