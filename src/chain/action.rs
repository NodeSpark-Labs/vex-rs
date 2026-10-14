use serde::{Deserialize, Serialize};

use super::{Bytes, Name, PermissionLevel, ABI};
use crate::serializer::{
    decode_json, encode_json, ABIDecoder, ABIEncoder, ABISerializable, AbiBuilder, DecodeOptions,
    Serializer,
};
use crate::{Error, Result};

/// Antelope/EOSIO action (`action`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    /// The account (a.k.a. contract) to run action on.
    pub account: Name,
    /// The name of the action.
    pub name: Name,
    /// The permissions authorizing the action.
    pub authorization: Vec<PermissionLevel>,
    /// The ABI-encoded action data.
    pub data: Bytes,
}

/// An action whose data has not been encoded yet, mirrors `AnyAction`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnyAction {
    pub account: Name,
    pub name: Name,
    pub authorization: Vec<PermissionLevel>,
    /// Either a hex string with encoded data or a JSON object to encode using an ABI.
    pub data: serde_json::Value,
}

impl Action {
    /// Create an action with already encoded data.
    pub fn new(
        account: impl Into<Name>,
        name: impl Into<Name>,
        authorization: Vec<PermissionLevel>,
        data: impl Into<Bytes>,
    ) -> Self {
        Action {
            account: account.into(),
            name: name.into(),
            authorization,
            data: data.into(),
        }
    }

    /// Create an action with typed action data.
    pub fn from_typed<T: ABISerializable>(
        account: impl Into<Name>,
        name: impl Into<Name>,
        authorization: Vec<PermissionLevel>,
        data: &T,
    ) -> Self {
        Self::new(account, name, authorization, Serializer::encode(data))
    }

    /// Create an action from JSON action data encoded using the contract ABI.
    pub fn from_json(
        account: impl Into<Name>,
        name: impl Into<Name>,
        authorization: Vec<PermissionLevel>,
        data: &serde_json::Value,
        abi: &ABI,
    ) -> Result<Self> {
        let name = name.into();
        let r#type = abi.get_action_type(name).ok_or_else(|| {
            Error::msg(format!(
                "The action \"{}\" does not exist on the ABI provided.",
                name
            ))
        })?;
        let encoded = encode_json(abi, r#type, data)?;
        Ok(Self::new(account, name, authorization, encoded))
    }

    /// Resolve an [`AnyAction`], encoding the data with the ABI if it isn't hex.
    pub fn from_any(action: &AnyAction, abi: Option<&ABI>) -> Result<Self> {
        if let serde_json::Value::String(s) = &action.data {
            if Bytes::is_bytes(s) {
                return Ok(Self::new(
                    action.account,
                    action.name,
                    action.authorization.clone(),
                    Bytes::from_hex(s)?,
                ));
            }
        }
        let abi = abi.ok_or_else(|| {
            Error::msg("Missing ABI definition when creating action with untyped action data")
        })?;
        Self::from_json(
            action.account,
            action.name,
            action.authorization.clone(),
            &action.data,
            abi,
        )
    }

    /// Decode action data as given type.
    pub fn decode_data<T: ABISerializable>(&self) -> Result<T> {
        Serializer::decode(&self.data)
    }

    /// Decode action data to JSON using the contract ABI.
    pub fn decode_data_with_abi(&self, abi: &ABI) -> Result<serde_json::Value> {
        let r#type = abi.get_action_type(self.name).ok_or_else(|| {
            Error::msg(format!(
                "Action {} does not exist in provided ABI",
                self.name
            ))
        })?;
        decode_json(abi, r#type, &self.data, &DecodeOptions::default())
    }

    /// Return a JSON representation with the data decoded using the contract ABI.
    pub fn decoded(&self, abi: &ABI) -> Result<serde_json::Value> {
        let mut rv = serde_json::to_value(self)?;
        rv["data"] = self.decode_data_with_abi(abi)?;
        Ok(rv)
    }
}

impl ABISerializable for Action {
    fn abi_type() -> String {
        "action".into()
    }
    fn to_abi(&self, encoder: &mut ABIEncoder) {
        self.account.to_abi(encoder);
        self.name.to_abi(encoder);
        self.authorization.to_abi(encoder);
        self.data.to_abi(encoder);
    }
    fn from_abi(decoder: &mut ABIDecoder) -> Result<Self> {
        Ok(Action {
            account: Name::from_abi(decoder)?,
            name: Name::from_abi(decoder)?,
            authorization: Vec::from_abi(decoder)?,
            data: Bytes::from_abi(decoder)?,
        })
    }
    fn synthesize(builder: &mut AbiBuilder) {
        if builder.begin("action") {
            PermissionLevel::synthesize(builder);
            builder.add_struct(
                "action",
                "",
                vec![
                    ("account".into(), "name".into()),
                    ("name".into(), "name".into()),
                    ("authorization".into(), "permission_level[]".into()),
                    ("data".into(), "bytes".into()),
                ],
            );
        }
    }
}
