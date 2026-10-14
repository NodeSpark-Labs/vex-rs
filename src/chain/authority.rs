use super::{PermissionLevel, PublicKey, UInt32, Weight};

crate::abi_struct! {
    /// Key with weight (`key_weight`).
    #[derive(Eq)]
    pub struct KeyWeight("key_weight") {
        pub key: PublicKey,
        pub weight: Weight,
    }
}

crate::abi_struct! {
    /// Permission level with weight (`permission_level_weight`).
    #[derive(Eq)]
    pub struct PermissionLevelWeight("permission_level_weight") {
        pub permission: PermissionLevel,
        pub weight: Weight,
    }
}

crate::abi_struct! {
    /// Wait with weight (`wait_weight`).
    #[derive(Eq)]
    pub struct WaitWeight("wait_weight") {
        pub wait_sec: UInt32,
        pub weight: Weight,
    }
}

crate::abi_struct! {
    /// Account permission authority (`authority`).
    #[derive(Eq)]
    pub struct Authority("authority") {
        pub threshold: UInt32,
        #[serde(default)]
        pub keys: Vec<KeyWeight>,
        #[serde(default)]
        pub accounts: Vec<PermissionLevelWeight>,
        #[serde(default)]
        pub waits: Vec<WaitWeight>,
    }
}

impl Authority {
    /// Create a new authority, the weights are sorted.
    pub fn new(
        threshold: u32,
        keys: Vec<KeyWeight>,
        accounts: Vec<PermissionLevelWeight>,
        waits: Vec<WaitWeight>,
    ) -> Self {
        let mut rv = Authority {
            threshold: UInt32(threshold),
            keys,
            accounts,
            waits,
        };
        rv.sort();
        rv
    }

    /// Total weight of all waits.
    pub fn wait_threshold(&self) -> u32 {
        self.waits.iter().map(|w| w.weight.0 as u32).sum()
    }

    /// Weight a key needs to sign for this authority.
    pub fn key_threshold(&self) -> i64 {
        self.threshold.0 as i64 - self.wait_threshold() as i64
    }

    /// Return the weight for given public key, or zero if it is not included in this authority.
    pub fn key_weight(&self, public_key: &PublicKey) -> u32 {
        self.keys
            .iter()
            .find(|k| &k.key == public_key)
            .map(|k| k.weight.0 as u32)
            .unwrap_or(0)
    }

    /// Check if given public key has permission in this authority.
    /// Does not take indirect permissions for the key via account weights into account.
    /// If `include_partial` is set, keys that can't reach the threshold alone are included.
    pub fn has_permission(&self, public_key: &PublicKey, include_partial: bool) -> bool {
        let threshold = if include_partial {
            1
        } else {
            self.key_threshold()
        };
        self.key_weight(public_key) as i64 >= threshold
    }

    /// Sorts the authority weights in place, should be called before including the
    /// authority in a `updateauth` action or it might be rejected.
    pub fn sort(&mut self) {
        // nodeos expects the binary ordering of the C++ types
        self.keys
            .sort_by(|a, b| (a.key.key_type.index(), &a.key.data).cmp(&(b.key.key_type.index(), &b.key.data)));
        self.accounts
            .sort_by_key(|a| (a.permission.actor.0, a.permission.permission.0));
        self.waits.sort_by_key(|a| a.wait_sec.0);
    }
}
