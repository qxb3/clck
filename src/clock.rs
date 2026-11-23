//! Time/date model. Kept free of Android APIs so it can be unit tested.

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;

const MONTHS: [&str; 12] = [
    "JANUARY",
    "FEBRUARY",
    "MARCH",
    "APRIL",
    "MAY",
    "JUNE",
    "JULY",
    "AUGUST",
    "SEPTEMBER",
    "OCTOBER",
    "NOVEMBER",
    "DECEMBER",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clock {
    /// Hour in 12-hour form, 1..=12 (no leading zero).
    pub hour: u32,
    pub minute: u32,
    pub pm: bool,
    /// e.g. `MONDAY - NOV 23 2026`
    pub date: String,
    pub year: i32,
    /// 1..=12
    pub month: u32,
    pub day: u32,
    /// 0 = Monday
    pub weekday_monday: u32,
}

/// A month laid out for the calendar page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthView {
    pub year: i32,
    /// 1..=12
    pub month: u32,
    /// e.g. `NOVEMBER 2026`
    pub title: String,
    pub weeks: usize,
    /// `weeks * 7` cells, `None` before the 1st and after the last day.
    pub cells: Vec<Option<u32>>,
    /// Day of the current month, only when this view is today's month.
    pub today: Option<u32>,
}

/// Layout for any month, with `today` highlighted when it belongs to it.
pub fn month_view_for(year: i32, month: u32, today: Option<u32>) -> MonthView {
    let month = month.clamp(1, 12);
    let first = NaiveDate::from_ymd_opt(year, month, 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());
    let offset = first.weekday().num_days_from_monday() as usize;
    let days = days_in_month(year, month);
    let weeks = (offset + days as usize).div_ceil(7).max(1);
    let mut cells = vec![None; weeks * 7];
    for day in 1..=days {
        cells[offset + day as usize - 1] = Some(day);
    }
    MonthView {
        year,
        month,
        title: format!("{} {}", MONTHS[(month - 1) as usize], year),
        weeks,
        cells,
        today,
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|d| d.pred_opt())
        .map(|d| d.day())
        .unwrap_or(28)
}

impl Clock {
    pub fn period(&self) -> &'static str {
        if self.pm {
            "PM"
        } else {
            "AM"
        }
    }

    /// e.g. `9:07` or `12:42` (tests only; the UI draws hour and minute as
    /// separate animated groups)
    #[cfg(test)]
    pub fn time_label(&self) -> String {
        format!("{}:{:02}", self.hour, self.minute)
    }
}

pub fn from_datetime<Tz2: chrono::TimeZone>(dt: &DateTime<Tz2>) -> Clock
where
    Tz2::Offset: std::fmt::Display,
{
    let h24 = dt.hour();
    let date = format!(
        "{} - {} {} {}",
        dt.format("%A"),
        dt.format("%b"),
        dt.day(),
        dt.year()
    )
    .to_uppercase();
    Clock {
        hour: match h24 % 12 {
            0 => 12,
            h => h,
        },
        minute: dt.minute(),
        pm: h24 >= 12,
        date,
        year: dt.year(),
        month: dt.month(),
        day: dt.day(),
        weekday_monday: dt.weekday().num_days_from_monday(),
    }
}

impl Clock {
    pub fn month_view(&self) -> MonthView {
        month_view_for(self.year, self.month, Some(self.day))
    }
}

/// Formats an absolute instant in `tz`.
pub fn at_epoch(tz: Tz, secs: i64) -> Clock {
    let dt = DateTime::<Utc>::from_timestamp(secs, 0)
        .unwrap_or_default()
        .with_timezone(&tz);
    from_datetime(&dt)
}

/// Current wall clock plus a test offset, keeping the clock live.
pub fn now_in(tz: Tz, offset_secs: i64) -> Clock {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    at_epoch(tz, now + offset_secs)
}

/// Debug hook for jump-testing transitions without waiting for real time:
///
/// ```sh
/// adb shell setprop debug.clck.offset_secs 43200   # +12 hours
/// ```
///
/// The value is added to the system clock, so the display keeps ticking from
/// the shifted time. `scripts/simulate.sh` sets this for you.
#[cfg(target_os = "android")]
pub fn simulated_offset() -> i64 {
    android_properties::getprop("debug.clck.offset_secs")
        .value()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(0)
}

/// Android does not export a `TZ` variable to native activities, so bionic's
/// `localtime` would hand us UTC. Read the platform's zone name instead.
#[cfg(target_os = "android")]
pub fn android_timezone() -> Tz {
    let name = android_properties::getprop("persist.sys.timezone").value();
    name.as_deref()
        .and_then(|n| n.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32, m: u32) -> Clock {
        let dt = chrono_tz::UTC
            .with_ymd_and_hms(2026, 11, 23, h, m, 0)
            .unwrap();
        from_datetime(&dt)
    }

    #[test]
    fn midnight_is_twelve_am() {
        let c = at(0, 5);
        assert_eq!(c.time_label(), "12:05");
        assert_eq!(c.period(), "AM");
    }

    #[test]
    fn noon_is_twelve_pm() {
        let c = at(12, 0);
        assert_eq!(c.time_label(), "12:00");
        assert_eq!(c.period(), "PM");
    }

    #[test]
    fn afternoon_has_no_leading_zero() {
        let c = at(15, 7);
        assert_eq!(c.time_label(), "3:07");
        assert_eq!(c.period(), "PM");
    }

    #[test]
    fn date_is_short_and_uppercase() {
        let c = at(9, 0);
        assert_eq!(c.date, "MONDAY - NOV 23 2026");
    }

    #[test]
    fn calendar_month_starts_on_the_right_weekday() {
        // 1 Nov 2026 is a Sunday, so the grid starts with six empty cells.
        let c = at(12, 0);
        let view = c.month_view();
        assert_eq!(view.title, "NOVEMBER 2026");
        assert_eq!(view.today, Some(23));
        assert_eq!((view.year, view.month), (2026, 11));
        assert_eq!(view.weeks, 6);
        assert_eq!(view.cells.len(), 42);
        assert!(view.cells[..6].iter().all(|c| c.is_none()));
        assert_eq!(view.cells[6], Some(1));
        assert_eq!(view.cells[35], Some(30));
        assert!(view.cells[36..].iter().all(|c| c.is_none()));
    }

    #[test]
    fn calendar_handles_leap_years_and_five_week_months() {
        let leap = at_epoch(
            chrono_tz::UTC,
            chrono::NaiveDate::from_ymd_opt(2028, 2, 10)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap()
                .and_utc()
                .timestamp(),
        );
        let view = leap.month_view();
        assert_eq!(view.title, "FEBRUARY 2028");
        assert_eq!(view.weeks, 5);
        assert_eq!(view.cells.iter().flatten().count(), 29);
        assert_eq!(view.today, Some(10));
    }

    #[test]
    fn epoch_formatting_handles_period_and_date() {
        // 2027-01-15T08:00:00Z, a Friday morning.
        let c = at_epoch(chrono_tz::UTC, 1_800_000_000);
        assert_eq!(c.time_label(), "8:00");
        assert_eq!(c.period(), "AM");
        assert_eq!(c.date, "FRIDAY - JAN 15 2027");

        let evening = at_epoch(chrono_tz::UTC, 1_800_000_000 + 12 * 3600);
        assert_eq!(evening.time_label(), "8:00");
        assert_eq!(evening.period(), "PM");
    }
}
