#[test]
fn sdk_delivery_stamp_policy_envelope_roundtrip() {
    let daemon = RpcDaemon::test_instance();
    let registry_entries = daemon
        .handle_rpc(rpc_request(12150, "sdk_operation_registry_v2", json!({})))
        .expect("operation registry")
        .result
        .expect("registry result")["registry"]["entries"]
        .as_array()
        .expect("registry entries")
        .clone();
    assert!(registry_entries
        .iter()
        .any(|entry| entry["id"] == json!("app.delivery.stamp_policy.get")));
    assert!(registry_entries
        .iter()
        .any(|entry| entry["id"] == json!("app.delivery.stamp_policy.set")));

    let stamp_policy_get_envelope = daemon
        .handle_rpc(rpc_request(
            12151,
            "sdk_envelope_execute_v2",
            json!({
                "operation_id": "app.delivery.stamp_policy.get",
                "kind": "query",
                "correlation_id": "stamp-policy-get-corr-1",
                "payload": {},
            }),
        ))
        .expect("delivery stamp policy get envelope");
    assert!(stamp_policy_get_envelope.error.is_none());
    let stamp_policy_get_response =
        stamp_policy_get_envelope.result.expect("delivery stamp policy get envelope result");
    assert_eq!(
        stamp_policy_get_response["response"]["operation_id"],
        json!("app.delivery.stamp_policy.get")
    );
    assert_eq!(
        stamp_policy_get_response["response"]["payload"]["stamp_policy"]["enforce"],
        json!(true)
    );

    let stamp_policy_set_envelope = daemon
        .handle_rpc(rpc_request(
            12152,
            "sdk_envelope_execute_v2",
            json!({
                "operation_id": "stamp_policy_set",
                "kind": "command",
                "correlation_id": "stamp-policy-set-corr-1",
                "payload": {
                    "target_cost": 14,
                    "flexibility": 5,
                    "enforce": true
                },
            }),
        ))
        .expect("delivery stamp policy set envelope");
    assert!(stamp_policy_set_envelope.error.is_none());
    let stamp_policy_set_response =
        stamp_policy_set_envelope.result.expect("delivery stamp policy set envelope result");
    assert_eq!(
        stamp_policy_set_response["response"]["operation_id"],
        json!("app.delivery.stamp_policy.set")
    );
    assert_eq!(
        stamp_policy_set_response["response"]["correlation_id"],
        json!("stamp-policy-set-corr-1")
    );
    assert_eq!(
        stamp_policy_set_response["response"]["payload"]["stamp_policy"]["target_cost"],
        json!(14)
    );
    assert_eq!(
        stamp_policy_set_response["response"]["payload"]["stamp_policy"]["flexibility"],
        json!(5)
    );
    assert_eq!(
        stamp_policy_set_response["response"]["payload"]["stamp_policy"]["enforce"],
        json!(true)
    );
}
