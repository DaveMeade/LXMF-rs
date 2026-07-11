#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouterStoragePolicy {
    pub message_limit_bytes: Option<u64>,
    pub information_limit_bytes: Option<u64>,
    pub retain_node_lxms: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouterStoragePolicyPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_limit_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub information_limit_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retain_node_lxms: Option<bool>,
}

impl RouterStoragePolicyPatch {
    pub fn is_empty(&self) -> bool {
        self.message_limit_bytes.is_none()
            && self.information_limit_bytes.is_none()
            && self.retain_node_lxms.is_none()
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouterStats {
    pub messages: u64,
    pub message_bytes: u64,
    pub peers: u64,
    pub interfaces: u64,
    pub tickets: u64,
    pub propagation_payloads: u64,
    pub outbound_inflight: u64,
    pub propagation_enabled: bool,
    pub propagation_node_enabled: bool,
    pub storage_policy: RouterStoragePolicy,
}
