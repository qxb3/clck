//! Tap counter page. Purely in-memory: the count lives only for the current
//! process and resets to zero whenever the app is restarted.

#[derive(Default)]
pub struct Counter {
    count: u32,
}

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn increment(&mut self) {
        self.count = self.count.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero_and_increments() {
        let mut counter = Counter::new();
        assert_eq!(counter.count(), 0);
        counter.increment();
        counter.increment();
        assert_eq!(counter.count(), 2);
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        let mut counter = Counter { count: u32::MAX };
        counter.increment();
        assert_eq!(counter.count(), u32::MAX);
    }
}
