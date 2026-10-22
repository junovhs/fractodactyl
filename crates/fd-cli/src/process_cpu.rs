//! Process CPU time, including all threads but excluding child processes.
//! Unsupported/failed clocks stay absent, rather than looking like idle CPUs.
use std::time::Duration;

#[cfg(unix)]
pub(crate) fn now() -> Option<Duration> {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime writes one initialized timespec to this valid pointer.
    let result = unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut ts) };
    if result != 0 || ts.tv_sec < 0 || !(0..1_000_000_000).contains(&ts.tv_nsec) {
        return None;
    }
    Some(Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32))
}

#[cfg(not(unix))]
pub(crate) fn now() -> Option<Duration> {
    None
}

pub(crate) fn elapsed(start: Option<Duration>) -> Option<f64> {
    now()?.checked_sub(start?).map(|d| d.as_secs_f64())
}
