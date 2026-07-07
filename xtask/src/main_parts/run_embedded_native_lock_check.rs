fn run_embedded_native_lock_check() -> Result<()> {
    let lockfile = fs::read_to_string(EMBEDDED_NATIVE_LOCKFILE_PATH)
        .with_context(|| format!("missing {EMBEDDED_NATIVE_LOCKFILE_PATH}"))?;
    let required_markers = [
        "contract_ble_camera_wire_ref =",
        "contract_ble_transport_runtime_ref =",
        "contract_native_embedded_interop_ref =",
        "firmware_repo =",
        "firmware_ref =",
        "owners = [",
        "ci_workflow =",
        "xtask_gate = \"embedded-native-lock-check\"",
    ];
    for marker in required_markers {
        if !lockfile.contains(marker) {
            bail!("embedded native lockfile missing marker '{marker}' in {EMBEDDED_NATIVE_LOCKFILE_PATH}");
        }
    }
    for forbidden in ["<set-me>", "TODO", "TBD"] {
        if lockfile.contains(forbidden) {
            bail!("embedded native lockfile contains unresolved placeholder '{forbidden}'");
        }
    }

    verify_embedded_native_profile_files()?;

    for marker in [
        "contract_native_embedded_lab_profile_ref =",
        "contract_native_embedded_node_config_ref =",
        "release_revision_mode = \"pinned\"",
        "tcp_read_timeout_secs = 8",
        "tcp_heartbeat_interval_ms = 30000",
        "capture_hard_max_bytes = 2097152",
    ] {
        if !lockfile.contains(marker) {
            bail!("embedded native lockfile missing marker '{marker}' in {EMBEDDED_NATIVE_LOCKFILE_PATH}");
        }
    }

    Ok(())
}

fn verify_embedded_native_profile_files() -> Result<()> {
    verify_markers(
        EMBEDDED_NATIVE_INTEROP_PROFILE_PATH,
        &[
            "# Native Embedded Interop Profile v1",
            "## Lab Profile Reference",
            "## Normative Encoding Rules",
            "## Transport Invariants",
            "## Canonical Transport Parameters",
            "## Lifecycle Ownership",
            "## Success Response Schemas",
            "## Error Code Mapping",
            "## Fixture Set",
        ],
    )?;
    for path in [
        BLE_CAMERA_WIRE_CONTRACT_PATH,
        BLE_TRANSPORT_RUNTIME_CONTRACT_PATH,
        EMBEDDED_NATIVE_LAB_PROFILE_PATH,
        EMBEDDED_NATIVE_NODE_CONFIG_PATH,
        EMBEDDED_NATIVE_WORKFLOW_PATH,
    ] {
        if !Path::new(path).exists() {
            bail!("required path missing for embedded native lock check: {path}");
        }
    }
    verify_markers(
        EMBEDDED_NATIVE_LAB_PROFILE_PATH,
        &[
            "# Native Embedded Lab Profile v1",
            "## Hardware",
            "## Network Profiles",
            "### LAN profile",
            "### Internet-shaped profile",
            "## Measurement Rules",
        ],
    )?;
    verify_markers(
        EMBEDDED_NATIVE_NODE_CONFIG_PATH,
        &[
            "# Native Embedded Node Config v1",
            "## Schema Version",
            "## Stored Fields",
            "### Node mode",
            "### Wi-Fi",
            "### TCP client",
            "### TCP server",
            "## Lifecycle coupling",
        ],
    )
}

fn verify_markers(path: &str, markers: &[&str]) -> Result<()> {
    let content = fs::read_to_string(path).with_context(|| format!("missing {path}"))?;
    for marker in markers {
        if !content.contains(marker) {
            bail!("embedded native document missing marker '{marker}' in {path}");
        }
    }
    Ok(())
}
