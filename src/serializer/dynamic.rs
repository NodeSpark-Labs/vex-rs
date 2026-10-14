//! ABI driven encoding/decoding of JSON values.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};

use super::{ABIDecoder, ABIEncoder, ABISerializable};
use crate::chain::abi::ResolvedTypeName;
use crate::chain::*;
use crate::{Error, Result};

const MAX_DEPTH: usize = 32;

/// Options for [`decode_json`].
#[derive(Debug, Clone, Default)]
pub struct DecodeOptions {
    /// Binary extension handling, if set to true missing extensions will be
    /// initialized to their default value, otherwise they will be set to null.
    pub strict_extensions: bool,
    /// Set to ignore invalid UTF-8, otherwise an error is returned (default).
    pub ignore_invalid_utf8: bool,
}

macro_rules! builtins {
    ($m:ident) => {
        $m! {
            "bool" => bool,
            "string" => String,
            "int8" => Int8,
            "int16" => Int16,
            "int32" => Int32,
            "int64" => Int64,
            "int128" => Int128,
            "uint8" => UInt8,
            "uint16" => UInt16,
            "uint32" => UInt32,
            "uint64" => UInt64,
            "uint128" => UInt128,
            "varint32" => VarInt,
            "varuint32" => VarUInt,
            "float32" => Float32,
            "float64" => Float64,
            "float128" => Float128,
            "name" => Name,
            "bytes" => Bytes,
            "checksum160" => Checksum160,
            "checksum256" => Checksum256,
            "checksum512" => Checksum512,
            "public_key" => PublicKey,
            "signature" => Signature,
            "symbol" => Symbol,
            "symbol_code" => SymbolCode,
            "asset" => Asset,
            "extended_asset" => ExtendedAsset,
            "time_point" => TimePoint,
            "time_point_sec" => TimePointSec,
            "block_timestamp_type" => BlockTimestamp,
            "block_id_type" => BlockId,
        }
    };
}

macro_rules! is_builtin_impl {
    ($($name:literal => $t:ty,)*) => {
        /// Returns true if the given (base) type name is a builtin type.
        pub fn is_builtin_type(name: &str) -> bool {
            matches!(name, $($name)|*)
        }
    };
}
builtins!(is_builtin_impl);

fn encode_typed<T: ABISerializable + DeserializeOwned>(
    value: &Value,
    encoder: &mut ABIEncoder,
) -> Result<()> {
    let v: T = T::deserialize(value).map_err(|e| Error::msg(e.to_string()))?;
    v.to_abi(encoder);
    Ok(())
}

fn decode_typed<T: ABISerializable + Serialize>(decoder: &mut ABIDecoder) -> Result<Value> {
    Ok(serde_json::to_value(T::from_abi(decoder)?)?)
}

fn default_typed<T: ABISerializable>() -> Result<Value> {
    Err(Error::msg(format!(
        "Unable to determine default value for {}",
        T::abi_type()
    )))
}

macro_rules! builtin_dispatch {
    ($($name:literal => $t:ty,)*) => {
        fn encode_builtin(name: &str, value: &Value, encoder: &mut ABIEncoder) -> Option<Result<()>> {
            match name {
                $($name => Some(encode_typed::<$t>(value, encoder)),)*
                _ => None,
            }
        }
        fn decode_builtin(name: &str, decoder: &mut ABIDecoder) -> Option<Result<Value>> {
            match name {
                $($name => Some(decode_typed::<$t>(decoder)),)*
                _ => None,
            }
        }
    };
}
builtins!(builtin_dispatch);

fn builtin_default(name: &str) -> Option<Result<Value>> {
    let v = match name {
        "bool" => Value::Bool(false),
        "string" => Value::String(String::new()),
        "int8" | "int16" | "int32" | "int64" | "int128" | "uint8" | "uint16" | "uint32"
        | "uint64" | "uint128" | "varint32" | "varuint32" => Value::from(0),
        "float32" => Value::String(Float32(0.0).to_string()),
        "float64" => Value::String(Float64(0.0).to_string()),
        "float128" => Value::String(Float128::default().to_string()),
        "name" => Value::String(String::new()),
        "bytes" => Value::String(String::new()),
        "checksum160" => Value::String(Checksum160::default().to_string()),
        "checksum256" => Value::String(Checksum256::default().to_string()),
        "checksum512" => Value::String(Checksum512::default().to_string()),
        "block_id_type" => Value::String(BlockId::default().to_string()),
        "symbol" => Value::String(Symbol::default().to_string()),
        "symbol_code" => Value::String(SymbolCode::default().to_string()),
        "asset" => Value::String(Asset::new(0, Symbol::default()).to_string()),
        "time_point" => Value::String(TimePoint(0).to_string()),
        "time_point_sec" => Value::String(TimePointSec(0).to_string()),
        "block_timestamp_type" => Value::String(BlockTimestamp(0).to_string()),
        "public_key" | "signature" | "extended_asset" => return Some(default_typed::<PublicKey>()),
        _ => return None,
    };
    Some(Ok(v))
}

struct Ctx<'a> {
    abi: &'a ABI,
    path: Vec<String>,
    strict_extensions: bool,
}

impl Ctx<'_> {
    fn path_string(&self) -> String {
        self.path.join(".")
    }
}

/// Encode a JSON value according to the given ABI and type name.
pub fn encode_json(abi: &ABI, type_name: &str, value: &Value) -> Result<Bytes> {
    let mut encoder = ABIEncoder::new();
    let mut ctx = Ctx {
        abi,
        path: vec![format!("root<{}>", type_name)],
        strict_extensions: false,
    };
    encode_any(value, type_name, &mut encoder, &mut ctx).map_err(|e| match e {
        Error::Msg(message) => Error::Encoding {
            path: ctx.path_string(),
            message,
        },
        other => other,
    })?;
    Ok(Bytes(encoder.into_data()))
}

fn encode_any(
    value: &Value,
    type_name: &str,
    encoder: &mut ABIEncoder,
    ctx: &mut Ctx,
) -> Result<()> {
    if ctx.path.len() > MAX_DEPTH * 2 {
        return Err(Error::msg("Maximum encoding depth exceeded"));
    }
    let t = ResolvedTypeName::parse(type_name);
    let exists = !value.is_null();
    if t.is_optional {
        encoder.write_byte(exists as u8);
        if !exists {
            return Ok(());
        }
    }
    if !exists {
        // an alias may resolve to an optional type
        if !t.is_array && !is_builtin_type(&t.name) {
            if let Some(alias) = ctx.abi.get_alias(&t.name) {
                return encode_any(value, &alias.r#type, encoder, ctx);
            }
        }
        if t.is_extension {
            return Ok(());
        }
        return Err(Error::msg(format!(
            "Found null for non-optional type: {}",
            t.type_name()
        )));
    }
    if t.is_array {
        let items = value
            .as_array()
            .ok_or_else(|| Error::msg(format!("Expected array for: {}", t.type_name())))?;
        match t.size {
            Some(size) if size != items.len() => {
                return Err(Error::msg(format!(
                    "Expected array of size {} for: {}",
                    size,
                    t.type_name()
                )))
            }
            Some(_) => {}
            None => encoder.write_varuint32(items.len() as u32),
        }
        for (i, item) in items.iter().enumerate() {
            ctx.path.push(i.to_string());
            encode_inner(item, &t.name, encoder, ctx)?;
            ctx.path.pop();
        }
        Ok(())
    } else {
        encode_inner(value, &t.name, encoder, ctx)
    }
}

fn encode_inner(value: &Value, name: &str, encoder: &mut ABIEncoder, ctx: &mut Ctx) -> Result<()> {
    if let Some(result) = encode_builtin(name, value, encoder) {
        return result;
    }
    let abi = ctx.abi;
    if let Some(alias) = abi.get_alias(name) {
        return encode_any(value, &alias.r#type, encoder, ctx);
    }
    if abi.get_struct(name).is_some() {
        let object = value
            .as_object()
            .ok_or_else(|| Error::msg(format!("Expected object for: {}", name)))?;
        for field in abi.all_fields(name)? {
            ctx.path.push(format!("{}<{}>", field.name, field.r#type));
            let v = object.get(&field.name).unwrap_or(&Value::Null);
            encode_any(v, &field.r#type, encoder, ctx)?;
            ctx.path.pop();
        }
        return Ok(());
    }
    if let Some(variant) = abi.get_variant(name) {
        let (v_name, v_value) = match value {
            Value::Array(items) if items.len() == 2 && items[0].is_string() => {
                (items[0].as_str().unwrap().to_string(), &items[1])
            }
            Value::Object(o) if o.contains_key("type") && o.contains_key("value") => (
                o["type"].as_str().unwrap_or_default().to_string(),
                &o["value"],
            ),
            Value::Bool(_) => ("bool".to_string(), value),
            Value::String(_) => ("string".to_string(), value),
            _ => {
                return Err(Error::msg(format!(
                    "Unable to determine variant type for {}, use [\"type\", value]",
                    name
                )))
            }
        };
        let idx = variant
            .types
            .iter()
            .position(|t| *t == v_name)
            .ok_or_else(|| {
                let types: Vec<String> = variant.types.iter().map(|t| format!("'{}'", t)).collect();
                Error::msg(format!(
                    "Unknown variant type '{}', expected one of {}",
                    v_name,
                    types.join(", ")
                ))
            })?;
        encoder.write_varuint32(idx as u32);
        ctx.path.push(format!("v{}", idx));
        encode_any(v_value, &v_name, encoder, ctx)?;
        ctx.path.pop();
        return Ok(());
    }
    Err(Error::msg(if name == "any" {
        "Unable to encode any type to binary".to_string()
    } else {
        format!("Unknown type: {}", name)
    }))
}

/// Decode binary data to a JSON value according to the given ABI and type name.
/// Structs decode to JSON objects (in field order), variants to `["type", value]`.
pub fn decode_json(
    abi: &ABI,
    type_name: &str,
    data: &[u8],
    options: &DecodeOptions,
) -> Result<Value> {
    let mut decoder = ABIDecoder::new(data);
    decoder.ignore_invalid_utf8 = options.ignore_invalid_utf8;
    let mut ctx = Ctx {
        abi,
        path: vec![format!("root<{}>", type_name)],
        strict_extensions: options.strict_extensions,
    };
    decode_any(type_name, &mut decoder, &mut ctx).map_err(|e| match e {
        Error::Msg(message) => Error::Decoding {
            path: ctx.path_string(),
            message,
        },
        other => other,
    })
}

fn decode_any(type_name: &str, decoder: &mut ABIDecoder, ctx: &mut Ctx) -> Result<Value> {
    if ctx.path.len() > MAX_DEPTH {
        return Err(Error::msg("Maximum decoding depth exceeded"));
    }
    let t = ResolvedTypeName::parse(type_name);
    if t.is_extension && !decoder.can_read(1) {
        return if ctx.strict_extensions {
            default_value(type_name, ctx, &mut vec![])
        } else {
            Ok(Value::Null)
        };
    }
    if t.is_optional && decoder.read_byte()? == 0 {
        return Ok(Value::Null);
    }
    if t.is_array {
        let len = match t.size {
            Some(size) => size,
            None => decoder.read_varuint32()? as usize,
        };
        let mut rv = Vec::with_capacity(len.min(decoder.remaining()));
        for i in 0..len {
            ctx.path.push(i.to_string());
            rv.push(decode_inner(&t.name, decoder, ctx)?);
            ctx.path.pop();
        }
        Ok(Value::Array(rv))
    } else {
        decode_inner(&t.name, decoder, ctx)
    }
}

fn decode_inner(name: &str, decoder: &mut ABIDecoder, ctx: &mut Ctx) -> Result<Value> {
    if let Some(result) = decode_builtin(name, decoder) {
        return result;
    }
    let abi = ctx.abi;
    if let Some(alias) = abi.get_alias(name) {
        ctx.path.push(format!("<{}>", alias.r#type));
        let rv = decode_any(&alias.r#type, decoder, ctx)?;
        ctx.path.pop();
        return Ok(rv);
    }
    if abi.get_struct(name).is_some() {
        let mut rv = Map::new();
        for field in abi.all_fields(name)? {
            ctx.path.push(format!("{}<{}>", field.name, field.r#type));
            let v = decode_any(&field.r#type, decoder, ctx)?;
            rv.insert(field.name.clone(), v);
            ctx.path.pop();
        }
        return Ok(Value::Object(rv));
    }
    if let Some(variant) = abi.get_variant(name) {
        let idx = decoder.read_varuint32()? as usize;
        let v_type = variant
            .types
            .get(idx)
            .ok_or_else(|| Error::msg(format!("Unknown variant idx: {}", idx)))?
            .clone();
        ctx.path.push(format!("v{}", idx));
        let v = decode_any(&v_type, decoder, ctx)?;
        ctx.path.pop();
        return Ok(Value::Array(vec![Value::String(v_type), v]));
    }
    Err(Error::msg(if name == "any" {
        "Unable to decode 'any' type from binary".to_string()
    } else {
        format!("Unknown type: {}", name)
    }))
}

/// Default value (aka initialized value, matching C++ where possible) for given type.
fn default_value(type_name: &str, ctx: &mut Ctx, seen: &mut Vec<String>) -> Result<Value> {
    let t = ResolvedTypeName::parse(type_name);
    if t.is_array {
        return Ok(Value::Array(vec![]));
    }
    if t.is_optional {
        return Ok(Value::Null);
    }
    if let Some(v) = builtin_default(&t.name) {
        return v;
    }
    if seen.contains(&t.name) {
        return Err(Error::msg("Circular type reference"));
    }
    seen.push(t.name.clone());
    let abi = ctx.abi;
    if let Some(alias) = abi.get_alias(&t.name) {
        return default_value(&alias.r#type, ctx, seen);
    }
    if abi.get_struct(&t.name).is_some() {
        let mut rv = Map::new();
        for field in abi.all_fields(&t.name)? {
            rv.insert(field.name.clone(), default_value(&field.r#type, ctx, seen)?);
        }
        return Ok(Value::Object(rv));
    }
    if let Some(variant) = abi.get_variant(&t.name) {
        if let Some(first) = variant.types.first() {
            let first = first.clone();
            let v = default_value(&first, ctx, &mut vec![])?;
            return Ok(Value::Array(vec![Value::String(first), v]));
        }
    }
    Err(Error::msg("Unable to determine default value"))
}
