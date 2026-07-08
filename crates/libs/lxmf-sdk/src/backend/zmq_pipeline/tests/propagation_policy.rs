use super::*;

#[test]
fn propagation_delivery_policy_single_entry_methods_use_zmq_sdk_envelopes() {
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_response_sequence_zmq_server(
        command_endpoint.clone(),
        vec![
            json!({
                "response": {
                    "operation_id": "app.propagation.delivery_policy.allow_destination",
                    "kind": "result",
                    "accepted": true,
                    "correlation_id": null,
                    "payload": {
                        "policy": {
                            "auth_required": false,
                            "allowed_destinations": ["dest-allow"],
                            "denied_destinations": [],
                            "ignored_destinations": [],
                            "prioritised_destinations": []
                        }
                    }
                }
            }),
            json!({
                "response": {
                    "operation_id": "app.propagation.delivery_policy.disallow_destination",
                    "kind": "result",
                    "accepted": true,
                    "correlation_id": null,
                    "payload": {
                        "policy": {
                            "auth_required": false,
                            "allowed_destinations": [],
                            "denied_destinations": [],
                            "ignored_destinations": [],
                            "prioritised_destinations": []
                        }
                    }
                }
            }),
            json!({
                "response": {
                    "operation_id": "app.propagation.delivery_policy.prioritise_destination",
                    "kind": "result",
                    "accepted": true,
                    "correlation_id": null,
                    "payload": {
                        "policy": {
                            "auth_required": false,
                            "allowed_destinations": [],
                            "denied_destinations": [],
                            "ignored_destinations": [],
                            "prioritised_destinations": ["dest-priority"]
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

    let allowed = client
        .propagation_delivery_policy_allow_destination(
            crate::PropagationDeliveryPolicyEntryRequest { destination: "dest-allow".to_string() },
        )
        .expect("allow destination");
    let disallowed = client
        .propagation_delivery_policy_disallow_destination(
            crate::PropagationDeliveryPolicyEntryRequest { destination: "dest-allow".to_string() },
        )
        .expect("disallow destination");
    let prioritised = client
        .propagation_delivery_policy_prioritise_destination(
            crate::PropagationDeliveryPolicyEntryRequest {
                destination: "dest-priority".to_string(),
            },
        )
        .expect("prioritise destination");

    assert_eq!(allowed.policy_state.allowed_destinations, vec!["dest-allow".to_string()]);
    assert_eq!(disallowed.policy_state.allowed_destinations, Vec::<String>::new());
    assert_eq!(
        prioritised.policy_state.prioritised_destinations,
        vec!["dest-priority".to_string()]
    );

    let captured = captured.lock().expect("captured requests");
    let operation_ids = captured
        .iter()
        .map(|request| request.params.as_ref().expect("params")["operation_id"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        operation_ids,
        vec![
            json!("app.propagation.delivery_policy.allow_destination"),
            json!("app.propagation.delivery_policy.disallow_destination"),
            json!("app.propagation.delivery_policy.prioritise_destination"),
        ]
    );
    assert_eq!(
        captured[0].params.as_ref().expect("params")["payload"],
        json!({ "destination": "dest-allow" })
    );
    assert_eq!(
        captured[1].params.as_ref().expect("params")["payload"],
        json!({ "destination": "dest-allow" })
    );
    assert_eq!(
        captured[2].params.as_ref().expect("params")["payload"],
        json!({ "destination": "dest-priority" })
    );
    server.join().expect("server joined");
}
