use std::fmt;
use std::str::FromStr;

use super::{Int64, Name};
use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::{Error, Result};

/// Token amount with symbol (`asset`), e.g. `1.0000 VEX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Asset {
    pub units: Int64,
    pub symbol: Symbol,
}

impl Asset {
    pub fn new(units: impl Into<Int64>, symbol: Symbol) -> Self {
        Asset {
            units: units.into(),
            symbol,
        }
    }

    /// Parse from string, e.g. `1.0000 VEX`.
    pub fn from_string(value: &str) -> Result<Self> {
        let parts: Vec<&str> = value.split(' ').collect();
        if parts.len() != 2 {
            return Err(Error::msg("Invalid asset string"));
        }
        let amount = parts[0].replacen('.', "", 1);
        let precision = parts[0].split('.').nth(1).map(|s| s.len()).unwrap_or(0);
        if precision > Symbol::MAX_PRECISION as usize {
            return Err(Error::msg("Invalid asset symbol, precision too large"));
        }
        let symbol = Symbol::from_parts(parts[1], precision as u8)?;
        Ok(Asset {
            units: amount.parse()?,
            symbol,
        })
    }

    /// Create from a floating point value, rounded to the symbol precision.
    pub fn from_float(value: f64, symbol: Symbol) -> Result<Self> {
        Ok(Asset {
            units: symbol.convert_float(value)?,
            symbol,
        })
    }

    /// Create from raw units.
    pub fn from_units(units: impl Into<Int64>, symbol: Symbol) -> Self {
        Self::new(units, symbol)
    }

    /// Format units with given precision, e.g. `(12345, 4) -> "1.2345"`.
    pub fn format_units(units: Int64, precision: u8) -> String {
        let s = units.0.to_string();
        let (negative, digits) = match s.strip_prefix('-') {
            Some(d) => (true, d.to_string()),
            None => (false, s),
        };
        let precision = precision as usize;
        let mut digits = digits;
        while digits.len() <= precision {
            digits.insert(0, '0');
        }
        if precision > 0 {
            digits.insert(digits.len() - precision, '.');
        }
        if negative {
            digits.insert(0, '-');
        }
        digits
    }

    /// The floating point value of this asset.
    pub fn value(&self) -> f64 {
        self.symbol.convert_units(self.units)
    }

    /// Set the floating point value of this asset.
    pub fn set_value(&mut self, value: f64) -> Result<()> {
        self.units = self.symbol.convert_float(value)?;
        Ok(())
    }

    /// Quantity as string without symbol, e.g. `1.0000`.
    pub fn quantity(&self) -> String {
        Self::format_units(self.units, self.symbol.precision())
    }
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.quantity(), self.symbol.name())
    }
}

impl FromStr for Asset {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        Asset::from_string(s)
    }
}

crate::utils::serde_helpers::serde_via_string!(Asset);

impl ABISerializable for Asset {
    fn abi_type() -> String {
        "asset".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        self.units.to_abi(encoder);
        self.symbol.to_abi(encoder);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let units = Int64::from_abi(decoder)?;
        let symbol = Symbol::from_abi(decoder)?;
        Ok(Asset { units, symbol })
    }
}

/// Asset symbol (`symbol`), precision + code, e.g. `4,VEX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(u64);

impl Symbol {
    pub const MAX_PRECISION: u8 = 18;

    /// Create from the raw value, validates precision and name.
    pub fn from_raw(value: u64) -> Result<Self> {
        if to_symbol_precision(value) > Self::MAX_PRECISION {
            return Err(Error::msg("Invalid asset symbol, precision too large"));
        }
        if value != 0 && !is_valid_symbol_name(&to_symbol_name(value)) {
            return Err(Error::msg("Invalid asset symbol, name must be uppercase A-Z"));
        }
        Ok(Symbol(value))
    }

    /// Create from name and precision.
    pub fn from_parts(name: &str, precision: u8) -> Result<Self> {
        Self::from_raw(to_raw_symbol(name, precision))
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn name(&self) -> String {
        to_symbol_name(self.0)
    }

    pub fn precision(&self) -> u8 {
        to_symbol_precision(self.0)
    }

    pub fn code(&self) -> SymbolCode {
        SymbolCode(self.0 >> 8)
    }

    /// Convert units to floating point number according to symbol precision.
    pub fn convert_units(&self, units: Int64) -> f64 {
        units.0 as f64 / 10f64.powi(self.precision() as i32)
    }

    /// Convert floating point to units according to symbol precision,
    /// the value is rounded to the closest precision.
    pub fn convert_float(&self, float: f64) -> Result<Int64> {
        if !float.is_finite() {
            return Err(Error::msg("Invalid number"));
        }
        let s = format!("{:.*}", self.precision() as usize, float);
        s.replacen('.', "", 1).parse()
    }
}

impl Default for Symbol {
    /// `4,SYS` (`CORE_SYMBOL`)
    fn default() -> Self {
        Symbol::from_parts("SYS", 4).expect("valid symbol")
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.precision(), self.name())
    }
}

impl FromStr for Symbol {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self> {
        let parts: Vec<&str> = value.split(',').collect();
        if parts.len() != 2 {
            return Err(Error::msg("Invalid symbol string"));
        }
        let precision: u8 = parts[0]
            .trim()
            .parse()
            .map_err(|_| Error::msg("Invalid symbol precision"))?;
        Symbol::from_parts(parts[1], precision)
    }
}

crate::utils::serde_helpers::serde_via_string!(Symbol);

impl ABISerializable for Symbol {
    fn abi_type() -> String {
        "symbol".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0.to_le_bytes());
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Symbol::from_raw(u64::from_le_bytes(decoder.read_fixed()?))
    }
}

/// Asset symbol code (`symbol_code`), e.g. `VEX`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolCode(u64);

impl SymbolCode {
    pub fn from_raw(value: u64) -> Result<Self> {
        if value != 0 && !is_valid_symbol_name(&chars_to_symbol_name(value)) {
            return Err(Error::msg("Invalid asset symbol, name must be uppercase A-Z"));
        }
        Ok(SymbolCode(value))
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Default for SymbolCode {
    /// `SYS` (`CORE_SYMBOL_NAME`)
    fn default() -> Self {
        "SYS".parse().expect("valid symbol code")
    }
}

impl fmt::Display for SymbolCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&chars_to_symbol_name(self.0))
    }
}

impl FromStr for SymbolCode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        let code = to_raw_symbol_code(s);
        let mut bytes = [0u8; 8];
        bytes[..code.len()].copy_from_slice(&code);
        SymbolCode::from_raw(u64::from_le_bytes(bytes))
    }
}

crate::utils::serde_helpers::serde_via_string!(SymbolCode);

impl ABISerializable for SymbolCode {
    fn abi_type() -> String {
        "symbol_code".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_array(&self.0.to_le_bytes());
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        SymbolCode::from_raw(u64::from_le_bytes(decoder.read_fixed()?))
    }
}

crate::abi_struct! {
    /// Asset with token contract (`extended_asset`).
    pub struct ExtendedAsset("extended_asset") {
        pub quantity: Asset,
        pub contract: Name,
    }
}

crate::abi_struct! {
    /// Symbol with token contract (`extended_symbol`).
    pub struct ExtendedSymbol("extended_symbol") {
        pub sym: Symbol,
        pub contract: Name,
    }
}

fn is_valid_symbol_name(name: &str) -> bool {
    name.len() <= 7 && name.bytes().all(|c| c.is_ascii_uppercase())
}

fn to_symbol_precision(raw: u64) -> u8 {
    (raw & 0xff) as u8
}

fn to_symbol_name(raw: u64) -> String {
    chars_to_symbol_name(raw >> 8)
}

fn chars_to_symbol_name(raw: u64) -> String {
    raw.to_le_bytes()
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as char)
        .collect()
}

fn to_raw_symbol_code(name: &str) -> Vec<u8> {
    name.bytes().take(7).collect()
}

fn to_raw_symbol(name: &str, precision: u8) -> u64 {
    let code = to_raw_symbol_code(name);
    let mut bytes = [0u8; 8];
    bytes[0] = precision;
    bytes[1..1 + code.len()].copy_from_slice(&code);
    u64::from_le_bytes(bytes)
}
