//! Hold-to-reset gesture for the counter page.
//!
//! Pressing and holding the counter fills the page with a white wash that grows
//! outward from the finger, starting only after [`SHOW_AFTER`] so quick taps
//! show nothing. When the hold reaches [`THRESHOLD`] the count resets,
//! the wash is full, and it then fades back to the plain background. Lifting the
//! finger before the threshold, or moving it past [`SLOP_PX`], is not a reset.

use std::time::{Duration, Instant};

/// How long a finger must rest on the counter before it resets.
pub const THRESHOLD: Duration = Duration::from_millis(600);
/// How long the finger must rest before the wash first appears. The wash then
/// grows from nothing and still fills the page exactly at [`THRESHOLD`].
pub const SHOW_AFTER: Duration = Duration::from_millis(200);
/// Finger travel, in pixels, past which a press is a drag rather than a hold or tap.
pub const SLOP_PX: f32 = 24.0;
/// How long the page stays fully washed after the reset, before fading.
const FULL_SECS: f32 = 0.12;
/// How long the full wash takes to fade away.
const FADE_SECS: f32 = 1.0;
/// Softness of the wash's leading edge, in pixels.
const FEATHER_PX: f32 = 48.0;

/// The wash as it should be drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wash {
    /// Point the wash grows from, in window pixels with a top-left origin.
    pub centre: [f32; 2],
    /// 0 is nothing, 1 reaches the farthest corner of the page.
    pub extent: f32,
    /// Opacity, 0..=1.
    pub alpha: f32,
}

#[derive(Default)]
pub struct Hold {
    origin: [f32; 2],
    /// Start of the current press, while it is still down and has not moved.
    pressed: Option<Instant>,
    /// When the threshold was reached for the current press.
    fired: Option<Instant>,
}

impl Hold {
    /// Starts a press on the counter at `(x, y)`.
    pub fn press(&mut self, now: Instant, x: f32, y: f32) {
        self.origin = [x, y];
        self.pressed = Some(now);
        self.fired = None;
    }

    /// Tracks the finger. Cancels the hold once it has moved past the slop.
    pub fn moved(&mut self, x: f32, y: f32) {
        let [ox, oy] = self.origin;
        if (x - ox).abs() > SLOP_PX || (y - oy).abs() > SLOP_PX {
            self.pressed = None;
        }
    }

    /// Stops tracking the press without treating it as a tap.
    pub fn cancel(&mut self) {
        self.pressed = None;
    }

    /// True on the single call where the hold reaches the threshold. The caller
    /// resets the counter on that call.
    pub fn poll(&mut self, now: Instant) -> bool {
        match self.pressed {
            Some(start) if self.fired.is_none() && now.duration_since(start) >= THRESHOLD => {
                self.fired = Some(now);
                true
            }
            _ => false,
        }
    }

    /// Ends the press. Returns true if it was a tap: still tracked and never
    /// reached the threshold.
    pub fn release(&mut self) -> bool {
        let fired = self.fired.is_some();
        self.pressed = None;
        fired
    }

    /// Whether anything is in progress, so the frame loop keeps running.
    pub fn active(&self, now: Instant) -> bool {
        (self.pressed.is_some() && self.fired.is_none()) || self.wash(now).is_some()
    }

    /// The wash for this frame, or `None` when the page is plain.
    pub fn wash(&self, now: Instant) -> Option<Wash> {
        if let Some(fired) = self.fired {
            let since = now.duration_since(fired).as_secs_f32() - FULL_SECS;
            if since >= FADE_SECS {
                return None;
            }
            let fade = (since / FADE_SECS).max(0.0);
            return Some(Wash {
                centre: self.origin,
                extent: 1.0,
                alpha: 1.0 - smoothstep(fade),
            });
        }

        let start = self.pressed?;
        let grow = (THRESHOLD - SHOW_AFTER).as_secs_f32();
        let progress = ((now.duration_since(start).as_secs_f32() - SHOW_AFTER.as_secs_f32())
            / grow)
            .clamp(0.0, 1.0);
        (progress > 0.0).then_some(Wash {
            centre: self.origin,
            extent: progress,
            alpha: 1.0,
        })
    }
}

/// Distance from `centre` to the farthest corner of a `width` x `height` page.
/// A wash with this radius covers the whole page.
pub fn reach(centre: [f32; 2], width: f32, height: f32) -> f32 {
    [[0.0, 0.0], [width, 0.0], [0.0, height], [width, height]]
        .iter()
        .map(|c| ((c[0] - centre[0]).powi(2) + (c[1] - centre[1]).powi(2)).sqrt())
        .fold(0.0, f32::max)
}

/// Radial mask for the wash as `[cx, cy, radius, feather]`, sized so that at
/// full extent the mask covers every corner of a `width` x `height` page.
pub fn disc(wash: Wash, width: f32, height: f32) -> [f32; 4] {
    let radius = wash.extent * (reach(wash.centre, width, height) + FEATHER_PX);
    let feather = FEATHER_PX.min(radius).max(1.0);
    [wash.centre[0], wash.centre[1], radius, feather]
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn quick_release_is_a_tap() {
        let t0 = Instant::now();
        let mut hold = Hold::default();
        hold.press(t0, 100.0, 100.0);
        assert!(!hold.poll(t0 + ms(100)));
        assert!(!hold.release(), "a tap never fires the hold");
    }

    #[test]
    fn holding_past_the_threshold_fires_once_and_is_not_a_tap() {
        let t0 = Instant::now();
        let mut hold = Hold::default();
        hold.press(t0, 100.0, 100.0);
        assert!(hold.poll(t0 + THRESHOLD));
        assert!(!hold.poll(t0 + THRESHOLD + ms(50)));
        assert!(hold.release(), "a fired hold reports true on release");
    }

    #[test]
    fn moving_past_the_slop_cancels_the_hold() {
        let t0 = Instant::now();
        let mut hold = Hold::default();
        hold.press(t0, 100.0, 100.0);
        hold.moved(100.0, 150.0);
        assert!(!hold.poll(t0 + THRESHOLD + ms(10)));
        assert!(!hold.release());
    }

    #[test]
    fn small_jitter_keeps_the_hold() {
        let t0 = Instant::now();
        let mut hold = Hold::default();
        hold.press(t0, 100.0, 100.0);
        hold.moved(105.0, 110.0);
        assert!(hold.poll(t0 + THRESHOLD));
    }

    #[test]
    fn wash_grows_from_the_press_point_while_held() {
        let t0 = Instant::now();
        let mut hold = Hold::default();
        hold.press(t0, 40.0, 60.0);
        assert!(hold.wash(t0).is_none());
        assert!(hold.wash(t0 + SHOW_AFTER).is_none(), "nothing shows before the delay");
        let mid = SHOW_AFTER + (THRESHOLD - SHOW_AFTER) / 2;
        let half = hold.wash(t0 + mid).unwrap();
        assert!((half.extent - 0.5).abs() < 1e-3);
        assert_eq!(half.alpha, 1.0);
        assert_eq!(half.centre, [40.0, 60.0]);
    }

    #[test]
    fn threshold_fills_the_page_then_fades_out() {
        let t0 = Instant::now();
        let mut hold = Hold::default();
        hold.press(t0, 100.0, 100.0);
        let fired = t0 + THRESHOLD;
        hold.poll(fired);

        let full = hold.wash(fired).unwrap();
        assert!((full.extent - 1.0).abs() < 1e-3);
        assert_eq!(full.alpha, 1.0);

        let mid_fade = hold.wash(fired + ms(620)).unwrap();
        assert!(mid_fade.alpha > 0.0 && mid_fade.alpha < 1.0);

        hold.release();
        assert!(hold.wash(fired + ms(1300)).is_none());
    }

    #[test]
    fn reach_covers_the_farthest_corner() {
        assert!((reach([0.0, 0.0], 3.0, 4.0) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn full_wash_mask_covers_every_corner() {
        let wash = Wash {
            centre: [10.0, 10.0],
            extent: 1.0,
            alpha: 1.0,
        };
        let [cx, cy, radius, feather] = disc(wash, 300.0, 200.0);
        let far = reach([cx, cy], 300.0, 200.0);
        assert!(radius - feather >= far - 1e-3);
    }
}
