//! Fixed and variable width integer types.
//!
//! Each type wraps the matching Rust primitive. Arithmetic operators follow C++
//! semantics for overflow (wrap around), `checked_*` variants are available on the
//! wrapped value via [`value`](UInt64::value).
//!
//! JSON representation follows FC: integers whose magnitude fits in 32 bits are
//! written as JSON numbers, larger ones as strings. Both numbers and strings are
//! accepted when deserializing.

use std::fmt;
use std::str::FromStr;

use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::utils::secure_random;
use crate::{Error, Result};

macro_rules! int_type {
    ($(#[$meta:meta])* $name:ident, $prim:ty, $abi:literal, $signed:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(pub $prim);

        impl $name {
            /// Largest value that can be represented by this integer type.
            pub const MAX: $name = $name(<$prim>::MAX);
            /// Smallest value that can be represented by this integer type.
            pub const MIN: $name = $name(<$prim>::MIN);
            /// Zero value.
            pub const ZERO: $name = $name(0);
            /// Size in bytes.
            pub const BYTE_WIDTH: usize = std::mem::size_of::<$prim>();
            /// Whether the type is signed.
            pub const IS_SIGNED: bool = $signed;

            pub const fn new(value: $prim) -> Self {
                $name(value)
            }

            /// The underlying primitive value.
            pub const fn value(self) -> $prim {
                self.0
            }

            /// Create from little-endian bytes, `bytes` must be exactly the type's byte width.
            pub fn from_le_slice(bytes: &[u8]) -> Result<Self> {
                let arr: [u8; std::mem::size_of::<$prim>()] = bytes
                    .try_into()
                    .map_err(|_| Error::msg(concat!("Invalid byte length for ", $abi)))?;
                Ok($name(<$prim>::from_le_bytes(arr)))
            }

            /// Number as bytes in little endian (matches memory layout in C++ contract).
            pub fn byte_array(self) -> Vec<u8> {
                self.0.to_le_bytes().to_vec()
            }

            /// Create a random value using a secure random source.
            pub fn random() -> Self {
                Self::from_le_slice(&secure_random(Self::BYTE_WIDTH)).expect("valid size")
            }

            /// Create a new instance from any integer that fits, errors on overflow.
            pub fn try_from_i128(value: i128) -> Result<Self> {
                <$prim>::try_from(value).map($name).map_err(|_| {
                    if value < 0 {
                        Error::msg(format!("Number {} underflows {}", value, $abi))
                    } else {
                        Error::msg(format!("Number {} overflows {}", value, $abi))
                    }
                })
            }

            /// Create a new instance clamping the value within the supported range.
            pub fn clamped_from_i128(value: i128) -> Self {
                match <$prim>::try_from(value) {
                    Ok(v) => $name(v),
                    Err(_) if value < 0 => Self::MIN,
                    Err(_) => Self::MAX,
                }
            }

            /// Create a new instance truncating the value (C++11 conversion behavior).
            pub fn truncated_from_i128(value: i128) -> Self {
                $name(value as $prim)
            }

            /// Lossy conversion to f64.
            pub fn to_f64(self) -> f64 {
                self.0 as f64
            }

            /// Divide rounding to the nearest integer (half away from zero).
            pub fn div_round(self, rhs: Self) -> Result<Self> {
                if rhs.0 == 0 {
                    return Err(Error::msg("Division by zero"));
                }
                let q = self.0.wrapping_div(rhs.0);
                let r = self.0.wrapping_rem(rhs.0);
                if r == 0 {
                    return Ok($name(q));
                }
                // compare |2r| >= |rhs| without overflow
                #[allow(unused_comparisons)]
                let neg = (self.0 < 0) != (rhs.0 < 0);
                let r_abs = Self::abs_u128(r);
                let d_abs = Self::abs_u128(rhs.0);
                if r_abs >= d_abs - r_abs {
                    Ok($name(if neg { q.wrapping_sub(1) } else { q.wrapping_add(1) }))
                } else {
                    Ok($name(q))
                }
            }

            /// Divide rounding up (towards positive infinity).
            pub fn div_ceil(self, rhs: Self) -> Result<Self> {
                if rhs.0 == 0 {
                    return Err(Error::msg("Division by zero"));
                }
                let q = self.0.wrapping_div(rhs.0);
                let r = self.0.wrapping_rem(rhs.0);
                #[allow(unused_comparisons)]
                let neg = (self.0 < 0) != (rhs.0 < 0);
                if r != 0 && !neg {
                    Ok($name(q.wrapping_add(1)))
                } else {
                    Ok($name(q))
                }
            }

            /// Divide rounding down (truncating), errors when dividing by zero.
            pub fn div_floor(self, rhs: Self) -> Result<Self> {
                if rhs.0 == 0 {
                    return Err(Error::msg("Division by zero"));
                }
                Ok($name(self.0.wrapping_div(rhs.0)))
            }

            fn abs_u128(v: $prim) -> u128 {
                if $signed {
                    (v as i128).unsigned_abs()
                } else {
                    v as u128
                }
            }

            fn magnitude_fits_32(self) -> bool {
                Self::abs_u128(self.0) <= u32::MAX as u128
            }
        }

        impl From<$prim> for $name {
            fn from(v: $prim) -> Self {
                $name(v)
            }
        }

        impl From<$name> for $prim {
            fn from(v: $name) -> Self {
                v.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl FromStr for $name {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self> {
                let s = s.trim();
                if s.is_empty() || !s.bytes().any(|c| c.is_ascii_digit()) {
                    return Err(Error::msg("Invalid number"));
                }
                match s.parse::<i128>() {
                    Ok(v) => Self::try_from_i128(v),
                    Err(_) => s
                        .parse::<$prim>()
                        .map($name)
                        .map_err(|_| Error::msg(format!("Invalid number {} for {}", s, $abi))),
                }
            }
        }

        impl ABISerializable for $name {
            fn abi_type() -> String {
                $abi.into()
            }
            fn to_abi(&self, encoder: &mut ABIEncoder) {
                encoder.write_array(&self.0.to_le_bytes());
            }
            fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
                Ok($name(<$prim>::from_le_bytes(decoder.read_fixed()?)))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
                if self.magnitude_fits_32() {
                    s.serialize_i64(self.0 as i64)
                } else {
                    s.collect_str(&self.0)
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
                struct V;
                impl<'de> serde::de::Visitor<'de> for V {
                    type Value = $name;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str(concat!("an integer or numeric string for ", $abi))
                    }
                    fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<$name, E> {
                        $name::try_from_i128(v as i128).map_err(E::custom)
                    }
                    fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<$name, E> {
                        $name::try_from_i128(v as i128).map_err(E::custom)
                    }
                    fn visit_i128<E: serde::de::Error>(self, v: i128) -> std::result::Result<$name, E> {
                        $name::try_from_i128(v).map_err(E::custom)
                    }
                    fn visit_u128<E: serde::de::Error>(self, v: u128) -> std::result::Result<$name, E> {
                        <$prim>::try_from(v)
                            .map($name)
                            .map_err(|_| E::custom(format!("Number {} overflows {}", v, $abi)))
                    }
                    fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<$name, E> {
                        if !v.is_finite() || v.fract() != 0.0 {
                            return Err(E::custom("Invalid number"));
                        }
                        $name::try_from_i128(v as i128).map_err(E::custom)
                    }
                    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<$name, E> {
                        v.parse().map_err(E::custom)
                    }
                    fn visit_bool<E: serde::de::Error>(self, v: bool) -> std::result::Result<$name, E> {
                        Ok($name(v as $prim))
                    }
                }
                d.deserialize_any(V)
            }
        }

        impl std::ops::Add for $name {
            type Output = $name;
            fn add(self, rhs: $name) -> $name {
                $name(self.0.wrapping_add(rhs.0))
            }
        }
        impl std::ops::Sub for $name {
            type Output = $name;
            fn sub(self, rhs: $name) -> $name {
                $name(self.0.wrapping_sub(rhs.0))
            }
        }
        impl std::ops::Mul for $name {
            type Output = $name;
            fn mul(self, rhs: $name) -> $name {
                $name(self.0.wrapping_mul(rhs.0))
            }
        }
        /// Truncating division, panics on division by zero like the primitive.
        impl std::ops::Div for $name {
            type Output = $name;
            fn div(self, rhs: $name) -> $name {
                $name(self.0.wrapping_div(rhs.0))
            }
        }
        impl std::ops::Rem for $name {
            type Output = $name;
            fn rem(self, rhs: $name) -> $name {
                $name(self.0.wrapping_rem(rhs.0))
            }
        }
        impl std::ops::AddAssign for $name {
            fn add_assign(&mut self, rhs: $name) {
                *self = *self + rhs;
            }
        }
        impl std::ops::SubAssign for $name {
            fn sub_assign(&mut self, rhs: $name) {
                *self = *self - rhs;
            }
        }
        impl std::ops::MulAssign for $name {
            fn mul_assign(&mut self, rhs: $name) {
                *self = *self * rhs;
            }
        }
        impl std::ops::DivAssign for $name {
            fn div_assign(&mut self, rhs: $name) {
                *self = *self / rhs;
            }
        }
        impl PartialEq<$prim> for $name {
            fn eq(&self, other: &$prim) -> bool {
                self.0 == *other
            }
        }
    };
}

int_type!(
    /// Signed 8-bit integer (`int8`).
    Int8, i8, "int8", true
);
int_type!(
    /// Signed 16-bit integer (`int16`).
    Int16, i16, "int16", true
);
int_type!(
    /// Signed 32-bit integer (`int32`).
    Int32, i32, "int32", true
);
int_type!(
    /// Signed 64-bit integer (`int64`).
    Int64, i64, "int64", true
);
int_type!(
    /// Signed 128-bit integer (`int128`).
    Int128, i128, "int128", true
);
int_type!(
    /// Unsigned 8-bit integer (`uint8`).
    UInt8, u8, "uint8", false
);
int_type!(
    /// Unsigned 16-bit integer (`uint16`).
    UInt16, u16, "uint16", false
);
int_type!(
    /// Unsigned 32-bit integer (`uint32`).
    UInt32, u32, "uint32", false
);
int_type!(
    /// Unsigned 64-bit integer (`uint64`).
    UInt64, u64, "uint64", false
);
int_type!(
    /// Unsigned 128-bit integer (`uint128`).
    UInt128, u128, "uint128", false
);

macro_rules! var_int_type {
    ($(#[$meta:meta])* $name:ident, $prim:ty, $abi:literal, $write:ident, $read:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(pub $prim);

        impl $name {
            pub const fn new(value: $prim) -> Self {
                $name(value)
            }
            pub const fn value(self) -> $prim {
                self.0
            }
        }

        impl From<$prim> for $name {
            fn from(v: $prim) -> Self {
                $name(v)
            }
        }

        impl From<$name> for $prim {
            fn from(v: $name) -> Self {
                v.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl FromStr for $name {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self> {
                s.trim()
                    .parse::<$prim>()
                    .map($name)
                    .map_err(|_| Error::msg(format!("Invalid number {} for {}", s, $abi)))
            }
        }

        impl ABISerializable for $name {
            fn abi_type() -> String {
                $abi.into()
            }
            fn to_abi(&self, encoder: &mut ABIEncoder) {
                encoder.$write(self.0);
            }
            fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
                Ok($name(decoder.$read()?))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
                s.serialize_i64(self.0 as i64)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
                // reuse the 64-bit parsing logic and range check
                let v = Int64::deserialize(d)?;
                <$prim>::try_from(v.0)
                    .map($name)
                    .map_err(|_| serde::de::Error::custom(format!("Number {} overflows {}", v, $abi)))
            }
        }
    };
}

var_int_type!(
    /// Variable length unsigned 32-bit integer (`varuint32`).
    VarUInt, u32, "varuint32", write_varuint32, read_varuint32
);
var_int_type!(
    /// Variable length signed 32-bit integer (`varint32`), zigzag encoded.
    VarInt, i32, "varint32", write_varint32, read_varint32
);

/// Alias for permission weights (`weight_type`).
pub type Weight = UInt16;
