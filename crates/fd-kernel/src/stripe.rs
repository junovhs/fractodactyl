//! Stripe average (STYL-09): the mean of `0.5 + 0.5 sin(k arg z_i)` over the last `K`
//! iterates before escape, smooth-interpolated with the escape fraction between the
//! window ending at `z_n` and the one ending at `z_{n-1}`. A short window is the look,
//! not an approximation: the full-orbit average is flat grey, the last ~16 iterates give
//! the combed fibre texture (probe in STYL-09).
//!
//! The kernel records each iterate it lands on. BLA blocks skip iterates, so the kernel
//! also snapshots its state at every block start; when an escape finds its window
//! incomplete it rewinds to a snapshot before the window and replays plainly
//! ([`Trace`] is a no-op unless the column is requested).

/// Ring capacity: windows are at most `MAX_WINDOW`, plus one iterate for interpolation.
const RING: usize = 128;
/// Block-start snapshots kept per sample.
const SNAPS: usize = 8;

/// Largest window.
pub const MAX_WINDOW: u32 = 64;

/// Stripe-average settings of a render.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StripeParams {
    /// Iterates averaged before escape, `1..=MAX_WINDOW`.
    pub window: u32,
    /// Stripe frequency `k` in `sin(k arg z)`.
    pub freq: f64,
}

impl Default for StripeParams {
    fn default() -> Self {
        StripeParams {
            window: 16,
            freq: 5.0,
        }
    }
}

/// Loop state at the start of a BLA block (the replay restarts here).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Snap {
    pub(crate) m: usize,
    pub(crate) n: u64,
    pub(crate) xr: f64,
    pub(crate) xi: f64,
    pub(crate) delta_exp: i64,
    pub(crate) scale: f64,
    pub(crate) dc_r: f64,
    pub(crate) dc_i: f64,
    /// First iterate of the contiguous recorded run ending at `n` (set by the recorder).
    pub(crate) valid: u64,
}

/// What the perturbation loop reports to a stripe recorder.
pub(crate) trait Trace {
    const ON: bool;
    /// Iteration starts after iterate `n` (0, or a series start).
    fn start(&mut self, n: u64);
    /// Iterate `z_n = (zr, zi)`; iterates arrive in order, gaps only after blocks.
    fn record(&mut self, n: u64, zr: f64, zi: f64);
    /// A block starts from `s` and lands on iterate `landed`.
    fn snap(&mut self, s: Snap, landed: u64);
    /// Escaped at `z_n`: a snapshot to replay from if the window misses iterates.
    fn rewind(&mut self, n: u64) -> Option<Snap>;
    /// Escaped at `z_n = (zr, zi)` with squared escape radius `r2`: compute the value.
    fn finish(&mut self, n: u64, zr: f64, zi: f64, r2: f64);
    /// A new sample starts.
    fn begin(&mut self);
    /// The finished sample's value (0 unless it escaped).
    fn value(&self) -> f32;
}

/// Whether `t` records (a constant per instantiation, so disabled hooks compile away).
#[inline(always)]
pub(crate) fn on<T: Trace>(_: &T) -> bool {
    T::ON
}

/// No stripe column: every hook compiles away.
pub(crate) struct NoTrace;

impl Trace for NoTrace {
    const ON: bool = false;
    #[inline(always)]
    fn start(&mut self, _: u64) {}
    #[inline(always)]
    fn record(&mut self, _: u64, _: f64, _: f64) {}
    #[inline(always)]
    fn snap(&mut self, _: Snap, _: u64) {}
    #[inline(always)]
    fn rewind(&mut self, _: u64) -> Option<Snap> {
        None
    }
    #[inline(always)]
    fn finish(&mut self, _: u64, _: f64, _: f64, _: f64) {}
    #[inline(always)]
    fn begin(&mut self) {}
    #[inline(always)]
    fn value(&self) -> f32 {
        0.0
    }
}

/// Per-thread recorder, reset per sample with [`Trace::begin`].
pub(crate) struct Stripe {
    p: StripeParams,
    /// `z_n` at slot `n % RING`; valid for `valid <= n` up to the last recorded iterate
    /// (iterates arrive in order, so a slot is overwritten only `RING` iterates later).
    ring: [(f64, f64); RING],
    valid: u64,
    snaps: [Snap; SNAPS],
    count: usize,
    value: f32,
}

impl Stripe {
    pub(crate) fn new(p: StripeParams) -> Stripe {
        Stripe {
            p,
            ring: [(0.0, 0.0); RING],
            valid: u64::MAX,
            snaps: [Snap::default(); SNAPS],
            count: 0,
            value: 0.0,
        }
    }

    /// `z_i`, if recorded for this sample.
    #[inline]
    fn z(&self, i: u64) -> Option<(f64, f64)> {
        (i >= self.valid).then(|| self.ring[i as usize % RING])
    }

    /// `0.5 + 0.5 sin(k arg z)`; for a whole `k` up to 16 as `sign(k) Im (z / |z|)^|k|`,
    /// without `atan2`.
    fn term(&self, (zr, zi): (f64, f64)) -> f64 {
        let k = self.p.freq;
        let s = if k.fract() == 0.0 && k.abs() <= 16.0 {
            let h = zr.hypot(zi);
            let (ur, ui) = (zr / h, zi / h);
            let (mut pr, mut pi) = (1.0, 0.0);
            for _ in 0..k.abs() as u32 {
                (pr, pi) = (pr * ur - pi * ui, pr * ui + pi * ur);
            }
            if k < 0.0 {
                -pi
            } else {
                pi
            }
        } else {
            (k * zi.atan2(zr)).sin()
        };
        0.5 + 0.5 * s
    }

    /// Whether every iterate of both windows ending at `z_n` is recorded.
    fn complete(&self, n: u64) -> bool {
        self.valid <= n.saturating_sub(u64::from(self.p.window)).max(1)
    }
}

impl Trace for Stripe {
    const ON: bool = true;
    fn begin(&mut self) {
        self.valid = u64::MAX;
        self.count = 0;
        self.value = 0.0;
    }

    fn value(&self) -> f32 {
        self.value
    }

    fn start(&mut self, n: u64) {
        self.valid = n + 1;
    }

    #[inline(always)]
    fn record(&mut self, n: u64, zr: f64, zi: f64) {
        self.ring[n as usize % RING] = (zr, zi);
    }

    fn snap(&mut self, s: Snap, landed: u64) {
        self.snaps[self.count % SNAPS] = Snap {
            valid: self.valid,
            ..s
        };
        self.count += 1;
        self.valid = landed;
    }

    fn rewind(&mut self, n: u64) -> Option<Snap> {
        if self.count == 0 || self.complete(n) {
            return None;
        }
        // The latest block start at or before the iterate preceding both windows;
        // failing that, the oldest one kept (more replay, never less history).
        let need = n.saturating_sub(u64::from(self.p.window));
        let kept = self.count.min(SNAPS);
        let all = (0..kept).map(|j| self.snaps[(self.count - 1 - j) % SNAPS]);
        let s = all
            .clone()
            .find(|s| s.n <= need)
            .or_else(|| all.min_by_key(|s| s.n))?;
        // The replay records from s.n + 1 on, continuing the run that reached s.n.
        self.valid = s.valid;
        Some(s)
    }

    fn finish(&mut self, n: u64, zr: f64, zi: f64, r2: f64) {
        // Terms of z_{n-K} ..= z_n, each computed once: the window ending at z_n takes
        // all but the first, the one ending at z_{n-1} all but the last.
        let first = n.saturating_sub(u64::from(self.p.window)).max(1);
        let (mut now, mut before, mut count_now, mut count_before) = (0.0, 0.0, 0u32, 0u32);
        for i in first..=n {
            if let Some(z) = self.z(i) {
                let t = self.term(z);
                if i > first || first == 1 && n <= u64::from(self.p.window) {
                    now += t;
                    count_now += 1;
                }
                if i < n {
                    before += t;
                    count_before += 1;
                }
            }
        }
        if count_now == 0 {
            self.value = 0.0;
            return;
        }
        now /= f64::from(count_now);
        before = if count_before > 0 {
            before / f64::from(count_before)
        } else {
            now
        };
        // How far past the escape radius z_n landed: 0 just past it, 1 at its square.
        let frac = (1.0 - ((zr * zr + zi * zi).ln() / r2.ln()).log2()).clamp(0.0, 1.0);
        self.value = (before * (1.0 - frac) + now * frac) as f32;
    }
}
