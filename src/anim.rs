//! Per-character roll animation state for the clock.

use std::time::Instant;

/// How long a single character takes to travel from one slot to the next.
pub const SLIDE_SECS: f32 = 0.55;
/// Delay between neighbouring characters, for a cascading roll.
pub const STAGGER_SECS: f32 = 0.045;
/// Travel distance in em: enough for a glyph to leave the clipped line band.
pub const DISTANCE_EM: f32 = 1.1;

/// One outgoing/incoming pair for a text group (hour, minute or AM/PM).
#[derive(Debug, Clone)]
pub struct Slide {
    pub prev: String,
    pub started: Instant,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub text: String,
    pub slide: Option<Slide>,
}

impl Group {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            slide: None,
        }
    }

    /// Replaces the text, starting a slide if it actually changed.
    pub fn set(&mut self, text: &str, now: Instant) {
        if self.text == text {
            return;
        }
        let prev = std::mem::replace(&mut self.text, text.to_string());
        self.slide = Some(Slide { prev, started: now });
    }

    pub fn active(&self) -> bool {
        self.slide.is_some()
    }

    pub fn chars(&self) -> usize {
        self.text.chars().count()
    }

    /// Total duration including the stagger of the last character.
    pub fn total_secs(&self) -> f32 {
        let prev_len = self
            .slide
            .as_ref()
            .map(|s| s.prev.chars().count())
            .unwrap_or(0);
        let longest = self.chars().max(prev_len).max(1);
        SLIDE_SECS + STAGGER_SECS * (longest - 1) as f32
    }

    /// 0..=1 progress of character `index` (0-based from the left).
    pub fn progress(&self, index: usize, now: Instant) -> f32 {
        match &self.slide {
            Some(slide) => {
                let elapsed = now.duration_since(slide.started).as_secs_f32();
                ((elapsed - index as f32 * STAGGER_SECS) / SLIDE_SECS).clamp(0.0, 1.0)
            }
            None => 1.0,
        }
    }

    /// Drops the slide once every character has landed.
    pub fn prune(&mut self, now: Instant) {
        if let Some(slide) = &self.slide {
            if now.duration_since(slide.started).as_secs_f32() >= self.total_secs() {
                self.slide = None;
            }
        }
    }
}

pub struct ClockAnim {
    pub hour: Group,
    pub minute: Group,
    pub period: Group,
    pub date: Group,
}

impl ClockAnim {
    pub fn new(hour: &str, minute: &str, period: &str, date: &str) -> Self {
        Self {
            hour: Group::new(hour),
            minute: Group::new(minute),
            period: Group::new(period),
            date: Group::new(date),
        }
    }

    pub fn update(&mut self, hour: &str, minute: &str, period: &str, date: &str, now: Instant) {
        self.hour.set(hour, now);
        self.minute.set(minute, now);
        self.period.set(period, now);
        self.date.set(date, now);
    }

    pub fn any_active(&self) -> bool {
        self.hour.active() || self.minute.active() || self.period.active() || self.date.active()
    }

    pub fn prune(&mut self, now: Instant) {
        self.hour.prune(now);
        self.minute.prune(now);
        self.period.prune(now);
        self.date.prune(now);
    }
}

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Slight overshoot then settle, so the incoming glyph lands with weight.
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    const C1: f32 = 1.28;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_hit_their_endpoints() {
        for f in [ease_out_cubic, ease_out_back] {
            assert!((f(0.0) - 0.0).abs() < 1e-5);
            assert!((f(1.0) - 1.0).abs() < 1e-5);
        }
        assert!(ease_out_back(0.75) > 1.0, "incoming glyph should overshoot");
        assert!(ease_out_cubic(0.5) > 0.5, "ease out should start fast");
    }

    #[test]
    fn group_starts_sliding_only_when_text_changes() {
        let now = Instant::now();
        let mut group = Group::new("12");
        group.set("12", now);
        assert!(!group.active(), "same text must not animate");

        group.set("13", now);
        assert!(group.active());
        assert_eq!(group.text, "13");
        assert_eq!(group.slide.as_ref().unwrap().prev, "12");
    }

    #[test]
    fn characters_are_staggered_and_land() {
        let start = Instant::now();
        let mut group = Group::new("9");
        group.set("10", start);
        // Right after the change nothing has moved yet.
        assert_eq!(group.progress(0, start), 0.0);
        // Then the first character leads the second.
        let mid = start + std::time::Duration::from_millis(300);
        assert!(group.progress(0, mid) > group.progress(1, mid));

        let end = start + std::time::Duration::from_secs_f32(group.total_secs() + 0.01);
        assert_eq!(group.progress(0, end), 1.0);
        group.prune(end);
        assert!(!group.active());
    }
}
