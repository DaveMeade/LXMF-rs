#[derive(Debug, Deserialize)]
struct PathLookupParams {
    #[serde(alias = "destination_hash", alias = "hash")]
    destination: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default, alias = "interface")]
    on_iface: Option<String>,
    #[serde(default, alias = "tag")]
    tag_hex: Option<String>,
}

const RETICULUM_MTU_BYTES: f64 = 500.0;
const RETICULUM_DEFAULT_PER_HOP_TIMEOUT_SECS: f64 = 6.0;

fn normalize_destination_hash_param(destination: &str) -> Result<String, std::io::Error> {
    let destination = destination.trim();
    let decoded = hex::decode(destination).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("destination must be hex-encoded: {err}"),
        )
    })?;
    if decoded.len() != 16 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "destination must decode to a 16-byte RNS destination hash",
        ));
    }
    Ok(destination.to_ascii_lowercase())
}

fn normalize_optional_iface_hash_param(
    on_iface: Option<&str>,
) -> Result<Option<String>, std::io::Error> {
    let Some(on_iface) = on_iface else {
        return Ok(None);
    };
    let on_iface = on_iface.trim();
    if on_iface.is_empty() {
        return Ok(None);
    }
    normalize_destination_hash_param(on_iface)
        .map(Some)
        .map_err(|err| std::io::Error::new(err.kind(), format!("on_iface {err}")))
}

fn normalize_optional_tag_hex_param(tag_hex: Option<&str>) -> Result<Option<Vec<u8>>, std::io::Error> {
    let Some(tag_hex) = tag_hex else {
        return Ok(None);
    };
    let tag_hex = tag_hex.trim();
    if tag_hex.is_empty() {
        return Ok(None);
    }
    let tag = hex::decode(tag_hex).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("tag_hex must be hex-encoded: {err}"),
        )
    })?;
    if tag.is_empty() || tag.len() > 16 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "tag_hex must decode to 1..=16 bytes",
        ));
    }
    Ok(Some(tag))
}

fn path_found_from_status(status_fields: &JsonValue) -> bool {
    status_fields
        .get("path_found")
        .and_then(JsonValue::as_bool)
        .or_else(|| status_fields.get("known").and_then(JsonValue::as_bool))
        .unwrap_or(false)
}

fn bounded_path_poll_delay(delay: std::time::Duration) {
    std::thread::park_timeout(delay);
}

fn path_lookup_result(
    destination: String,
    status_fields: JsonValue,
    requested: Option<bool>,
    missing_status: &str,
) -> JsonValue {
    let mut object = match status_fields {
        JsonValue::Object(object) => object,
        _ => JsonMap::new(),
    };
    let path_found = object
        .get("path_found")
        .and_then(JsonValue::as_bool)
        .unwrap_or_else(|| object.get("known").and_then(JsonValue::as_bool).unwrap_or(false));

    object.insert("destination".to_string(), json!(destination));
    object.insert("destination_hash".to_string(), json!(destination));
    object.entry("known".to_string()).or_insert_with(|| json!(path_found));
    object.entry("path_found".to_string()).or_insert_with(|| json!(path_found));
    if let Some(requested) = requested {
        object.insert("requested".to_string(), json!(requested));
    }
    object
        .entry("status".to_string())
        .or_insert_with(|| json!(if path_found { "found" } else { missing_status }));
    JsonValue::Object(object)
}

fn add_path_request_scope_fields(
    result: &mut JsonValue,
    on_iface: Option<&str>,
    tag: Option<&[u8]>,
) {
    let JsonValue::Object(object) = result else {
        return;
    };
    if let Some(on_iface) = on_iface {
        object.insert("on_iface".to_string(), json!(on_iface));
        object.insert("interface_scope".to_string(), json!(on_iface));
    }
    if let Some(tag) = tag {
        object.insert("tag_hex".to_string(), json!(hex::encode(tag)));
    }
}

fn path_status_string(status_fields: &JsonValue, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        status_fields
            .get(*key)
            .and_then(JsonValue::as_str)
            .map(str::to_string)
    })
}

fn first_hop_timeout_from_status(status_fields: &JsonValue) -> f64 {
    if let Some(timeout) = status_fields.get("first_hop_timeout").and_then(JsonValue::as_f64) {
        return timeout;
    }
    if let Some(latency) = status_fields.get("per_byte_latency").and_then(JsonValue::as_f64) {
        return RETICULUM_DEFAULT_PER_HOP_TIMEOUT_SECS + RETICULUM_MTU_BYTES * latency;
    }
    let bitrate = ["interface_bitrate", "next_hop_interface_bitrate", "bitrate"]
        .iter()
        .find_map(|key| status_fields.get(*key).and_then(JsonValue::as_f64))
        .filter(|value| *value > 0.0);
    bitrate
        .map(|value| RETICULUM_DEFAULT_PER_HOP_TIMEOUT_SECS + (RETICULUM_MTU_BYTES * 8.0 / value))
        .unwrap_or(RETICULUM_DEFAULT_PER_HOP_TIMEOUT_SECS)
}

impl RpcDaemon {
    fn handle_rpc_legacy_path_metadata(
        &self,
        request: RpcRequest,
    ) -> Result<RpcResponse, std::io::Error> {
        let params = request.params.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
        })?;
        let parsed: PathLookupParams = serde_json::from_value(params)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        let destination = normalize_destination_hash_param(&parsed.destination)?;
        let Some(bridge) = self
            .path_lookup_bridge
            .lock()
            .expect("path_lookup_bridge mutex poisoned")
            .clone()
        else {
            return Ok(RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError::new(
                    "PATH_LOOKUP_UNAVAILABLE",
                    "path lookup bridge is not configured",
                )),
            });
        };
        let status_fields = match bridge.path_status(destination.as_str()) {
            Ok(status_fields) => status_fields,
            Err(err) => {
                return Ok(RpcResponse {
                    id: request.id,
                    result: None,
                    error: Some(RpcError::new("PATH_LOOKUP_FAILED", err.to_string())),
                });
            }
        };
        let mut result = path_lookup_result(destination, status_fields.clone(), None, "unknown");
        let JsonValue::Object(object) = &mut result else {
            return Ok(RpcResponse { id: request.id, result: Some(result), error: None });
        };

        match request.method.as_str() {
            "next_hop" => {
                let next_hop = path_status_string(&status_fields, &["next_hop"]);
                object.insert("next_hop".to_string(), json!(next_hop));
            }
            "next_hop_if_name" => {
                let interface = path_status_string(
                    &status_fields,
                    &["next_hop_if_name", "interface_name", "interface"],
                );
                object.insert("next_hop_if_name".to_string(), json!(interface));
            }
            "first_hop_timeout" => {
                object.insert(
                    "first_hop_timeout".to_string(),
                    json!(first_hop_timeout_from_status(&status_fields)),
                );
            }
            _ => {}
        }

        Ok(RpcResponse { id: request.id, result: Some(result), error: None })
    }
}
