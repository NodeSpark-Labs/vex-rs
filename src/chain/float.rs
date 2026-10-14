use std::fmt;
use std::str::FromStr;

use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::utils::{array_to_hex, hex_to_array, secure_random};
use crate::{Error, Result};

macro_rules! float_type {
    ($(#[$meta:meta])* $name:ident, $prim:ty, $abi:literal, $write:ident, $read:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
        pub struct $name(pub $prim);

        impl $name {
            pub const fn value(self) -> $prim {
                self.0
            }

            /// Create a random value from secure random bytes (may be NaN/inf).
            pub fn random() -> Self {
                let bytes = secure_random(std::mem::size_of::<$prim>());
                $name(<$prim>::from_le_bytes(bytes.try_into().expect("size")))
            }
        }

        impl From<$prim> for $name {
            fn from(v: $prim) -> Self {
                $name(v)
            }
        }

        impl FromStr for $name {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self> {
                s.trim()
                    .parse::<$prim>()
                    .map($name)
                    .map_err(|_| Error::msg(format!("Invalid {}: {}", $abi, s)))
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
                s.collect_str(self)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
                struct V;
                impl<'de> serde::de::Visitor<'de> for V {
                    type Value = $name;
                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str("a number or numeric string")
                    }
                    fn visit_f64<E: serde::de::Error>(self, v: f64) -> std::result::Result<$name, E> {
                        Ok($name(v as $prim))
                    }
                    fn visit_i64<E: serde::de::Error>(self, v: i64) -> std::result::Result<$name, E> {
                        Ok($name(v as $prim))
                    }
                    fn visit_u64<E: serde::de::Error>(self, v: u64) -> std::result::Result<$name, E> {
                        Ok($name(v as $prim))
                    }
                    fn visit_str<E: serde::de::Error>(self, v: &str) -> std::result::Result<$name, E> {
                        v.parse().map_err(E::custom)
                    }
                }
                d.deserialize_any(V)
            }
        }
    };
}

float_type!(
    /// 32-bit float (`float32`), JSON encoded as a string with 7 decimals.
    Float32, f32, "float32", write_f32, read_f32
);
float_type!(
    /// 64-bit float (`float64`), JSON encoded as a string.
    Float64, f64, "float64", write_f64, read_f64
);

impl fmt::Display for Float32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.7}", self.0 as f64)
    }
}

impl fmt::Display for Float64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_infinite() {
            f.write_str(if self.0 > 0.0 {
                "Infinity"
            } else {
                "-Infinity"
            })
        } else {
            fmt::Display::fmt(&self.0, f)
        }
    }
}

/// 128-bit float (`float128`), stored as raw bytes and JSON encoded as `0x` prefixed hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Float128(pub [u8; 16]);

impl Float128 {
    pub fn from_slice(data: &[u8]) -> Result<Self> {
        Ok(Float128(
            data.try_into()
                .map_err(|_| Error::msg("Invalid float128"))?,
        ))
    }

    pub fn random() -> Self {
        Self::from_slice(&secure_random(16)).expect("size")
    }

    pub fn hex_string(&self) -> String {
        array_to_hex(&self.0)
    }
}

impl fmt::Display for Float128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{}", self.hex_string())
    }
}

impl FromStr for Float128 {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        Self::from_slice(&hex_to_array(s)?)
    }
}

crate::utils::serde_helpers::serde_via_string!(Float128);

impl ABISerializable for Float128 {
    fn abi_type() -> String {
        "float128".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(Float128(decoder.read_fixed()?))
    }
}
