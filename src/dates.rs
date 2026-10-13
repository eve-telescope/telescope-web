//! The three ways the API writes a moment, and the dates it reads.
//!
//! - Models (users, networks, entries, raw access rows, the share page's
//!   `createdAt`): `2026-10-13T08:30:00.000000Z`.
//! - The access and scan answers (`toIso8601String()`):
//!   `2026-10-13T08:30:00+00:00`.
//!
//! Everything is UTC, as the app and its server always were.

use time::macros::format_description;
use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

/// A model's timestamp.
pub fn model(moment: OffsetDateTime) -> String {
    moment
        .to_offset(UtcOffset::UTC)
        .format(format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second].000000Z"
        ))
        .expect("a timestamp")
}

/// `toIso8601String()`.
pub fn iso8601(moment: OffsetDateTime) -> String {
    moment
        .to_offset(UtcOffset::UTC)
        .format(format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second]+00:00"
        ))
        .expect("a timestamp")
}

/// The parts of `YYYY-MM-DD[( |T)HH:MM[:SS[.fraction]]][Z|±HH[:]MM]`.
struct Parts {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    offset_seconds: i32,
}

fn number(text: &str, digits: usize) -> Option<u32> {
    (text.len() == digits && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

fn parts(text: &str) -> Option<Parts> {
    let (date, rest) = if text.len() > 10 {
        (&text[..10], &text[10..])
    } else {
        (text, "")
    };
    let mut fields = date.split('-');
    let year = number(fields.next()?, 4)? as i32;
    let month = number(fields.next()?, 2)? as u8;
    let day = number(fields.next()?, 2)? as u8;
    if fields.next().is_some() {
        return None;
    }
    let mut parts = Parts {
        year,
        month,
        day,
        hour: 0,
        minute: 0,
        second: 0,
        offset_seconds: 0,
    };
    if rest.is_empty() {
        return Some(parts);
    }
    let rest = rest.strip_prefix(['T', ' '])?;
    // The zone at the end: `Z`, `+02:00` or `+0200`.
    let (clock, zone) = match rest.find(['Z', '+', '-']) {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };
    let (clock, _fraction) = match clock.split_once('.') {
        Some((clock, fraction))
            if !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()) =>
        {
            (clock, fraction)
        }
        Some(_) => return None,
        None => (clock, ""),
    };
    let mut times = clock.split(':');
    parts.hour = number(times.next()?, 2)? as u8;
    parts.minute = number(times.next()?, 2)? as u8;
    if let Some(second) = times.next() {
        parts.second = number(second, 2)? as u8;
    }
    if times.next().is_some() || parts.hour > 23 || parts.minute > 59 || parts.second > 59 {
        return None;
    }
    parts.offset_seconds = match zone {
        "" | "Z" => 0,
        zone => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let digits = zone[1..].replace(':', "");
            let hours = number(&digits[..2.min(digits.len())], 2)? as i32;
            let minutes = number(digits.get(2..)?, 2)? as i32;
            if hours > 14 || minutes > 59 {
                return None;
            }
            sign * (hours * 3600 + minutes * 60)
        }
    };
    Some(parts)
}

fn moment(parts: &Parts, date: Date) -> Option<OffsetDateTime> {
    let time = Time::from_hms(parts.hour, parts.minute, parts.second).ok()?;
    let offset = UtcOffset::from_whole_seconds(parts.offset_seconds).ok()?;
    Some(
        PrimitiveDateTime::new(date, time)
            .assume_offset(offset)
            .to_offset(UtcOffset::UTC),
    )
}

/// The `date` rule's reading: an ISO 8601 date or date-time of a day that
/// exists, in UTC unless it names its offset.
pub fn parse(text: &str) -> Option<OffsetDateTime> {
    let parts = parts(text)?;
    let month = Month::try_from(parts.month).ok()?;
    let date = Date::from_calendar_date(parts.year, month, parts.day).ok()?;
    moment(&parts, date)
}

/// What `after:now` compares, like PHP's `strtotime`: the same forms, a
/// day past the month's end rolling over (`2027-02-30` is 2 March), and
/// the relative words `now`, `today`, `midnight`, `tomorrow` and
/// `yesterday`.
pub fn parse_lenient(text: &str, now: OffsetDateTime) -> Option<OffsetDateTime> {
    let midnight = now.replace_time(Time::MIDNIGHT);
    match text.to_ascii_lowercase().as_str() {
        "now" => return Some(now),
        "today" | "midnight" => return Some(midnight),
        "tomorrow" => return Some(midnight + Duration::days(1)),
        "yesterday" => return Some(midnight - Duration::days(1)),
        _ => {}
    }
    let parts = parts(text)?;
    if !(1..=12).contains(&parts.month) || !(1..=31).contains(&parts.day) {
        return None;
    }
    let first = Date::from_calendar_date(parts.year, Month::try_from(parts.month).ok()?, 1).ok()?;
    moment(&parts, first + Duration::days(i64::from(parts.day) - 1))
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn writes_the_three_forms() {
        let moment = datetime!(2026-10-13 08:30:05 UTC);
        assert_eq!(model(moment), "2026-10-13T08:30:05.000000Z");
        assert_eq!(iso8601(moment), "2026-10-13T08:30:05+00:00");
    }

    #[test]
    fn reads_iso_dates() {
        assert_eq!(
            parse("2027-06-01"),
            Some(datetime!(2027-06-01 00:00:00 UTC))
        );
        assert_eq!(
            parse("2027-06-01 12:00"),
            Some(datetime!(2027-06-01 12:00:00 UTC))
        );
        assert_eq!(
            parse("2027-06-01T10:00:00+02:00"),
            Some(datetime!(2027-06-01 08:00:00 UTC))
        );
        assert_eq!(
            parse("2027-06-01T10:00:00.123-0130"),
            Some(datetime!(2027-06-01 11:30:00 UTC))
        );
        assert_eq!(
            parse("2027-06-01T10:00:00Z"),
            Some(datetime!(2027-06-01 10:00:00 UTC))
        );
        for invalid in [
            "2027-02-30",
            "2027-13-01",
            "27-06-01",
            "2027-06-01T25:00",
            "tomorrow",
            "June 1 2027",
            "2027-06-01T10:00:00+2",
            "",
        ] {
            assert_eq!(parse(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn compares_like_strtotime() {
        let now = datetime!(2026-10-13 08:30:00 UTC);
        assert_eq!(
            parse_lenient("2027-02-30", now),
            Some(datetime!(2027-03-02 00:00:00 UTC))
        );
        assert_eq!(
            parse_lenient("tomorrow", now),
            Some(datetime!(2026-10-14 00:00:00 UTC))
        );
        assert_eq!(parse_lenient("not a date", now), None);
        assert_eq!(parse_lenient("2027-13-01", now), None);
    }
}
