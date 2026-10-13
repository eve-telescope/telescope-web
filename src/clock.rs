//! The app's notion of now, in whole seconds like the timestamps it
//! stores. Every day boundary is UTC midnight. Tests fix the clock at a
//! known moment and move it by hand.

use std::sync::{Arc, RwLock};

use time::{Date, OffsetDateTime};

#[derive(Clone, Default)]
pub struct Clock {
    /// `None` follows the system clock.
    fixed: Arc<RwLock<Option<OffsetDateTime>>>,
}

impl Clock {
    pub fn system() -> Self {
        Self::default()
    }

    pub fn fixed(now: OffsetDateTime) -> Self {
        let clock = Self::default();
        clock.set(now);
        clock
    }

    /// Moves a fixed clock to `now`.
    pub fn set(&self, now: OffsetDateTime) {
        *self.fixed.write().expect("clock lock") = Some(now);
    }

    /// Moves a fixed clock forward.
    pub fn advance(&self, by: time::Duration) {
        let now = self.now();
        self.set(now + by);
    }

    pub fn now(&self) -> OffsetDateTime {
        let now = self
            .fixed
            .read()
            .expect("clock lock")
            .unwrap_or_else(OffsetDateTime::now_utc)
            .to_offset(time::UtcOffset::UTC);
        now.replace_nanosecond(0).expect("zero nanoseconds")
    }

    /// Today in UTC.
    pub fn today(&self) -> Date {
        self.now().date()
    }
}
