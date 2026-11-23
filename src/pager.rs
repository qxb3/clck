//! Two-page horizontal pager: page 0 is the clock, page 1 the calendar, which
//! sits to its right. Swiping left (finger moves left) advances to the
//! calendar; swiping right returns to the clock.

use std::time::Instant;

use crate::anim::ease_out_cubic;

const SETTLE_SECS: f32 = 0.42;
/// Fraction of the width a drag must cover to flip the page.
const FLIP_FRACTION: f32 = 0.16;

#[derive(Default)]
pub struct Pager {
    pos: f32,
    drag: Option<(f32, f32)>,
    settle: Option<(f32, f32, Instant)>,
}

impl Pager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Continuous page position: 0 = clock, 1 = calendar.
    pub fn pos(&self) -> f32 {
        self.pos
    }

    pub fn start_drag(&mut self, x: f32) {
        self.drag = Some((x, self.pos));
        self.settle = None;
    }

    pub fn drag_to(&mut self, x: f32, width: f32) {
        if let Some((start_x, start_pos)) = self.drag {
            if width > 0.0 {
                // Fingers move content with them, so dragging left advances.
                self.pos = (start_pos - (x - start_x) / width).clamp(0.0, 1.0);
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
                (start_pos.round() + 1.0).min(1.0)
            } else {
                (start_pos.round() - 1.0).max(0.0)
            }
        } else {
            start_pos.round().clamp(0.0, 1.0)
        };
        self.settle = Some((self.pos, to, now));
    }

    pub fn cancel_drag(&mut self, now: Instant) {
        if self.drag.take().is_some() {
            self.settle = Some((self.pos, self.pos.round().clamp(0.0, 1.0), now));
        }
    }

    pub fn animating(&self) -> bool {
        self.settle.is_some()
    }

    /// Advances the settle animation; returns true while a frame is needed.
    pub fn update(&mut self, now: Instant) -> bool {
        let Some((from, to, started)) = self.settle else {
            return false;
        };
        let t = now.duration_since(started).as_secs_f32() / SETTLE_SECS;
        if t >= 1.0 {
            self.pos = to.clamp(0.0, 1.0);
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
pub fn page_offset(page: usize, pos: f32, width: f32) -> f32 {
    (page as f32 - pos) * width
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
    fn starts_on_the_clock_with_the_calendar_to_its_right() {
        let pager = Pager::new();
        assert_eq!(pager.pos(), 0.0);
        assert_eq!(page_offset(0, pager.pos(), W), 0.0);
        assert_eq!(page_offset(1, pager.pos(), W), W);
    }

    #[test]
    fn swipe_left_advances_to_the_calendar() {
        let now = Instant::now();
        let mut pager = Pager::new();
        pager.start_drag(700.0);
        pager.drag_to(500.0, W);
        assert!(pager.pos() > 0.0);
        assert!(
            page_offset(1, pager.pos(), W) > 0.0,
            "calendar comes from the right"
        );
        pager.end_drag(500.0, W, now);
        assert!(pager.animating());
        pager.update(settled(now));
        assert_eq!(pager.pos(), 1.0);
        assert_eq!(page_offset(1, pager.pos(), W), 0.0);
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
    fn swiping_right_on_the_clock_stays_there() {
        let now = Instant::now();
        let mut pager = Pager::new();
        pager.start_drag(300.0);
        pager.drag_to(700.0, W);
        assert_eq!(pager.pos(), 0.0, "no page to the left");
        pager.end_drag(700.0, W, now);
        pager.update(settled(now));
        assert_eq!(pager.pos(), 0.0);
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
        assert!((page_offset(0, pos, W) + 300.0).abs() < 0.01);
        assert!((page_offset(1, pos, W) - 700.0).abs() < 0.01);
    }
}
