//! Contract ABI definition (`abi`).

use std::collections::HashSet;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::{Blob, Name};
use crate::serializer::{ABIDecoder, ABIEncoder, ABISerializable, Serializer};
use crate::{Error, Result};

/// Type alias definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeDef {
    pub new_type_name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

/// Struct field definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub r#type: String,
}

/// Struct definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Struct {
    pub name: String,
    #[serde(default)]
    pub base: String,
    #[serde(default)]
    pub fields: Vec<Field>,
}

/// Action definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub name: Name,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(default)]
    pub ricardian_contract: String,
}

/// Table definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Table {
    pub name: Name,
    #[serde(default)]
    pub index_type: String,
    #[serde(default)]
    pub key_names: Vec<String>,
    #[serde(default)]
    pub key_types: Vec<String>,
    #[serde(rename = "type")]
    pub r#type: String,
}

/// Ricardian clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Clause {
    pub id: String,
    pub body: String,
}

/// Variant definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub types: Vec<String>,
}

/// Action result definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionResult {
    pub name: Name,
    pub result_type: String,
}

fn default_version() -> String {
    ABI::VERSION.to_string()
}

/// Antelope/EOSIO contract ABI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ABI {
    #[serde(default = "default_version")]
    pub version: String,
    /// List of type aliases.
    #[serde(default)]
    pub types: Vec<TypeDef>,
    /// List of struct types.
    #[serde(default)]
    pub structs: Vec<Struct>,
    /// List of contract actions.
    #[serde(default)]
    pub actions: Vec<Action>,
    /// List of contract tables.
    #[serde(default)]
    pub tables: Vec<Table>,
    /// Ricardian contracts.
    #[serde(default)]
    pub ricardian_clauses: Vec<Clause>,
    /// Not used, always empty.
    #[serde(default, skip_deserializing)]
    pub error_messages: Vec<serde_json::Value>,
    /// Not used, always empty.
    #[serde(default, skip_deserializing)]
    pub abi_extensions: Vec<serde_json::Value>,
    /// List of variant types.
    #[serde(default)]
    pub variants: Vec<Variant>,
    /// Action results.
    #[serde(default)]
    pub action_results: Vec<ActionResult>,
}

impl Default for ABI {
    fn default() -> Self {
        ABI {
            version: default_version(),
            types: vec![],
            structs: vec![],
            actions: vec![],
            tables: vec![],
            ricardian_clauses: vec![],
            error_messages: vec![],
            abi_extensions: vec![],
            variants: vec![],
            action_results: vec![],
        }
    }
}

/// A type name split into its base name and modifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTypeName {
    /// Base name without suffixes.
    pub name: String,
    pub is_array: bool,
    pub is_optional: bool,
    pub is_extension: bool,
    /// Fixed array size, if any.
    pub size: Option<usize>,
}

impl ResolvedTypeName {
    pub fn parse(full_name: &str) -> Self {
        let mut name = full_name;
        let is_extension = name.ends_with('$');
        if is_extension {
            name = &name[..name.len() - 1];
        }
        let is_optional = name.ends_with('?');
        if is_optional {
            name = &name[..name.len() - 1];
        }
        let mut is_array = false;
        let mut size = None;
        if name.ends_with("[]") {
            name = &name[..name.len() - 2];
            is_array = true;
        } else if name.ends_with(']') {
            if let Some(open) = name.rfind('[') {
                if let Ok(n) = name[open + 1..name.len() - 1].parse::<usize>() {
                    size = Some(n);
                    is_array = true;
                    name = &name[..open];
                }
            }
        }
        ResolvedTypeName {
            name: name.to_string(),
            is_array,
            is_optional,
            is_extension,
            size,
        }
    }

    /// Type name including suffixes: `[]` array, `?` optional, `$` binary extension.
    pub fn type_name(&self) -> String {
        let mut rv = self.name.clone();
        if self.is_array {
            match self.size {
                Some(size) => rv.push_str(&format!("[{}]", size)),
                None => rv.push_str("[]"),
            }
        }
        if self.is_optional {
            rv.push('?');
        }
        if self.is_extension {
            rv.push('$');
        }
        rv
    }
}

impl ABI {
    pub const VERSION: &'static str = "eosio::abi/1.1";

    /// Parse an ABI from its JSON representation.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Decode an ABI from its binary representation.
    pub fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        Serializer::decode(data)
    }

    /// Decode an ABI from a blob (base64 binary ABI).
    pub fn from_blob(blob: &Blob) -> Result<Self> {
        Self::from_binary(&blob.0)
    }

    /// Encode the ABI to its binary representation.
    pub fn to_binary(&self) -> Vec<u8> {
        Serializer::encode(self).0
    }

    pub fn get_struct(&self, name: &str) -> Option<&Struct> {
        self.structs.iter().find(|s| s.name == name)
    }

    pub fn get_variant(&self, name: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.name == name)
    }

    /// Return the type alias target for given name, if it is an alias.
    pub fn get_alias(&self, name: &str) -> Option<&TypeDef> {
        self.types.iter().find(|t| t.new_type_name == name)
    }

    /// Return arguments type of an action in this ABI.
    pub fn get_action_type(&self, action_name: impl Into<Name>) -> Option<&str> {
        let name = action_name.into();
        self.actions
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.r#type.as_str())
    }

    /// All fields of a struct including base struct(s), errors on circular references.
    pub fn all_fields(&self, struct_name: &str) -> Result<Vec<&Field>> {
        let mut chain = vec![];
        let mut seen = HashSet::new();
        let mut current = Some(struct_name.to_string());
        while let Some(name) = current {
            let name = self.resolve_alias(&name);
            if !seen.insert(name.clone()) {
                return Err(Error::msg("Circular struct base reference"));
            }
            let s = self
                .get_struct(&name)
                .ok_or_else(|| Error::msg(format!("Invalid struct base: {}", name)))?;
            chain.push(s);
            current = if s.base.is_empty() {
                None
            } else {
                Some(s.base.clone())
            };
        }
        Ok(chain.iter().rev().flat_map(|s| s.fields.iter()).collect())
    }

    /// Follow type aliases (without modifiers) to the final type name.
    fn resolve_alias(&self, name: &str) -> String {
        let mut name = name.to_string();
        let mut depth = 0;
        while let Some(alias) = self.get_alias(&name) {
            name = alias.r#type.clone();
            depth += 1;
            if depth > 32 {
                break;
            }
        }
        name
    }
}

impl FromStr for ABI {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        ABI::from_json(s)
    }
}

impl ABISerializable for ABI {
    fn abi_type() -> String {
        "abi".into()
    }

    fn to_abi(&self, encoder: &mut ABIEncoder) {
        encoder.write_string(&self.version);
        encoder.write_varuint32(self.types.len() as u32);
        for t in &self.types {
            encoder.write_string(&t.new_type_name);
            encoder.write_string(&t.r#type);
        }
        encoder.write_varuint32(self.structs.len() as u32);
        for s in &self.structs {
            encoder.write_string(&s.name);
            encoder.write_string(&s.base);
            encoder.write_varuint32(s.fields.len() as u32);
            for f in &s.fields {
                encoder.write_string(&f.name);
                encoder.write_string(&f.r#type);
            }
        }
        encoder.write_varuint32(self.actions.len() as u32);
        for a in &self.actions {
            a.name.to_abi(encoder);
            encoder.write_string(&a.r#type);
            encoder.write_string(&a.ricardian_contract);
        }
        encoder.write_varuint32(self.tables.len() as u32);
        for t in &self.tables {
            t.name.to_abi(encoder);
            encoder.write_string(&t.index_type);
            encoder.write_varuint32(t.key_names.len() as u32);
            for k in &t.key_names {
                encoder.write_string(k);
            }
            encoder.write_varuint32(t.key_types.len() as u32);
            for k in &t.key_types {
                encoder.write_string(k);
            }
            encoder.write_string(&t.r#type);
        }
        encoder.write_varuint32(self.ricardian_clauses.len() as u32);
        for c in &self.ricardian_clauses {
            encoder.write_string(&c.id);
            encoder.write_string(&c.body);
        }
        encoder.write_varuint32(0); // error_messages
        encoder.write_varuint32(0); // extensions
        encoder.write_varuint32(self.variants.len() as u32);
        for v in &self.variants {
            encoder.write_string(&v.name);
            encoder.write_varuint32(v.types.len() as u32);
            for t in &v.types {
                encoder.write_string(t);
            }
        }
        encoder.write_varuint32(self.action_results.len() as u32);
        for r in &self.action_results {
            r.name.to_abi(encoder);
            encoder.write_string(&r.result_type);
        }
    }

    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        let version = decoder.read_string()?;
        let mut abi = ABI {
            version,
            ..Default::default()
        };
        for _ in 0..decoder.read_varuint32()? {
            abi.types.push(TypeDef {
                new_type_name: decoder.read_string()?,
                r#type: decoder.read_string()?,
            });
        }
        for _ in 0..decoder.read_varuint32()? {
            let name = decoder.read_string()?;
            let base = decoder.read_string()?;
            let mut fields = vec![];
            for _ in 0..decoder.read_varuint32()? {
                fields.push(Field {
                    name: decoder.read_string()?,
                    r#type: decoder.read_string()?,
                });
            }
            abi.structs.push(Struct { name, base, fields });
        }
        for _ in 0..decoder.read_varuint32()? {
            abi.actions.push(Action {
                name: Name::from_abi(decoder)?,
                r#type: decoder.read_string()?,
                ricardian_contract: decoder.read_string()?,
            });
        }
        for _ in 0..decoder.read_varuint32()? {
            let name = Name::from_abi(decoder)?;
            let index_type = decoder.read_string()?;
            let mut key_names = vec![];
            for _ in 0..decoder.read_varuint32()? {
                key_names.push(decoder.read_string()?);
            }
            let mut key_types = vec![];
            for _ in 0..decoder.read_varuint32()? {
                key_types.push(decoder.read_string()?);
            }
            let r#type = decoder.read_string()?;
            abi.tables.push(Table {
                name,
                index_type,
                key_names,
                key_types,
                r#type,
            });
        }
        for _ in 0..decoder.read_varuint32()? {
            abi.ricardian_clauses.push(Clause {
                id: decoder.read_string()?,
                body: decoder.read_string()?,
            });
        }
        // error_messages, never used
        for _ in 0..decoder.read_varuint32()? {
            decoder.advance(8)?; // uint64 error_code
            let len = decoder.read_varuint32()? as usize;
            decoder.advance(len)?; // string error_msg
        }
        // extensions, not used
        for _ in 0..decoder.read_varuint32()? {
            decoder.advance(2)?; // uint16 type
            let len = decoder.read_varuint32()? as usize;
            decoder.advance(len)?; // bytes data
        }
        // variants is a binary extension
        if decoder.can_read(1) {
            for _ in 0..decoder.read_varuint32()? {
                let name = decoder.read_string()?;
                let mut types = vec![];
                for _ in 0..decoder.read_varuint32()? {
                    types.push(decoder.read_string()?);
                }
                abi.variants.push(Variant { name, types });
            }
        }
        if decoder.can_read(1) {
            for _ in 0..decoder.read_varuint32()? {
                abi.action_results.push(ActionResult {
                    name: Name::from_abi(decoder)?,
                    result_type: decoder.read_string()?,
                });
            }
        }
        Ok(abi)
    }
}
