#[test]
fn sdk_operation_registry_includes_policy_mutator_entries() {
    let daemon = RpcDaemon::test_instance();
    let registry_entries = daemon
        .handle_rpc(rpc_request(1390, "sdk_operation_registry_v2", json!({})))
        .expect("operation registry")
        .result
        .expect("registry result")["registry"]["entries"]
        .as_array()
        .expect("registry entries")
        .clone();

    for operation_id in [
        "app.propagation.delivery_policy.auth.set",
        "app.propagation.delivery_policy.auth.get",
        "app.propagation.delivery_policy.allow",
        "app.propagation.delivery_policy.disallow",
        "app.propagation.delivery_policy.ignore",
        "app.propagation.delivery_policy.unignore",
        "app.propagation.delivery_policy.prioritise",
        "app.propagation.delivery_policy.unprioritise",
    ] {
        assert!(
            registry_entries
                .iter()
                .any(|entry| entry["id"] == json!(operation_id)),
            "{operation_id}"
        );
    }
}
