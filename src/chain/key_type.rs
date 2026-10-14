use std::fmt;
use std::str::FromStr;

use crate::{Error, Result};

/// Supported Antelope/EOSIO curve types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyType {
    /// secp256k1
    K1,
    /// secp256r1 (NIST P-256)
    R1,
    /// WebAuthn (P-256)
    WA,
}

impl KeyType {
    /// Binary index of the key type.
    pub fn index(&self) -> u8 {
        match self {
            KeyType::K1 => 0,
            KeyType::R1 => 1,
            KeyType::WA => 2,
        }
    }

    pub fn from_index(index: u8) -> Result<Self> {
        match index {
            0 => Ok(KeyType::K1),
            1 => Ok(KeyType::R1),
            2 => Ok(KeyType::WA),
            _ => Err(Error::msg("Unknown curve type")),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            KeyType::K1 => "K1",
            KeyType::R1 => "R1",
            KeyType::WA => "WA",
        }
    }
}

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for KeyType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "K1" => Ok(KeyType::K1),
            "R1" => Ok(KeyType::R1),
            "WA" => Ok(KeyType::WA),
            _ => Err(Error::msg(format!("Unknown curve type: {}", s))),
        }
    }
}

crate::utils::serde_helpers::serde_via_string!(KeyType);
