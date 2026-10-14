//! Misc helpers.

use crate::{Error, Result};

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Encode bytes as lowercase hex.
pub fn array_to_hex(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for b in data {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Decode a hex string (case-insensitive) to bytes.
pub fn hex_to_array(hex: &str) -> Result<Vec<u8>> {
    let bytes = hex.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err(Error::msg("Odd number of hex digits"));
    }
    bytes
        .chunks(2)
        .map(|pair| match (hex_val(pair[0]), hex_val(pair[1])) {
            (Some(h), Some(l)) => Ok((h << 4) | l),
            _ => Err(Error::msg("Expected hex string")),
        })
        .collect()
}

/// Generate N random bytes using the OS secure random source.
pub fn secure_random(length: usize) -> Vec<u8> {
    use rand_core::{OsRng, RngCore};
    let mut buf = vec![0u8; length];
    OsRng.fill_bytes(&mut buf);
    buf
}

/// Serde helpers shared by the core types.
pub(crate) mod serde_helpers {
    /// Implement `Serialize`/`Deserialize` via `Display`/`FromStr`.
    macro_rules! serde_via_string {
        ($t:ty) => {
            impl serde::Serialize for $t {
                fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                    s.collect_str(self)
                }
            }
            impl<'de> serde::Deserialize<'de> for $t {
                fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                    let s = <std::borrow::Cow<'de, str> as serde::Deserialize>::deserialize(d)?;
                    s.parse::<$t>().map_err(serde::de::Error::custom)
                }
            }
        };
    }
    pub(crate) use serde_via_string;
}
