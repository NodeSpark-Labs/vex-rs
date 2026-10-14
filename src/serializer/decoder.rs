//! Antelope/EOSIO ABI binary decoder.

use crate::{Error, Result};

/// Binary ABI decoder, reads little-endian data from a byte slice.
#[derive(Debug, Clone)]
pub struct ABIDecoder<'a> {
    array: &'a [u8],
    pos: usize,
    /// Set to ignore invalid UTF-8 (replaced with U+FFFD), otherwise an error is returned (default).
    pub ignore_invalid_utf8: bool,
}

impl<'a> ABIDecoder<'a> {
    pub fn new(array: &'a [u8]) -> Self {
        ABIDecoder {
            array,
            pos: 0,
            ignore_invalid_utf8: false,
        }
    }

    /// Returns true if `bytes` more bytes can be read.
    pub fn can_read(&self, bytes: usize) -> bool {
        self.pos + bytes <= self.array.len()
    }

    fn ensure(&self, bytes: usize) -> Result<()> {
        if !self.can_read(bytes) {
            return Err(Error::msg("Read past end of buffer"));
        }
        Ok(())
    }

    pub fn set_position(&mut self, pos: usize) -> Result<()> {
        if pos > self.array.len() {
            return Err(Error::msg("Invalid position"));
        }
        self.pos = pos;
        Ok(())
    }

    pub fn get_position(&self) -> usize {
        self.pos
    }

    /// Number of bytes left to read.
    pub fn remaining(&self) -> usize {
        self.array.len() - self.pos
    }

    pub fn advance(&mut self, bytes: usize) -> Result<()> {
        self.ensure(bytes)?;
        self.pos += bytes;
        Ok(())
    }

    /// Read one byte.
    pub fn read_byte(&mut self) -> Result<u8> {
        self.ensure(1)?;
        let b = self.array[self.pos];
        self.pos += 1;
        Ok(b)
    }

    /// Read `length` bytes.
    pub fn read_array(&mut self, length: usize) -> Result<&'a [u8]> {
        self.ensure(length)?;
        let rv = &self.array[self.pos..self.pos + length];
        self.pos += length;
        Ok(rv)
    }

    /// Read exactly N bytes into a fixed size array.
    pub fn read_fixed<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut rv = [0u8; N];
        rv.copy_from_slice(self.read_array(N)?);
        Ok(rv)
    }

    pub fn read_f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.read_fixed()?))
    }

    pub fn read_f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.read_fixed()?))
    }

    pub fn read_varuint32(&mut self) -> Result<u32> {
        let mut v: u64 = 0;
        let mut bit = 0u32;
        loop {
            let b = self.read_byte()?;
            if bit < 35 {
                v |= ((b & 0x7f) as u64) << bit;
            }
            bit += 7;
            if b & 0x80 == 0 {
                break;
            }
        }
        Ok(v as u32)
    }

    pub fn read_varint32(&mut self) -> Result<i32> {
        let v = self.read_varuint32()?;
        Ok(((v >> 1) as i32) ^ -((v & 1) as i32))
    }

    /// Read a varuint32 length prefixed UTF-8 string.
    pub fn read_string(&mut self) -> Result<String> {
        let length = self.read_varuint32()? as usize;
        let data = self.read_array(length)?;
        if self.ignore_invalid_utf8 {
            Ok(String::from_utf8_lossy(data).into_owned())
        } else {
            std::str::from_utf8(data)
                .map(|s| s.to_owned())
                .map_err(|_| Error::msg("The encoded data was not valid for encoding utf-8"))
        }
    }
}
