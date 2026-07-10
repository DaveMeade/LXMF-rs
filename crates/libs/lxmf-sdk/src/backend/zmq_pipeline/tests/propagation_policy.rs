use super::*;

#[test]
fn propagation_delivery_policy_mutators_preserve_existing_fields() {
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_response_sequence_zmq_server(
        command_endpoint.clone(),
        vec![
            policy_response(
                "app.propagation.delivery_policy.get",
                true,
                &["dest-allow"],
                &["dest-deny"],
                &[],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.set",
                true,
                &["dest-allow", "dest-allow-new"],
                &["dest-deny"],
                &[],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.get",
                true,
                &["dest-allow", "dest-allow-new"],
                &["dest-deny"],
                &[],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.set",
                true,
                &["dest-allow-new"],
                &["dest-deny"],
                &[],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.get",
                false,
                &["dest-allow-new"],
                &["dest-deny"],
                &["dest-ignore"],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.set",
                false,
                &["dest-allow-new"],
                &["dest-deny"],
                &["dest-ignore", "dest-ignore-new"],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.get",
                false,
                &["dest-allow-new"],
                &["dest-deny"],
                &["dest-ignore", "dest-ignore-new"],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.set",
                false,
                &["dest-allow-new"],
                &["dest-deny"],
                &["dest-ignore-new"],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.get",
                true,
                &["dest-allow"],
                &["dest-deny"],
                &[],
                &["dest-priority"],
            ),
            policy_response(
                "app.propagation.delivery_policy.set",
                true,
                &["dest-allow"],
                &["dest-deny"],
                &[],
                &["dest-priority", "dest-priority-new"],
            ),
            policy_response(
                "app.propagation.delivery_policy.get",
                true,
                &["dest-allow"],
                &["dest-deny"],
                &[],
                &["dest-priority", "dest-priority-new"],
            ),
            policy_response(
                "app.propagation.delivery_policy.set",
                true,
                &["dest-allow"],
                &["dest-deny"],
                &[],
                &["dest-priority-new"],
            ),
        ],
        Arc::clone(&captured),
    );
    let mut config = ZmqPipelineBackendConfig::local_tcp(command_endpoint, response_endpoint);
    config.request_timeout = std::time::Duration::from_secs(2);
    let client = ZmqPipelineBackendClient::new(config).expect("zmq client");

    let allowed =
        client.propagation_delivery_policy_allow("dest-allow-new").expect("allow destination");
    let disallowed =
        client.propagation_delivery_policy_disallow("dest-allow").expect("disallow destination");
    let ignored =
        client.propagation_delivery_policy_ignore("dest-ignore-new").expect("ignore destination");
    let unignored =
        client.propagation_delivery_policy_unignore("dest-ignore").expect("unignore destination");
    let prioritised = client
        .propagation_delivery_policy_prioritise("dest-priority-new")
        .expect("prioritise destination");
    let unprioritised = client
        .propagation_delivery_policy_unprioritise("dest-priority")
        .expect("unprioritise destination");

    assert_eq!(
        allowed.policy_state.allowed_destinations,
        vec!["dest-allow".to_string(), "dest-allow-new".to_string()]
    );
    assert_eq!(disallowed.policy_state.allowed_destinations, vec!["dest-allow-new".to_string()]);
    assert_eq!(
        ignored.policy_state.ignored_destinations,
        vec!["dest-ignore".to_string(), "dest-ignore-new".to_string()]
    );
    assert_eq!(unignored.policy_state.ignored_destinations, vec!["dest-ignore-new".to_string()]);
    assert_eq!(
        prioritised.policy_state.prioritised_destinations,
        vec!["dest-priority".to_string(), "dest-priority-new".to_string()]
    );
    assert_eq!(
        unprioritised.policy_state.prioritised_destinations,
        vec!["dest-priority-new".to_string()]
    );

    let captured = captured.lock().expect("captured requests");
    let operation_ids = captured
        .iter()
        .map(|request| request.params.as_ref().expect("params")["operation_id"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        operation_ids,
        ["get", "set", "get", "set", "get", "set", "get", "set", "get", "set", "get", "set"]
            .map(|suffix| json!(format!("app.propagation.delivery_policy.{suffix}")))
    );
    assert_eq!(
        captured[1].params.as_ref().expect("params")["payload"],
        json!({
            "auth_required": true,
            "allowed_destinations": ["dest-allow", "dest-allow-new"],
            "denied_destinations": ["dest-deny"],
            "ignored_destinations": [],
            "prioritised_destinations": ["dest-priority"]
        })
    );
    assert_eq!(
        captured[7].params.as_ref().expect("params")["payload"],
        json!({
            "auth_required": false,
            "allowed_destinations": ["dest-allow-new"],
            "denied_destinations": ["dest-deny"],
            "ignored_destinations": ["dest-ignore-new"],
            "prioritised_destinations": ["dest-priority"]
        })
    );
    server.join().expect("server joined");
}

fn policy_response(
    operation_id: &str,
    auth_required: bool,
    allowed_destinations: &[&str],
    denied_destinations: &[&str],
    ignored_destinations: &[&str],
    prioritised_destinations: &[&str],
) -> JsonValue {
    json!({
        "response": {
            "operation_id": operation_id,
            "kind": "result",
            "accepted": true,
            "correlation_id": null,
            "payload": {
                "policy": {
                    "auth_required": auth_required,
                    "allowed_destinations": allowed_destinations,
                    "denied_destinations": denied_destinations,
                    "ignored_destinations": ignored_destinations,
                    "prioritised_destinations": prioritised_destinations
                }
            }
        }
    })
}

#[test]
fn propagation_delivery_policy_single_entry_methods_use_zmq_sdk_envelopes() {
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let server = spawn_response_sequence_zmq_server(
        command_endpoint.clone(),
        vec![
            policy_response(
                "app.propagation.delivery_policy.allow_destination",
                false,
                &["dest-allow"],
                &[],
                &[],
                &[],
            ),
            policy_response(
                "app.propagation.delivery_policy.disallow_destination",
                false,
                &[],
                &[],
                &[],
                &[],
            ),
            policy_response(
                "app.propagation.delivery_policy.prioritise_destination",
                false,
                &[],
                &[],
                &[],
                &["dest-priority"],
            ),
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
