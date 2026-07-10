#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[non_exhaustive]
pub struct PropagationRemoteStatusState {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub selected_node: Option<String>,
    #[serde(default)]
    pub selected_peer: Option<String>,
    #[serde(default)]
    pub failure_kind: Option<String>,
    #[serde(default)]
    pub timed_out: bool,
    #[serde(default)]
    pub access_denied: bool,
    #[serde(default)]
    pub queue_depth: u64,
    #[serde(default)]
    pub retry_count: u64,
    #[serde(default)]
    pub next_sync_attempt: Option<i64>,
    #[serde(default)]
    pub last_sync_error: Option<String>,
}

impl PropagationRemoteStatusState {
    fn from_status(status: &JsonValue) -> Self {
        let state = remote_status_json_string(status, "state").ok().flatten()
            .or_else(|| remote_status_json_string(status, "state_name").ok().flatten());
        let failure_kind = remote_status_json_string(status, "failure_kind").ok().flatten();
        let timed_out = failure_kind.as_deref() == Some("timeout")
            || state.as_deref() == Some("timeout");
        let access_denied = remote_status_json_bool(status, "access_denied").ok().flatten().unwrap_or(false)
            || matches!(
                failure_kind.as_deref(),
                Some("access_denied" | "access-denied" | "no_access")
            );
        Self {
            state,
            selected_node: remote_status_json_string(status, "selected_node").ok().flatten(),
            selected_peer: remote_status_json_string(status, "selected_peer").ok().flatten(),
            failure_kind,
            timed_out,
            access_denied,
            queue_depth: remote_status_json_u64(status, "queue_depth").ok().flatten().unwrap_or(0),
            retry_count: remote_status_json_u64(status, "retry_count").ok().flatten().unwrap_or(0),
            next_sync_attempt: remote_status_json_i64(status, "next_sync_attempt").ok().flatten(),
            last_sync_error: remote_status_json_string(status, "last_sync_error").ok().flatten(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[non_exhaustive]
pub struct PropagationRemoteStats {
    #[serde(default)]
    pub messagestore: PropagationRemoteMessageStoreStats,
    #[serde(default)]
    pub clients: PropagationRemoteClientStats,
    #[serde(default)]
    pub peers: PropagationRemotePeerStats,
    #[serde(default)]
    pub limits: PropagationRemoteLimitStats,
}

impl PropagationRemoteStats {
    fn from_status(status: &JsonValue) -> Self {
        Self {
            messagestore: PropagationRemoteMessageStoreStats::from_status(status),
            clients: PropagationRemoteClientStats::from_status(status),
            peers: PropagationRemotePeerStats::from_status(status),
            limits: PropagationRemoteLimitStats::from_status(status),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[non_exhaustive]
pub struct PropagationRemoteMessageStoreStats {
    #[serde(default)]
    pub count: Option<u64>,
    #[serde(default)]
    pub bytes: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
}

impl PropagationRemoteMessageStoreStats {
    fn from_status(status: &JsonValue) -> Self {
        let messagestore = status
            .get("messagestore")
            .or_else(|| status.get("message_store"))
            .unwrap_or(&JsonValue::Null);
        Self {
            count: remote_status_json_u64(messagestore, "count").ok().flatten(),
            bytes: remote_status_json_u64(messagestore, "bytes").ok().flatten(),
            limit: remote_status_json_u64(messagestore, "limit").ok().flatten(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[non_exhaustive]
pub struct PropagationRemoteClientStats {
    #[serde(default)]
    pub received: Option<u64>,
    #[serde(default)]
    pub served: Option<u64>,
}

impl PropagationRemoteClientStats {
    fn from_status(status: &JsonValue) -> Self {
        let clients = status.get("clients").unwrap_or(&JsonValue::Null);
        Self {
            received: remote_status_json_u64(clients, "client_propagation_messages_received")
                .ok()
                .flatten()
                .or_else(|| {
                    remote_status_json_u64(status, "client_propagation_messages_received")
                        .ok()
                        .flatten()
                }),
            served: remote_status_json_u64(clients, "client_propagation_messages_served")
                .ok()
                .flatten()
                .or_else(|| {
                    remote_status_json_u64(status, "client_propagation_messages_served")
                        .ok()
                        .flatten()
                }),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[non_exhaustive]
pub struct PropagationRemotePeerStats {
    #[serde(default)]
    pub static_peers: Option<u64>,
    #[serde(default)]
    pub discovered_peers: Option<u64>,
    #[serde(default)]
    pub total_peers: Option<u64>,
    #[serde(default)]
    pub max_peers: Option<u64>,
    #[serde(default)]
    pub unpeered_incoming: Option<u64>,
    #[serde(default)]
    pub unpeered_rx_bytes: Option<u64>,
}

impl PropagationRemotePeerStats {
    fn from_status(status: &JsonValue) -> Self {
        Self {
            static_peers: remote_status_count_or_u64(status, "static_peers"),
            discovered_peers: remote_status_count_or_u64(status, "discovered_peers"),
            total_peers: remote_status_json_u64(status, "total_peers").ok().flatten(),
            max_peers: remote_status_json_u64(status, "max_peers").ok().flatten(),
            unpeered_incoming: remote_status_json_u64(status, "unpeered_propagation_incoming")
                .ok()
                .flatten(),
            unpeered_rx_bytes: remote_status_json_u64(status, "unpeered_propagation_rx_bytes")
                .ok()
                .flatten(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[non_exhaustive]
pub struct PropagationRemoteLimitStats {
    #[serde(default)]
    pub delivery_limit: Option<u64>,
    #[serde(default)]
    pub propagation_limit: Option<u64>,
    #[serde(default)]
    pub sync_limit: Option<u64>,
    #[serde(default)]
    pub target_stamp_cost: Option<u64>,
    #[serde(default)]
    pub stamp_cost_flexibility: Option<u64>,
    #[serde(default)]
    pub peering_cost: Option<u64>,
    #[serde(default)]
    pub max_peering_cost: Option<u64>,
}

impl PropagationRemoteLimitStats {
    fn from_status(status: &JsonValue) -> Self {
        Self {
            delivery_limit: remote_status_json_u64(status, "delivery_limit").ok().flatten(),
            propagation_limit: remote_status_json_u64(status, "propagation_limit")
                .ok()
                .flatten(),
            sync_limit: remote_status_json_u64(status, "sync_limit").ok().flatten(),
            target_stamp_cost: remote_status_json_u64(status, "target_stamp_cost")
                .ok()
                .flatten(),
            stamp_cost_flexibility: remote_status_json_u64(status, "stamp_cost_flexibility")
                .ok()
                .flatten(),
            peering_cost: remote_status_json_u64(status, "peering_cost").ok().flatten(),
            max_peering_cost: remote_status_json_u64(status, "max_peering_cost")
                .ok()
                .flatten()
                .or_else(|| remote_status_json_u64(status, "remote_peering_cost_max").ok().flatten()),
        }
    }
}

fn remote_status_count_or_u64(value: &JsonValue, key: &str) -> Option<u64> {
    match value.get(key) {
        Some(JsonValue::Array(rows)) => u64::try_from(rows.len()).ok(),
        _ => remote_status_json_u64(value, key).ok().flatten(),
    }
}

fn remote_status_json_bool(value: &JsonValue, key: &str) -> Result<Option<bool>, &'static str> {
    match value.get(key) {
        None => Ok(None),
        Some(v) => v.as_bool().ok_or("field is not a bool").map(Some),
    }
}

fn remote_status_json_i64(value: &JsonValue, key: &str) -> Result<Option<i64>, &'static str> {
    match value.get(key) {
        None => Ok(None),
        Some(v) => v.as_i64().ok_or("field is not an integer").map(Some),
    }
}

fn remote_status_json_u64(value: &JsonValue, key: &str) -> Result<Option<u64>, &'static str> {
    match value.get(key) {
        None => Ok(None),
        Some(v) => v.as_u64().ok_or("field is not an unsigned integer").map(Some),
    }
}

fn remote_status_json_string(value: &JsonValue, key: &str) -> Result<Option<String>, &'static str> {
    match value.get(key) {
        None => Ok(None),
        Some(v) => v.as_str().ok_or("field is not a string").map(|s| Some(s.to_owned())),
    }
}
