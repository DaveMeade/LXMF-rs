#[test]
fn delivery_policy_single_entry_mutators_follow_python_router_lists() {
    let daemon = RpcDaemon::test_instance();
    let destination = "AABBCCDDEEFF00112233445566778899";
    let normalized = "aabbccddeeff00112233445566778899";

    let allow = daemon
        .handle_rpc(rpc_request(
            10,
            "allow_destination",
            json!({ "destination_hash": destination }),
        ))
        .expect("allow destination");
    assert!(allow.error.is_none());
    assert_eq!(
        allow.result.as_ref().expect("allow result")["policy"]["allowed_destinations"],
        json!([normalized])
    );

    let duplicate_allow = daemon
        .handle_rpc(rpc_request(
            11,
            "allow_destination",
            json!({ "destination": normalized }),
        ))
        .expect("duplicate allow destination");
    assert!(duplicate_allow.error.is_none());
    assert_eq!(
        duplicate_allow.result.as_ref().expect("duplicate allow result")["policy"]
            ["allowed_destinations"],
        json!([normalized])
    );

    let prioritise = daemon
        .handle_rpc(rpc_request(12, "prioritise_destination", json!({ "hash": destination })))
        .expect("prioritise destination");
    assert!(prioritise.error.is_none());
    assert_eq!(
        prioritise.result.as_ref().expect("prioritise result")["policy"]
            ["prioritised_destinations"],
        json!([normalized])
    );

    let duplicate_prioritise = daemon
        .handle_rpc(rpc_request(
            13,
            "prioritise_destination",
            json!({ "destination_hash": normalized }),
        ))
        .expect("duplicate prioritise destination");
    assert!(duplicate_prioritise.error.is_none());
    assert_eq!(
        duplicate_prioritise.result.as_ref().expect("duplicate prioritise result")["policy"]
            ["prioritised_destinations"],
        json!([normalized])
    );

    let disallow = daemon
        .handle_rpc(rpc_request(
            14,
            "disallow_destination",
            json!({ "destination_hash": normalized }),
        ))
        .expect("disallow destination");
    assert!(disallow.error.is_none());
    assert_eq!(
        disallow.result.as_ref().expect("disallow result")["policy"]["allowed_destinations"],
        json!([])
    );

    let duplicate_disallow = daemon
        .handle_rpc(rpc_request(
            15,
            "disallow_destination",
            json!({ "destination_hash": normalized }),
        ))
        .expect("duplicate disallow destination");
    assert!(duplicate_disallow.error.is_none());
    assert_eq!(
        duplicate_disallow.result.as_ref().expect("duplicate disallow result")["policy"]
            ["allowed_destinations"],
        json!([])
    );
}

#[test]
fn delivery_policy_single_entry_mutators_reject_malformed_hashes() {
    let daemon = RpcDaemon::test_instance();

    let err = daemon
        .handle_rpc(rpc_request(20, "allow_destination", json!({ "destination": "abcd" })))
        .expect_err("short destination hash should be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

    let err = daemon
        .handle_rpc(rpc_request(
            21,
            "prioritise_destination",
            json!({ "destination_hash": "not-hex" }),
        ))
        .expect_err("non-hex destination hash should be rejected");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn delivery_policy_single_entry_mutators_delegate_through_sdk_envelope() {
    let daemon = RpcDaemon::test_instance();
    let destination = "00112233445566778899AABBCCDDEEFF";
    let normalized = "00112233445566778899aabbccddeeff";

    let envelope = daemon
        .handle_rpc(rpc_request(
            30,
            "sdk_envelope_execute_v2",
            json!({
                "operation_id": "app.propagation.delivery_policy.allow_destination",
                "kind": "command",
                "correlation_id": "allow-destination-corr-1",
                "payload": {
                    "destination": destination
                }
            }),
        ))
        .expect("allow destination envelope");
    assert!(envelope.error.is_none());
    let result = envelope.result.expect("envelope result");
    assert_eq!(
        result["response"]["operation_id"],
        json!("app.propagation.delivery_policy.allow_destination")
    );
    assert_eq!(result["response"]["correlation_id"], json!("allow-destination-corr-1"));
    assert_eq!(
        result["response"]["payload"]["policy"]["allowed_destinations"],
        json!([normalized])
    );
}
