#[test]
fn router_storage_policy_and_stats_are_typed_and_runtime_visible() {
    let daemon = RpcDaemon::test_instance();
    let policy = daemon
        .handle_rpc(rpc_request(
            80,
            "router_storage_policy_set",
            json!({
                "message_limit_bytes": 2_500_000,
                "information_limit_bytes": 750_000,
                "retain_node_lxms": true,
            }),
        ))
        .expect("set policy")
        .result
        .expect("policy");
    assert_eq!(policy["message_limit_bytes"].as_u64(), Some(3_000_000));
    assert_eq!(policy["information_limit_bytes"].as_u64(), Some(750_000));
    assert_eq!(policy["retain_node_lxms"].as_bool(), Some(true));

    let stats = daemon
        .handle_rpc(rpc_request(81, "router_stats", json!({})))
        .expect("router stats")
        .result
        .expect("stats");
    assert_eq!(stats["messages"].as_u64(), Some(0));
    assert_eq!(stats["message_bytes"].as_u64(), Some(0));
    assert_eq!(stats["storage_policy"], policy);
}
