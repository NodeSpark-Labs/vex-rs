use std::fmt;
use std::str::FromStr;

use ripemd::Ripemd160;
use sha2::{Digest, Sha256, Sha512};

use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::utils::{array_to_hex, hex_to_array};
use crate::{Error, Result};

macro_rules! checksum_type {
    ($(#[$meta:meta])* $name:ident, $size:literal, $abi:literal, $hasher:ty) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub [u8; $size]);

        impl $name {
            pub const BYTE_SIZE: usize = $size;

            /// Create from a byte slice, must be exactly the checksum size.
            pub fn from_slice(data: &[u8]) -> Result<Self> {
                let arr: [u8; $size] = data.try_into().map_err(|_| {
                    Error::msg(format!(
                        "Checksum size mismatch, expected {} bytes got {}",
                        $size,
                        data.len()
                    ))
                })?;
                Ok($name(arr))
            }

            /// Create from a hex string.
            pub fn from_hex(hex: &str) -> Result<Self> {
                Self::from_slice(&hex_to_array(hex)?)
            }

            /// Hash given data.
            pub fn hash(data: impl AsRef<[u8]>) -> Self {
                let digest = <$hasher>::digest(data.as_ref());
                let mut arr = [0u8; $size];
                arr.copy_from_slice(&digest);
                $name(arr)
            }

            pub fn array(&self) -> &[u8; $size] {
                &self.0
            }

            pub fn hex_string(&self) -> String {
                array_to_hex(&self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                $name([0u8; $size])
            }
        }

        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl From<[u8; $size]> for $name {
            fn from(v: [u8; $size]) -> Self {
                $name(v)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.hex_string())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.hex_string())
            }
        }

        impl FromStr for $name {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self> {
                Self::from_hex(s)
            }
        }

        crate::utils::serde_helpers::serde_via_string!($name);

        impl ABISerializable for $name {
            fn abi_type() -> String {
                $abi.into()
            }
            fn to_abi(&self, encoder: &mut ABIEncoder) {
                encoder.write_array(&self.0);
            }
            fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
                Ok($name(decoder.read_fixed()?))
            }
        }
    };
}

checksum_type!(
    /// 160-bit checksum (`checksum160`), RIPEMD-160.
    Checksum160, 20, "checksum160", Ripemd160
);
checksum_type!(
    /// 256-bit checksum (`checksum256`), SHA-256.
    Checksum256, 32, "checksum256", Sha256
);
checksum_type!(
    /// 512-bit checksum (`checksum512`), SHA-512.
    Checksum512, 64, "checksum512", Sha512
);
