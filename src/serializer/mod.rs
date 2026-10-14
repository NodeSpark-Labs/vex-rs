//! ABI serialization.
//!
//! There are two ways of serializing data:
//!
//! * **Typed**: any type implementing [`ABISerializable`] (all core types, the structs
//!   defined with [`abi_struct!`](crate::abi_struct) and the std containers `Vec<T>`,
//!   `Option<T>`, `[T; N]`, `String` and `bool`) can be encoded with
//!   [`Serializer::encode`] and decoded with [`Serializer::decode`].
//! * **Dynamic**: arbitrary JSON values can be encoded/decoded according to an
//!   [`ABI`](crate::chain::ABI) definition with [`Serializer::encode_json`] and
//!   [`Serializer::decode_json`].

mod decoder;
mod dynamic;
mod encoder;
mod serializable;

pub use decoder::ABIDecoder;
pub use dynamic::{decode_json, encode_json, is_builtin_type, DecodeOptions};
pub use encoder::ABIEncoder;
#[doc(hidden)]
pub use serializable::__field_name;
pub use serializable::{ABISerializable, AbiBuilder, BinaryExtension};

use crate::chain::{Bytes, ABI};
use crate::Result;

/// Serializer helpers, mirrors the `Serializer` namespace of the JavaScript library.
pub struct Serializer;

impl Serializer {
    /// ABI encode a typed value.
    pub fn encode<T: ABISerializable>(object: &T) -> Bytes {
        let mut encoder = ABIEncoder::new();
        object.to_abi(&mut encoder);
        Bytes::from(encoder.into_data())
    }

    /// ABI decode a typed value from binary data.
    pub fn decode<T: ABISerializable>(data: impl AsRef<[u8]>) -> Result<T> {
        let mut decoder = ABIDecoder::new(data.as_ref());
        T::from_abi(&mut decoder).map_err(|e| crate::Error::Decoding {
            path: format!("root<{}>", T::abi_type()),
            message: e.to_string(),
        })
    }

    /// Encode a JSON value according to the given ABI and type name.
    pub fn encode_json(abi: &ABI, type_name: &str, value: &serde_json::Value) -> Result<Bytes> {
        encode_json(abi, type_name, value)
    }

    /// Decode binary data to a JSON value according to the given ABI and type name.
    pub fn decode_json(
        abi: &ABI,
        type_name: &str,
        data: impl AsRef<[u8]>,
    ) -> Result<serde_json::Value> {
        decode_json(abi, type_name, data.as_ref(), &DecodeOptions::default())
    }

    /// Create an Antelope/EOSIO ABI definition for given core type.
    pub fn synthesize<T: ABISerializable>() -> ABI {
        let mut builder = AbiBuilder::default();
        T::synthesize(&mut builder);
        builder.into_abi()
    }

    /// Create JSON representation of a core object.
    pub fn stringify<T: serde::Serialize + ?Sized>(object: &T) -> Result<String> {
        Ok(serde_json::to_string(object)?)
    }

    /// Create a plain JSON value representation of a core object.
    pub fn objectify<T: serde::Serialize + ?Sized>(object: &T) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(object)?)
    }
}
