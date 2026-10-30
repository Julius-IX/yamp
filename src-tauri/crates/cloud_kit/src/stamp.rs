//! Sortable timestamp names.
//!
//! The format is ISO 8601 with dashes in the time part (`2026-10-30T11-49-12.345Z`),
//! because colons are not allowed in keys. It has a fixed width, so sorting names as
//! text sorts them in time order.

use std::{
  sync::atomic::{AtomicI64, Ordering},
  time::{SystemTime, UNIX_EPOCH},
};

use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};

const LEN: usize = 24;
static LAST_MS: AtomicI64 = AtomicI64::new(0);

/// A name for the current moment. Calls in one process are strictly increasing, even
/// when the clock stalls or steps back, so appended entries never reorder or collide.
pub fn now() -> String {
  format_ms(next_ms())
}

fn next_ms() -> i64 {
  let wall = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |d| d.as_millis() as i64);
  loop {
    let last = LAST_MS.load(Ordering::SeqCst);
    let next = wall.max(last + 1);
    if LAST_MS
      .compare_exchange(last, next, Ordering::SeqCst, Ordering::SeqCst)
      .is_ok()
    {
      return next;
    }
  }
}

pub fn format_ms(ms: i64) -> String {
  let t = OffsetDateTime::from_unix_timestamp_nanos(ms as i128 * 1_000_000)
    .unwrap_or(OffsetDateTime::UNIX_EPOCH);
  format!(
    "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}.{:03}Z",
    t.year(),
    t.month() as u8,
    t.day(),
    t.hour(),
    t.minute(),
    t.second(),
    t.millisecond()
  )
}

/// Milliseconds since the Unix epoch of a name made by [`format_ms`], `None` for anything else.
pub fn parse_ms(s: &str) -> Option<i64> {
  let b = s.as_bytes();
  if s.len() != LEN || !s.is_ascii() {
    return None;
  }
  let shape = [
    (4, b'-'),
    (7, b'-'),
    (10, b'T'),
    (13, b'-'),
    (16, b'-'),
    (19, b'.'),
    (23, b'Z'),
  ];
  if shape.iter().any(|&(i, c)| b[i] != c) {
    return None;
  }
  let num = |a: usize, z: usize| -> Option<i64> {
    let part = &s[a..z];
    part
      .bytes()
      .all(|c| c.is_ascii_digit())
      .then(|| part.parse().ok())
      .flatten()
  };
  let date = Date::from_calendar_date(
    num(0, 4)? as i32,
    Month::try_from(num(5, 7)? as u8).ok()?,
    num(8, 10)? as u8,
  )
  .ok()?;
  let time = Time::from_hms_milli(
    num(11, 13)? as u8,
    num(14, 16)? as u8,
    num(17, 19)? as u8,
    num(20, 23)? as u16,
  )
  .ok()?;
  let nanos = PrimitiveDateTime::new(date, time)
    .assume_utc()
    .unix_timestamp_nanos();
  i64::try_from(nanos / 1_000_000).ok()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formats_and_parses() {
    assert_eq!(format_ms(0), "1970-01-01T00-00-00.000Z");
    let ms = 1_761_824_952_345;
    assert_eq!(format_ms(ms), "2025-10-30T11-49-12.345Z");
    assert_eq!(parse_ms(&format_ms(ms)), Some(ms));
  }

  #[test]
  fn rejects_other_shapes() {
    for s in [
      "",
      "2025-10-30",
      "2025-10-30T11:49:12.345Z",
      "2025-13-30T11-49-12.345Z",
      "+025-10-30T11-49-12.345Z",
    ] {
      assert_eq!(parse_ms(s), None, "{s}");
    }
  }

  #[test]
  fn names_are_strictly_increasing() {
    let names: Vec<String> = (0..500).map(|_| now()).collect();
    assert!(names.windows(2).all(|w| w[0] < w[1]));
  }
}
