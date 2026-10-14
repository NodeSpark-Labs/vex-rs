use std::collections::HashSet;

use super::{ABIDecoder, ABIEncoder};
use crate::chain::{abi, ABI};
use crate::Result;

/// A self-describing type that can be ABI encoded and decoded.
pub trait ABISerializable: Sized {
    /// ABI type name including suffixes, e.g. `name`, `asset[]` or `transfer`.
    fn abi_type() -> String;

    /// Encode to binary ABI format.
    fn to_abi(&self, encoder: &mut ABIEncoder);

    /// Decode from binary ABI format.
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self>;

    /// Register any struct, variant or alias definitions this type needs.
    /// Builtin types do not need to register anything.
    fn synthesize(_builder: &mut AbiBuilder) {}
}

/// Collects type definitions when synthesizing an ABI for a type.
#[derive(Debug, Default)]
pub struct AbiBuilder {
    seen: HashSet<String>,
    abi: ABI,
}

impl AbiBuilder {
    /// Returns true if the type has not been seen before and marks it as seen.
    pub fn begin(&mut self, name: &str) -> bool {
        self.seen.insert(name.to_string())
    }

    /// Add a struct definition, field names are `(name, type)` pairs.
    pub fn add_struct(&mut self, name: &str, base: &str, fields: Vec<(String, String)>) {
        self.abi.structs.push(abi::Struct {
            name: name.to_string(),
            base: base.to_string(),
            fields: fields
                .into_iter()
                .map(|(name, r#type)| abi::Field { name, r#type })
                .collect(),
        });
    }

    /// Add a variant definition.
    pub fn add_variant(&mut self, name: &str, types: Vec<String>) {
        self.abi.variants.push(abi::Variant {
            name: name.to_string(),
            types,
        });
    }

    /// Add a type alias definition.
    pub fn add_alias(&mut self, new_type_name: &str, r#type: &str) {
        self.abi.types.push(abi::TypeDef {
            new_type_name: new_type_name.to_string(),
            r#type: r#type.to_string(),
        });
    }

    pub fn into_abi(self) -> ABI {
        self.abi
    }
}

impl ABISerializable for bool {
    fn abi_type() -> String {
        "bool".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_byte(*self as u8);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(decoder.read_byte()? == 1)
    }
}

impl ABISerializable for String {
    fn abi_type() -> String {
        "string".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_string(self);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        decoder.read_string()
    }
}

impl<T: ABISerializable> ABISerializable for Vec<T> {
    fn abi_type() -> String {
        format!("{}[]", T::abi_type())
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_varuint32(self.len() as u32);
        for item in self {
            item.to_abi(encoder);
        }
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let len = decoder.read_varuint32()? as usize;
        // guard against bogus lengths causing huge allocations
        let mut rv = Vec::with_capacity(len.min(decoder.remaining()));
        for _ in 0..len {
            rv.push(T::from_abi(decoder)?);
        }
        Ok(rv)
    }
    fn synthesize(builder: &mut AbiBuilder) {
        T::synthesize(builder)
    }
}

impl<T: ABISerializable, const N: usize> ABISerializable for [T; N] {
    fn abi_type() -> String {
        format!("{}[{}]", T::abi_type(), N)
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        for item in self {
            item.to_abi(encoder);
        }
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let mut items = Vec::with_capacity(N);
        for _ in 0..N {
            items.push(T::from_abi(decoder)?);
        }
        items
            .try_into()
            .map_err(|_| crate::Error::msg("Invalid fixed size array"))
    }
    fn synthesize(builder: &mut AbiBuilder) {
        T::synthesize(builder)
    }
}

impl<T: ABISerializable> ABISerializable for Option<T> {
    fn abi_type() -> String {
        format!("{}?", T::abi_type())
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        match self {
            Some(v) => {
                encoder.write_byte(1);
                v.to_abi(encoder);
            }
            None => encoder.write_byte(0),
        }
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        if decoder.read_byte()? == 0 {
            Ok(None)
        } else {
            Ok(Some(T::from_abi(decoder)?))
        }
    }
    fn synthesize(builder: &mut AbiBuilder) {
        T::synthesize(builder)
    }
}

impl<T: ABISerializable> ABISerializable for Box<T> {
    fn abi_type() -> String {
        T::abi_type()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        (**self).to_abi(encoder)
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(Box::new(T::from_abi(decoder)?))
    }
    fn synthesize(builder: &mut AbiBuilder) {
        T::synthesize(builder)
    }
}

/// A binary extension field (`type$`), may be missing at the end of the encoded data.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct BinaryExtension<T>(pub Option<T>);

impl<T> BinaryExtension<T> {
    pub fn new(value: T) -> Self {
        BinaryExtension(Some(value))
    }
    pub fn none() -> Self {
        BinaryExtension(None)
    }
}

impl<T: ABISerializable> ABISerializable for BinaryExtension<T> {
    fn abi_type() -> String {
        format!("{}$", T::abi_type())
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        if let Some(v) = &self.0 {
            v.to_abi(encoder);
        }
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        if decoder.can_read(1) {
            Ok(BinaryExtension(Some(T::from_abi(decoder)?)))
        } else {
            Ok(BinaryExtension(None))
        }
    }
    fn synthesize(builder: &mut AbiBuilder) {
        T::synthesize(builder)
    }
}

#[doc(hidden)]
pub fn __field_name(name: &str) -> String {
    name.trim_start_matches("r#").to_string()
}

/// Define an ABI serializable struct.
///
/// Fields are encoded in declaration order. The generated struct derives
/// `Debug`, `Clone`, `PartialEq`, `serde::Serialize` and `serde::Deserialize`
/// (your crate needs to depend on `serde` with the `derive` feature).
///
/// ```
/// use antelope::{abi_struct, chain::{Asset, Name}, Serializer};
///
/// abi_struct! {
///     pub struct Transfer("transfer") {
///         pub from: Name,
///         pub to: Name,
///         pub quantity: Asset,
///         pub memo: String,
///     }
/// }
///
/// let transfer = Transfer {
///     from: "foo".parse().unwrap(),
///     to: "bar".parse().unwrap(),
///     quantity: "1.0000 VEX".parse().unwrap(),
///     memo: "hello".into(),
/// };
/// let data = Serializer::encode(&transfer);
/// let decoded: Transfer = Serializer::decode(&data).unwrap();
/// assert_eq!(decoded, transfer);
/// ```
#[macro_export]
macro_rules! abi_struct {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ($abi_name:literal) {
            $( $(#[$fmeta:meta])* $fvis:vis $field:ident : $ty:ty ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize)]
        $vis struct $name {
            $( $(#[$fmeta])* $fvis $field : $ty ),*
        }

        impl $crate::serializer::ABISerializable for $name {
            fn abi_type() -> ::std::string::String {
                ::std::string::String::from($abi_name)
            }
            #[allow(unused_variables)]
            fn to_abi(&self, encoder: &mut $crate::serializer::ABIEncoder) {
                $( $crate::serializer::ABISerializable::to_abi(&self.$field, encoder); )*
            }
            #[allow(unused_variables)]
            fn from_abi(decoder: &mut $crate::serializer::ABIDecoder) -> $crate::Result<Self> {
                Ok($name {
                    $( $field: <$ty as $crate::serializer::ABISerializable>::from_abi(decoder)?, )*
                })
            }
            fn synthesize(builder: &mut $crate::serializer::AbiBuilder) {
                if builder.begin($abi_name) {
                    $( <$ty as $crate::serializer::ABISerializable>::synthesize(builder); )*
                    builder.add_struct($abi_name, "", vec![
                        $( (
                            $crate::serializer::__field_name(stringify!($field)),
                            <$ty as $crate::serializer::ABISerializable>::abi_type(),
                        ), )*
                    ]);
                }
            }
        }
    };
}

/// Define an ABI serializable variant (`std::variant` / `fc::static_variant`).
///
/// Encoded as a varuint32 index followed by the value. JSON representation is
/// `["type_name", value]`. Each variant case must wrap exactly one type and
/// type names must be unique.
///
/// ```
/// use antelope::{abi_variant, chain::{Name, UInt8}, Serializer};
///
/// abi_variant! {
///     pub enum MyVariant("my_variant") {
///         Byte(UInt8),
///         Account(Name),
///         Text(String),
///     }
/// }
///
/// let v = MyVariant::Text("hello".into());
/// assert_eq!(Serializer::encode(&v).hex_string(), "020568656c6c6f");
/// assert_eq!(serde_json::to_string(&v).unwrap(), r#"["string","hello"]"#);
/// ```
#[macro_export]
macro_rules! abi_variant {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident ($abi_name:literal) {
            $( $(#[$vmeta:meta])* $case:ident ( $ty:ty ) ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        $vis enum $name {
            $( $(#[$vmeta])* $case($ty) ),+
        }

        impl $name {
            /// ABI type names of the variant cases, in index order.
            pub fn variant_types() -> ::std::vec::Vec<::std::string::String> {
                vec![ $( <$ty as $crate::serializer::ABISerializable>::abi_type() ),+ ]
            }

            /// Index of this variant case.
            pub fn variant_index(&self) -> usize {
                let mut _idx = 0usize;
                $(
                    if let $name::$case(_) = self {
                        return _idx;
                    }
                    _idx += 1;
                )+
                unreachable!()
            }

            /// ABI type name of this variant case.
            pub fn variant_name(&self) -> ::std::string::String {
                Self::variant_types().swap_remove(self.variant_index())
            }
        }

        impl $crate::serializer::ABISerializable for $name {
            fn abi_type() -> ::std::string::String {
                ::std::string::String::from($abi_name)
            }
            fn to_abi(&self, encoder: &mut $crate::serializer::ABIEncoder) {
                encoder.write_varuint32(self.variant_index() as u32);
                match self {
                    $( $name::$case(v) => $crate::serializer::ABISerializable::to_abi(v, encoder), )+
                }
            }
            fn from_abi(decoder: &mut $crate::serializer::ABIDecoder) -> $crate::Result<Self> {
                let idx = decoder.read_varuint32()? as usize;
                let mut _i = 0usize;
                $(
                    if idx == _i {
                        return Ok($name::$case(
                            <$ty as $crate::serializer::ABISerializable>::from_abi(decoder)?,
                        ));
                    }
                    _i += 1;
                )+
                Err($crate::Error::Msg(format!("Unknown variant idx: {}", idx)))
            }
            fn synthesize(builder: &mut $crate::serializer::AbiBuilder) {
                if builder.begin($abi_name) {
                    $( <$ty as $crate::serializer::ABISerializable>::synthesize(builder); )+
                    builder.add_variant($abi_name, Self::variant_types());
                }
            }
        }

        impl $crate::__serde::Serialize for $name {
            fn serialize<S: $crate::__serde::Serializer>(&self, s: S) -> ::std::result::Result<S::Ok, S::Error> {
                use $crate::__serde::ser::SerializeTuple;
                let mut t = s.serialize_tuple(2)?;
                t.serialize_element(&self.variant_name())?;
                match self {
                    $( $name::$case(v) => t.serialize_element(v)?, )+
                }
                t.end()
            }
        }

        impl<'de> $crate::__serde::Deserialize<'de> for $name {
            fn deserialize<D: $crate::__serde::Deserializer<'de>>(d: D) -> ::std::result::Result<Self, D::Error> {
                use $crate::__serde::de::Error as _;
                let (type_name, value): (::std::string::String, $crate::__serde_json::Value) =
                    $crate::__serde::Deserialize::deserialize(d)?;
                $(
                    if type_name == <$ty as $crate::serializer::ABISerializable>::abi_type() {
                        return $crate::__serde_json::from_value::<$ty>(value)
                            .map($name::$case)
                            .map_err(D::Error::custom);
                    }
                )+
                Err(D::Error::custom(format!(
                    "Unknown variant type '{}', expected one of {:?}",
                    type_name,
                    Self::variant_types()
                )))
            }
        }
    };
}
