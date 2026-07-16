fn parse_rnpath_output(output: &[u8], destination_hash: &str) -> io::Result<serde_json::Value> {
    let result: serde_json::Value = serde_json::from_slice(output)?;
    if result.get("path_found").and_then(serde_json::Value::as_bool) != Some(true) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("rnpath-rs did not report path_found=true: {result}"),
        ));
    }
    if result.get("status").and_then(serde_json::Value::as_str) != Some("found") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("rnpath-rs did not report status=found: {result}"),
        ));
    }
    if result.get("destination_hash").and_then(serde_json::Value::as_str) != Some(destination_hash)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("rnpath-rs reported unexpected destination hash: {result}"),
        ));
    }
    Ok(result)
}

fn validate_scoped_rnpath_result(
    result: &serde_json::Value,
    on_iface: &str,
    tag_hex: &str,
    expected_next_hop: &str,
    expected_hops: Option<u64>,
) -> io::Result<()> {
    let reported_iface = required_path_field(result, "on_iface")?;
    if reported_iface != on_iface {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("rnpath-rs reported unexpected on_iface: {result}"),
        ));
    }
    let interface_scope = required_path_field(result, "interface_scope")?;
    if interface_scope != on_iface {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("rnpath-rs reported unexpected interface_scope: {result}"),
        ));
    }
    let reported_tag = required_path_field(result, "tag_hex")?;
    if reported_tag != tag_hex {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("rnpath-rs reported unexpected tag_hex: {result}"),
        ));
    }
    let next_hop = required_path_field(result, "next_hop")?;
    if next_hop != expected_next_hop {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("scoped rnpath-rs result changed next_hop: {result}"),
        ));
    }
    let interface = normalized_hash_field(result, "interface")?;
    if interface != on_iface {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("scoped rnpath-rs result changed interface metadata: {result}"),
        ));
    }
    if result.get("hops").and_then(serde_json::Value::as_u64) != expected_hops {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("scoped rnpath-rs result changed hop metadata: {result}"),
        ));
    }
    Ok(())
}

fn normalized_hash_field(result: &serde_json::Value, key: &str) -> io::Result<String> {
    let value = required_path_field(result, key)?;
    let normalized =
        value.strip_prefix('/').and_then(|stripped| stripped.strip_suffix('/')).unwrap_or(value);
    if normalized.len() != 32 || !normalized.as_bytes().iter().all(u8::is_ascii_hexdigit) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("rnpath-rs reported non-hash {key}: {result}"),
        ));
    }
    Ok(normalized.to_ascii_lowercase())
}

fn required_path_field<'a>(result: &'a serde_json::Value, key: &str) -> io::Result<&'a str> {
    let value = result.get(key).and_then(serde_json::Value::as_str).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, format!("rnpath-rs omitted {key}: {result}"))
    })?;
    if value.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("rnpath-rs reported empty {key}: {result}"),
        ));
    }
    Ok(value)
}

fn rnpath_rs_path() -> io::Result<PathBuf> {
    let binary_name = format!("rnpath-rs{}", std::env::consts::EXE_SUFFIX);
    let exe = std::env::current_exe()?;
    let dir = exe.parent().ok_or_else(|| io::Error::other("missing exe parent"))?;
    let candidate = dir.join(&binary_name);
    if candidate.exists() {
        return Ok(candidate);
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(&binary_name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "rnpath-rs binary not found; build it with `cargo build -p reticulumd --bin reticulumd -p rns-tools --bin rnx --bin rnpath-rs` before running `rnx rnpath-smoke`",
    ))
}
