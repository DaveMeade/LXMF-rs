use super::*;

#[test]
fn delivery_stamp_policy_uses_sdk_envelopes_and_typed_state() {
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_response_sequence_zmq_server(
        command_endpoint.clone(),
        vec![
            json!({
                "response": {
                    "operation_id": "app.delivery.stamp_policy.get",
                    "kind": "result",
                    "accepted": true,
                    "correlation_id": null,
                    "payload": {
                        "stamp_policy": {
                            "target_cost": 8,
                            "flexibility": 3,
                            "enforce": false
                        }
                    }
                }
            }),
            json!({
                "response": {
                    "operation_id": "app.delivery.stamp_policy.set",
                    "kind": "result",
                    "accepted": true,
                    "correlation_id": null,
                    "payload": {
                        "stamp_policy": {
                            "target_cost": 11,
                            "flexibility": 4,
                            "enforce": true
                        }
                    }
                }
            }),
        ],
        Arc::clone(&captured),
    );
    let mut config = ZmqPipelineBackendConfig::local_tcp(command_endpoint, response_endpoint);
    config.request_timeout = std::time::Duration::from_secs(2);
    let client = ZmqPipelineBackendClient::new(config).expect("zmq client");

    let policy = client.delivery_stamp_policy_get().expect("stamp policy get");
    let updated = client
        .delivery_stamp_policy_set(crate::DeliveryStampPolicyRequest {
            target_cost: Some(11),
            flexibility: Some(4),
            enforce: Some(true),
        })
        .expect("stamp policy set");

    assert_eq!(policy.stamp_policy["target_cost"], json!(8));
    assert_eq!(policy.policy_state.target_cost, Some(8));
    assert_eq!(policy.policy_state.flexibility, Some(3));
    assert!(!policy.policy_state.enforce);
    assert_eq!(updated.stamp_policy["flexibility"], json!(4));
    assert_eq!(updated.policy_state.target_cost, Some(11));
    assert_eq!(updated.policy_state.flexibility, Some(4));
    assert!(updated.policy_state.enforce);

    let captured = captured.lock().expect("captured requests");
    assert_eq!(captured[0].method, "sdk_envelope_execute_v2");
    assert_eq!(
        captured[0].params.as_ref().expect("params")["operation_id"],
        json!("app.delivery.stamp_policy.get")
    );
    assert_eq!(captured[0].params.as_ref().expect("params")["kind"], json!("query"));
    assert_eq!(captured[0].params.as_ref().expect("params")["payload"], json!({}));
    assert_eq!(captured[1].method, "sdk_envelope_execute_v2");
    assert_eq!(
        captured[1].params.as_ref().expect("params")["operation_id"],
        json!("app.delivery.stamp_policy.set")
    );
    assert_eq!(captured[1].params.as_ref().expect("params")["kind"], json!("command"));
    assert_eq!(
        captured[1].params.as_ref().expect("params")["payload"],
        json!({
            "target_cost": 11,
            "flexibility": 4,
            "enforce": true
        })
    );

    server.join().expect("server joined");
}
