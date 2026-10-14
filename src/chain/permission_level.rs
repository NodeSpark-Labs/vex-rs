use std::fmt;
use std::str::FromStr;

use super::Name;
use crate::{Error, Result};

crate::abi_struct! {
    /// Antelope/EOSIO Permission Level, a.k.a "auth".
    #[derive(Copy, Eq, Hash)]
    pub struct PermissionLevel("permission_level") {
        pub actor: Name,
        pub permission: Name,
    }
}

impl PermissionLevel {
    pub fn new(actor: impl Into<Name>, permission: impl Into<Name>) -> Self {
        PermissionLevel {
            actor: actor.into(),
            permission: permission.into(),
        }
    }
}

impl fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.actor, self.permission)
    }
}

impl FromStr for PermissionLevel {
    type Err = Error;
    /// Parse from `<actor>@<permission>`.
    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(Error::msg(
                "Invalid permission level string, should be in the format <actor>@<permission>",
            ));
        }
        Ok(PermissionLevel::new(parts[0], parts[1]))
    }
}
