use std::fmt;
use std::str::FromStr;

use super::UInt64;
use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::{Error, Result};

/// Antelope/EOSIO Name (`name`), a base32 encoded 64-bit integer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Name(pub u64);

impl Name {
    /// Create a name from a string. Like the reference implementation this is
    /// lenient: invalid characters map to `.`, use [`Name::is_valid`] to check.
    pub fn new(s: &str) -> Self {
        Name(string_to_name(s))
    }

    /// Returns true if the string matches the Antelope name pattern `^[a-z1-5.]{0,13}$`
    /// (the 13th character, if present, must be in `[a-j1-5.]`).
    pub fn is_valid(s: &str) -> bool {
        let bytes = s.as_bytes();
        if bytes.len() > 13 {
            return false;
        }
        bytes.iter().enumerate().all(|(i, &c)| match c {
            b'.' | b'1'..=b'5' => true,
            b'a'..=b'z' => i < 12 || c <= b'j',
            _ => false,
        })
    }

    /// The numeric representation of the name.
    pub const fn value(&self) -> UInt64 {
        UInt64(self.0)
    }

    pub const fn from_u64(value: u64) -> Self {
        Name(value)
    }
}

impl From<u64> for Name {
    fn from(v: u64) -> Self {
        Name(v)
    }
}

impl From<UInt64> for Name {
    fn from(v: UInt64) -> Self {
        Name(v.0)
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Name::new(s)
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        *self == Name::new(other)
    }
}

impl FromStr for Name {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(Name::new(s))
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&name_to_string(self.0))
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Name({})", self)
    }
}

crate::utils::serde_helpers::serde_via_string!(Name);

impl ABISerializable for Name {
    fn abi_type() -> String {
        "name".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0.to_le_bytes());
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(Name(u64::from_le_bytes(decoder.read_fixed()?)))
    }
}

fn char_to_symbol(c: u8) -> u64 {
    match c {
        b'a'..=b'z' => (c - b'a') as u64 + 6,
        b'1'..=b'5' => (c - b'1') as u64 + 1,
        _ => 0,
    }
}

fn string_to_name(s: &str) -> u64 {
    let mut value: u64 = 0;
    let mut bit: i32 = 63;
    for c in s.bytes() {
        let mut c = char_to_symbol(c);
        if bit < 5 {
            c <<= 1;
        }
        for j in (0..=4).rev() {
            if bit >= 0 {
                value |= ((c >> j) & 1) << bit;
                bit -= 1;
            }
        }
    }
    value
}

fn name_to_string(n: u64) -> String {
    let mut result = String::with_capacity(13);
    let mut bit: i32 = 63;
    while bit >= 0 {
        let mut c: u64 = 0;
        for _ in 0..5 {
            if bit >= 0 {
                c = (c << 1) | ((n >> bit) & 1);
                bit -= 1;
            }
        }
        let ch = if c >= 6 {
            (c as u8 + b'a' - 6) as char
        } else if c >= 1 {
            (c as u8 + b'1' - 1) as char
        } else {
            '.'
        };
        result.push(ch);
    }
    while result.ends_with('.') {
        result.pop();
    }
    result
}
