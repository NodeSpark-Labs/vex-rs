use std::fmt;
use std::str::FromStr;

use super::{Checksum256, UInt32};
use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::utils::{array_to_hex, hex_to_array};
use crate::{Error, Result};

/// Block id (`block_id_type`), a sha256 checksum with the block number in the first 4 bytes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BlockId(pub [u8; 32]);

impl BlockId {
    pub fn from_slice(data: &[u8]) -> Result<Self> {
        let arr: [u8; 32] = data.try_into().map_err(|_| {
            Error::msg(format!(
                "BlockId size mismatch, expected 32 bytes got {}",
                data.len()
            ))
        })?;
        Ok(BlockId(arr))
    }

    pub fn from_hex(hex: &str) -> Result<Self> {
        Self::from_slice(&hex_to_array(hex)?)
    }

    /// Create a block id from a block checksum and block number.
    pub fn from_block_checksum(checksum: &Checksum256, block_num: u32) -> Self {
        let mut id = checksum.0;
        id[..4].copy_from_slice(&block_num.to_be_bytes());
        BlockId(id)
    }

    pub fn array(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn hex_string(&self) -> String {
        array_to_hex(&self.0)
    }

    /// The block number encoded in this id.
    pub fn block_num(&self) -> UInt32 {
        UInt32(u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]]))
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex_string())
    }
}

impl fmt::Debug for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlockId({})", self.hex_string())
    }
}

impl FromStr for BlockId {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Self::from_hex(s)
    }
}

impl From<Checksum256> for BlockId {
    fn from(c: Checksum256) -> Self {
        BlockId(c.0)
    }
}

crate::utils::serde_helpers::serde_via_string!(BlockId);

impl ABISerializable for BlockId {
    fn abi_type() -> String {
        "block_id_type".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(BlockId(decoder.read_fixed()?))
    }
}
