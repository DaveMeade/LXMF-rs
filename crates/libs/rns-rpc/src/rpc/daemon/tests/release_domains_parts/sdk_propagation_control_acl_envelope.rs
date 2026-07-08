#[test]
fn sdk_propagation_control_acl_envelope_roundtrip() {
    let daemon = RpcDaemon::test_instance();
    let registry_entries = daemon
        .handle_rpc(rpc_request(1260, "sdk_operation_registry_v2", json!({})))
        .expect("operation registry")
        .result
        .expect("registry result")["registry"]["entries"]
        .as_array()
        .expect("registry entries")
        .clone();
    assert!(registry_entries
        .iter()
        .any(|entry| entry["id"] == json!("app.propagation.control.allow")));
    assert!(registry_entries
        .iter()
        .any(|entry| entry["id"] == json!("app.propagation.control.disallow")));

    let propagation_enable_envelope = daemon
        .handle_rpc(rpc_request(
            1261,
            "sdk_envelope_execute_v2",
            json!({
                "operation_id": "app.propagation.enable",
                "kind": "command",
                "payload": {
                    "enabled": true,
                    "control_allowed": ["FFEEDDCCBBAA99887766554433221100"]
                },
            }),
        ))
        .expect("propagation enable envelope");
    assert!(propagation_enable_envelope.error.is_none());
    let propagation_enable_response =
        propagation_enable_envelope.result.expect("propagation enable envelope result");
    assert_eq!(
        propagation_enable_response["response"]["payload"]["propagation"]["control_allowed"],
        json!(["ffeeddccbbaa99887766554433221100"])
    );

    let propagation_control_allow_envelope = daemon
        .handle_rpc(rpc_request(
            1262,
            "sdk_envelope_execute_v2",
            json!({
                "operation_id": "app.propagation.control.allow",
                "kind": "command",
                "correlation_id": "propagation-control-allow-corr-1",
                "payload": {
                    "identity_hash": "AABBCCDDEEFF00112233445566778899"
                },
            }),
        ))
        .expect("propagation control allow envelope");
    assert!(propagation_control_allow_envelope.error.is_none());
    let propagation_control_allow_response = propagation_control_allow_envelope
        .result
        .expect("propagation control allow envelope result");
    assert_eq!(
        propagation_control_allow_response["response"]["operation_id"],
        json!("app.propagation.control.allow")
    );
    assert_eq!(
        propagation_control_allow_response["response"]["correlation_id"],
        json!("propagation-control-allow-corr-1")
    );
    assert_eq!(
        propagation_control_allow_response["response"]["payload"]["identity_hash"],
        json!("aabbccddeeff00112233445566778899")
    );
    assert_eq!(
        propagation_control_allow_response["response"]["payload"]["propagation"]
            ["control_allowed"],
        json!([
            "ffeeddccbbaa99887766554433221100",
            "aabbccddeeff00112233445566778899"
        ])
    );

    let propagation_control_disallow_envelope = daemon
        .handle_rpc(rpc_request(
            1263,
            "sdk_envelope_execute_v2",
            json!({
                "operation_id": "disallow_control",
                "kind": "command",
                "correlation_id": "propagation-control-disallow-corr-1",
                "payload": {
                    "identity_hash": "aabbccddeeff00112233445566778899"
                },
            }),
        ))
        .expect("propagation control disallow envelope");
    assert!(propagation_control_disallow_envelope.error.is_none());
    let propagation_control_disallow_response = propagation_control_disallow_envelope
        .result
        .expect("propagation control disallow envelope result");
    assert_eq!(
        propagation_control_disallow_response["response"]["operation_id"],
        json!("app.propagation.control.disallow")
    );
    assert_eq!(
        propagation_control_disallow_response["response"]["correlation_id"],
        json!("propagation-control-disallow-corr-1")
    );
    assert_eq!(
        propagation_control_disallow_response["response"]["payload"]["identity_hash"],
        json!("aabbccddeeff00112233445566778899")
    );
    assert_eq!(
        propagation_control_disallow_response["response"]["payload"]["propagation"]
            ["control_allowed"],
        json!(["ffeeddccbbaa99887766554433221100"])
    );
}
