use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDateTime, Utc};

use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::{Error, Result};

/// Milliseconds between the unix epoch and the block timestamp epoch (2000-01-01).
const BLOCK_TIMESTAMP_EPOCH_MS: i64 = 946_684_800_000;

/// Parse an ISO 8601 date string as UTC, e.g. `2020-07-01T17:32:13` or `2021-08-25T02:37:24.500`.
fn parse_date_ms(s: &str) -> Result<i64> {
    let trimmed = s.trim().trim_end_matches('Z');
    let formats = ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S%.f"];
    for format in formats {
        if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, format) {
            return Ok(dt.and_utc().timestamp_millis());
        }
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(date
            .and_hms_opt(0, 0, 0)
            .expect("valid time")
            .and_utc()
            .timestamp_millis());
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s.trim()) {
        return Ok(dt.timestamp_millis());
    }
    Err(Error::msg("Invalid date string"))
}

fn format_ms(ms: i64, millis: bool) -> String {
    let dt = DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_default();
    if millis {
        dt.format("%Y-%m-%dT%H:%M:%S%.3f").to_string()
    } else {
        dt.format("%Y-%m-%dT%H:%M:%S").to_string()
    }
}

/// Current time in milliseconds since the unix epoch.
pub(crate) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

macro_rules! time_common {
    ($name:ident, $abi:literal, $millis:expr) => {
        impl $name {
            /// Create from an ISO 8601 date string (interpreted as UTC).
            pub fn from_string(s: &str) -> Result<Self> {
                Ok(Self::from_milliseconds(parse_date_ms(s)?))
            }

            /// Create from a chrono `DateTime`.
            pub fn from_date(date: DateTime<Utc>) -> Self {
                Self::from_milliseconds(date.timestamp_millis())
            }

            /// Current time.
            pub fn now() -> Self {
                Self::from_milliseconds(now_ms())
            }

            /// Convert to a chrono `DateTime`.
            pub fn to_date(&self) -> DateTime<Utc> {
                DateTime::<Utc>::from_timestamp_millis(self.to_milliseconds()).unwrap_or_default()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&format_ms(self.to_milliseconds(), $millis))
            }
        }

        impl FromStr for $name {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self> {
                Self::from_string(s)
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
                struct V;
                impl<'de> serde::de::Visitor<'de> for V {
                    type Value = $name;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("a date string or integer")
                    }
                    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<$name, E> {
                        $name::from_string(v).map_err(E::custom)
                    }
                    fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<$name, E> {
                        $name::from_integer(v).map_err(E::custom)
                    }
                    fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<$name, E> {
                        $name::from_integer(v as i64).map_err(E::custom)
                    }
                }
                d.deserialize_any(V)
            }
        }
    };
}

/// Timestamp with microsecond accuracy (`time_point`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimePoint(pub i64);

impl TimePoint {
    pub fn from_milliseconds(ms: i64) -> Self {
        TimePoint(ms.saturating_mul(1000))
    }

    /// Create from the raw value (microseconds since epoch).
    pub fn from_integer(value: i64) -> Result<Self> {
        Ok(TimePoint(value))
    }

    /// Microseconds since epoch.
    pub fn value(&self) -> i64 {
        self.0
    }

    pub fn to_milliseconds(&self) -> i64 {
        // round half away from zero like `dividing(1000, 'round')`
        let q = self.0 / 1000;
        let r = self.0 % 1000;
        if r >= 500 {
            q + 1
        } else if r <= -500 {
            q - 1
        } else {
            q
        }
    }
}

time_common!(TimePoint, "time_point", true);

impl ABISerializable for TimePoint {
    fn abi_type() -> String {
        "time_point".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0.to_le_bytes());
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(TimePoint(i64::from_le_bytes(decoder.read_fixed()?)))
    }
}

fn div_round(a: i64, b: i64) -> i64 {
    // Math.round semantics (round half up)
    (a as f64 / b as f64 + 0.5).floor() as i64
}

/// Timestamp with second accuracy (`time_point_sec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimePointSec(pub u32);

impl TimePointSec {
    pub fn from_milliseconds(ms: i64) -> Self {
        TimePointSec(div_round(ms, 1000).clamp(0, u32::MAX as i64) as u32)
    }

    /// Create from the raw value (seconds since epoch).
    pub fn from_integer(value: i64) -> Result<Self> {
        u32::try_from(value)
            .map(TimePointSec)
            .map_err(|_| Error::msg(format!("Number {} overflows uint32", value)))
    }

    /// Seconds since epoch.
    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn to_milliseconds(&self) -> i64 {
        self.0 as i64 * 1000
    }
}

time_common!(TimePointSec, "time_point_sec", false);

impl ABISerializable for TimePointSec {
    fn abi_type() -> String {
        "time_point_sec".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0.to_le_bytes());
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(TimePointSec(u32::from_le_bytes(decoder.read_fixed()?)))
    }
}

/// Block timestamp (`block_timestamp_type`), half-second slots since 2000-01-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BlockTimestamp(pub u32);

impl BlockTimestamp {
    pub fn from_milliseconds(ms: i64) -> Self {
        BlockTimestamp(div_round(ms - BLOCK_TIMESTAMP_EPOCH_MS, 500).clamp(0, u32::MAX as i64) as u32)
    }

    /// Create from the raw value (slot number).
    pub fn from_integer(value: i64) -> Result<Self> {
        u32::try_from(value)
            .map(BlockTimestamp)
            .map_err(|_| Error::msg(format!("Number {} overflows uint32", value)))
    }

    /// Slot number.
    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn to_milliseconds(&self) -> i64 {
        self.0 as i64 * 500 + BLOCK_TIMESTAMP_EPOCH_MS
    }
}

time_common!(BlockTimestamp, "block_timestamp_type", true);

impl ABISerializable for BlockTimestamp {
    fn abi_type() -> String {
        "block_timestamp_type".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0.to_le_bytes());
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(BlockTimestamp(u32::from_le_bytes(decoder.read_fixed()?)))
    }
}

macro_rules! time_conversions {
    ($from:ident => $($to:ident),*) => {
        $(
            impl From<$from> for $to {
                fn from(v: $from) -> Self {
                    $to::from_milliseconds(v.to_milliseconds())
                }
            }
        )*
    };
}

time_conversions!(TimePoint => TimePointSec, BlockTimestamp);
time_conversions!(TimePointSec => TimePoint, BlockTimestamp);
time_conversions!(BlockTimestamp => TimePoint, TimePointSec);
