//! Calendar page state: displayed month, month-to-month slide, and keeping the
//! highlight on today.

use std::time::Instant;

use crate::clock::{self, Clock, MonthView};

pub const SLIDE_SECS: f32 = 0.45;

pub struct Calendar {
    view: MonthView,
    slide: Option<Slide>,
    /// Today as `(year, month, day)`, for highlighting.
    today: (i32, u32, u32),
}

struct Slide {
    prev: MonthView,
    started: Instant,
    /// +1 when moving to the next month, -1 when moving back.
    dir: f32,
}

fn step_month(year: i32, month: u32, dir: i32) -> (i32, u32) {
    let index = year * 12 + month as i32 - 1 + dir;
    (index.div_euclid(12), (index.rem_euclid(12) + 1) as u32)
}

impl Calendar {
    pub fn new(clock: &Clock) -> Self {
        Self {
            view: clock.month_view(),
            slide: None,
            today: (clock.year, clock.month, clock.day),
        }
    }

    pub fn view(&self) -> &MonthView {
        &self.view
    }

    /// Moves to the neighbouring month, sliding the old one out.
    pub fn shift(&mut self, dir: i32, now: Instant) {
        let (year, month) = step_month(self.view.year, self.view.month, dir);
        let mut next = clock::month_view_for(year, month, None);
        if (year, month) == (self.today.0, self.today.1) {
            next.today = Some(self.today.2);
        }
        let prev = std::mem::replace(&mut self.view, next);
        self.slide = Some(Slide {
            prev,
            started: now,
            dir: dir.signum() as f32,
        });
    }

    pub fn animating(&self) -> bool {
        self.slide.is_some()
    }

    /// `(progress 0..=1, outgoing view, direction)` while a month slide runs.
    pub fn slide(&self, now: Instant) -> Option<(f32, &MonthView, f32)> {
        let slide = self.slide.as_ref()?;
        let t = (now.duration_since(slide.started).as_secs_f32() / SLIDE_SECS).clamp(0.0, 1.0);
        Some((t, &slide.prev, slide.dir))
    }

    pub fn settle(&mut self, now: Instant) {
        if let Some(slide) = &self.slide {
            if now.duration_since(slide.started).as_secs_f32() >= SLIDE_SECS {
                self.slide = None;
            }
        }
    }

    /// Keeps the highlight on today, and follows the month when it rolls over
    /// while the calendar is showing the current month.
    pub fn sync(&mut self, clock: &Clock) {
        let current = (clock.year, clock.month);
        let previous = (self.today.0, self.today.1);
        if current != previous && (self.view.year, self.view.month) == previous {
            self.view = clock.month_view();
            self.slide = None;
        }
        self.today = (clock.year, clock.month, clock.day);
        self.view.today = if (self.view.year, self.view.month) == current {
            Some(clock.day)
        } else {
            None
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use std::time::Duration;

    fn clock_at(y: i32, m: u32, d: u32) -> Clock {
        let dt = Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap();
        clock::from_datetime(&dt)
    }

    #[test]
    fn month_steps_across_years() {
        assert_eq!(step_month(2026, 12, 1), (2027, 1));
        assert_eq!(step_month(2027, 1, -1), (2026, 12));
    }

    #[test]
    fn next_and_previous_slide() {
        let now = Instant::now();
        let mut cal = Calendar::new(&clock_at(2026, 11, 23));
        assert_eq!(cal.view().title, "NOVEMBER 2026");
        assert_eq!(cal.view().today, Some(23));

        cal.shift(1, now);
        assert_eq!(cal.view().title, "DECEMBER 2026");
        assert_eq!(cal.view().today, None, "other months have no today");
        let (progress, prev, dir) = cal.slide(now).unwrap();
        assert_eq!(progress, 0.0);
        assert_eq!(prev.title, "NOVEMBER 2026");
        assert_eq!(dir, 1.0);

        let done = now + Duration::from_secs_f32(SLIDE_SECS + 0.01);
        cal.settle(done);
        assert!(!cal.animating());

        cal.shift(-1, done);
        assert_eq!(cal.view().title, "NOVEMBER 2026");
        assert_eq!(cal.view().today, Some(23));
    }

    #[test]
    fn follows_month_rollover_only_when_showing_it() {
        let mut cal = Calendar::new(&clock_at(2026, 11, 23));
        cal.sync(&clock_at(2026, 12, 1));
        assert_eq!(cal.view().title, "DECEMBER 2026");
        assert_eq!(cal.view().today, Some(1));

        cal.shift(1, Instant::now());
        cal.sync(&clock_at(2027, 1, 1));
        assert_eq!(
            cal.view().title,
            "JANUARY 2027",
            "moved month must stay put"
        );
    }
}
