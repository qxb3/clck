//! Three-page horizontal pager: page -1 is the counter, page 0 the clock,
//! page 1 the calendar. Swiping left (finger moves left) advances to the
//! right (clock -> calendar); swiping right goes back (clock -> counter).

use std::time::Instant;

use crate::anim::ease_out_cubic;

const SETTLE_SECS: f32 = 0.42;
/// Fraction of the width a drag must cover to flip the page.
const FLIP_FRACTION: f32 = 0.16;

/// Leftmost and rightmost page indices the pager can rest on.
const MIN_PAGE: f32 = -1.0;
const MAX_PAGE: f32 = 1.0;

#[derive(Default)]
pub struct Pager {
    pos: f32,
    drag: Option<(f32, f32)>,
    /// Furthest the finger has travelled from its start, for tap detection.
    drag_extent: f32,
    settle: Option<(f32, f32, Instant)>,
}

impl Pager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Continuous page position: -1 = counter, 0 = clock, 1 = calendar.
    pub fn pos(&self) -> f32 {
        self.pos
    }

    pub fn start_drag(&mut self, x: f32) {
        self.drag = Some((x, self.pos));
        self.drag_extent = 0.0;
        self.settle = None;
    }

    pub fn drag_to(&mut self, x: f32, width: f32) {
        if let Some((start_x, start_pos)) = self.drag {
            self.drag_extent = self.drag_extent.max((x - start_x).abs());
            if width > 0.0 {
                // Fingers move content with them, so dragging left advances.
                self.pos = (start_pos - (x - start_x) / width).clamp(MIN_PAGE, MAX_PAGE);
            }
        }
    }

    pub fn end_drag(&mut self, x: f32, width: f32, now: Instant) {
        let Some((start_x, start_pos)) = self.drag.take() else {
            return;
        };
        let dx = x - start_x;
        let to = if dx.abs() > width * FLIP_FRACTION {
            if dx < 0.0 {
                (start_pos.round() + 1.0).min(MAX_PAGE)
            } else {
                (start_pos.round() - 1.0).max(MIN_PAGE)
            }
        } else {
            start_pos.round().clamp(MIN_PAGE, MAX_PAGE)
        };
        self.settle = Some((self.pos, to, now));
    }

    pub fn cancel_drag(&mut self, now: Instant) {
        if self.drag.take().is_some() {
            self.settle = Some((self.pos, self.pos.round().clamp(MIN_PAGE, MAX_PAGE), now));
        }
    }

    /// Whether the gesture that just ended (or is ending) travelled far
    /// enough to count as a swipe rather than a tap. Checked against the
    /// same threshold used to decide whether a drag flips the page, so a
    /// "tap" and a "swipe" can never both be true for the same gesture.
    pub fn was_swipe(&self, width: f32) -> bool {
        self.drag_extent > width * FLIP_FRACTION
    }

    pub fn animating(&self) -> bool {
        self.settle.is_some()
    }

    /// True once the pager is at rest exactly on `page` (not mid-swipe or
    /// mid-settle).
    pub fn is_settled_on(&self, page: f32) -> bool {
        self.settle.is_none() && self.drag.is_none() && (self.pos - page).abs() < 1e-4
    }

    /// Advances the settle animation; returns true while a frame is needed.
    pub fn update(&mut self, now: Instant) -> bool {
        let Some((from, to, started)) = self.settle else {
            return false;
        };
        let t = now.duration_since(started).as_secs_f32() / SETTLE_SECS;
        if t >= 1.0 {
            self.pos = to.clamp(MIN_PAGE, MAX_PAGE);
            self.settle = None;
        } else {
            self.pos = from + (to - from) * ease_out_cubic(t);
        }
        true
    }

    #[cfg(test)]
    pub fn settle_to(&mut self, page: f32, now: Instant) {
        self.settle = Some((self.pos, page, now));
    }
}

/// Horizontal offset of `page` in pixels for the current pager position.
pub fn page_offset(page: f32, pos: f32, width: f32) -> f32 {
    (page - pos) * width
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const W: f32 = 1000.0;

    fn settled(now: Instant) -> Instant {
        now + Duration::from_secs_f32(SETTLE_SECS + 0.01)
    }

    #[test]
    fn starts_on_the_clock_with_the_calendar_to_its_right_and_counter_to_its_left() {
        let pager = Pager::new();
        assert_eq!(pager.pos(), 0.0);
        assert_eq!(page_offset(-1.0, pager.pos(), W), -W);
        assert_eq!(page_offset(0.0, pager.pos(), W), 0.0);
        assert_eq!(page_offset(1.0, pager.pos(), W), W);
    }

    #[test]
    fn swipe_left_advances_to_the_calendar() {
        let now = Instant::now();
        let mut pager = Pager::new();
        pager.start_drag(700.0);
        pager.drag_to(500.0, W);
        assert!(pager.pos() > 0.0);
        assert!(
            page_offset(1.0, pager.pos(), W) > 0.0,
            "calendar comes from the right"
        );
        pager.end_drag(500.0, W, now);
        assert!(pager.animating());
        pager.update(settled(now));
        assert_eq!(pager.pos(), 1.0);
        assert_eq!(page_offset(1.0, pager.pos(), W), 0.0);
    }

    #[test]
    fn swipe_right_from_the_calendar_returns_to_the_clock() {
        let now = Instant::now();
        let mut pager = Pager::new();
        pager.settle_to(1.0, now);
        pager.update(settled(now));
        assert_eq!(pager.pos(), 1.0);

        pager.start_drag(300.0);
        pager.drag_to(500.0, W);
        assert!(pager.pos() < 1.0);
        pager.end_drag(500.0, W, now);
        pager.update(settled(now));
        assert_eq!(pager.pos(), 0.0);
    }

    #[test]
    fn swipe_right_on_the_clock_goes_to_the_counter() {
        let now = Instant::now();
        let mut pager = Pager::new();
        pager.start_drag(300.0);
        pager.drag_to(700.0, W);
        assert!(pager.pos() < 0.0, "counter page is to the left");
        pager.end_drag(700.0, W, now);
        pager.update(settled(now));
        assert_eq!(pager.pos(), -1.0);
    }

    #[test]
    fn swipe_right_on_the_counter_stays_there() {
        let now = Instant::now();
        let mut pager = Pager::new();
        pager.settle_to(-1.0, now);
        pager.update(settled(now));
        assert_eq!(pager.pos(), -1.0);

        pager.start_drag(300.0);
        pager.drag_to(700.0, W);
        assert_eq!(pager.pos(), -1.0, "no page further left");
        pager.end_drag(700.0, W, now);
        pager.update(settled(now));
        assert_eq!(pager.pos(), -1.0);
    }

    #[test]
    fn short_drag_snaps_back() {
        let now = Instant::now();
        let mut pager = Pager::new();
        pager.start_drag(500.0);
        pager.drag_to(560.0, W);
        pager.end_drag(560.0, W, now);
        pager.update(settled(now));
        assert_eq!(pager.pos(), 0.0);
    }

    #[test]
    fn mid_swipe_places_the_calendar_on_the_right() {
        let pos = 0.3;
        assert!((page_offset(0.0, pos, W) + 300.0).abs() < 0.01);
        assert!((page_offset(1.0, pos, W) - 700.0).abs() < 0.01);
    }

    #[test]
    fn a_small_wiggle_is_a_tap_not_a_swipe() {
        let now = Instant::now();
        let mut pager = Pager::new();
        pager.start_drag(500.0);
        pager.drag_to(505.0, W);
        pager.drag_to(498.0, W);
        assert!(!pager.was_swipe(W));
        pager.end_drag(500.0, W, now);
        pager.update(settled(now));
        assert_eq!(pager.pos(), 0.0);
    }

    #[test]
    fn a_long_drag_counts_as_a_swipe_even_if_it_snaps_back() {
        let mut pager = Pager::new();
        pager.start_drag(500.0);
        // Travels far past the flip threshold, then wanders back near the
        // start before release -- the gesture is still a swipe.
        pager.drag_to(100.0, W);
        pager.drag_to(495.0, W);
        assert!(pager.was_swipe(W));
    }

    #[test]
    fn is_settled_on_is_false_mid_drag_or_mid_settle() {
        let now = Instant::now();
        let mut pager = Pager::new();
        assert!(pager.is_settled_on(0.0));

        pager.start_drag(500.0);
        assert!(!pager.is_settled_on(0.0));
        pager.drag_to(500.0, W);
        pager.end_drag(500.0, W, now);
        // Rounds back to 0.0 but the settle animation hasn't finished yet.
        assert!(!pager.is_settled_on(0.0));
        pager.update(settled(now));
        assert!(pager.is_settled_on(0.0));
    }
}
