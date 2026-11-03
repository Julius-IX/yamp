use std::time::{SystemTime, UNIX_EPOCH};

/// Milliseconds since the Unix epoch, the unit of every timestamp column.
pub(crate) fn now_ms() -> i64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_or(0, |d| d.as_millis() as i64)
}
