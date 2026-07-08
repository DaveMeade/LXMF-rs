use super::*;

#[test]
fn delivery_ticket_generate_uses_zmq_sdk_envelope_and_decodes_result() {
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let captured = Arc::new(Mutex::new(None));
    let server = spawn_single_response_zmq_server(
        command_endpoint.clone(),
        json!({
            "response": {
                "operation_id": "app.delivery.ticket.generate",
                "kind": "result",
                "accepted": true,
                "correlation_id": null,
                "payload": {
                    "destination": "peer-ticket-zmq",
                    "ticket": "00112233445566778899aabbccddeeff",
                    "expires_at": 1_700_003_600i64,
                    "ttl_secs": 3_600,
                    "included": true
                }
            }
        }),
        Arc::clone(&captured),
    );
    let mut config = ZmqPipelineBackendConfig::local_tcp(command_endpoint, response_endpoint);
    config.request_timeout = std::time::Duration::from_secs(2);
    let client = ZmqPipelineBackendClient::new(config).expect("zmq client");

    let result = client
        .delivery_ticket_generate(crate::DeliveryTicketGenerateRequest {
            destination: "peer-ticket-zmq".to_string(),
            ttl_secs: Some(3_600),
        })
        .expect("delivery ticket generate");

    assert_eq!(result.destination, "peer-ticket-zmq");
    assert_eq!(result.ticket.as_deref(), Some("00112233445566778899aabbccddeeff"));
    assert_eq!(result.expires_at, Some(1_700_003_600));
    assert_eq!(result.ttl_secs, 3_600);
    assert!(result.included);
    assert_eq!(result.reason, None);
    let captured = captured.lock().expect("captured request");
    let request = captured.as_ref().expect("zmq request");
    assert_eq!(request.method, "sdk_envelope_execute_v2");
    assert_eq!(
        request.params,
        Some(json!({
            "operation_id": "app.delivery.ticket.generate",
            "kind": "command",
            "target": null,
            "correlation_id": null,
            "timeout_ms": null,
            "payload": {
                "destination": "peer-ticket-zmq",
                "ttl_secs": 3_600
            },
            "extensions": {}
        }))
    );
    server.join().expect("server joined");
}

#[test]
fn delivery_ticket_generate_preserves_ticket_interval_suppression() {
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let captured = Arc::new(Mutex::new(None));
    let server = spawn_single_response_zmq_server(
        command_endpoint.clone(),
        json!({
            "response": {
                "operation_id": "app.delivery.ticket.generate",
                "kind": "result",
                "accepted": true,
                "correlation_id": null,
                "payload": {
                    "destination": "peer-ticket-suppressed",
                    "ticket": null,
                    "expires_at": null,
                    "ttl_secs": 1_800,
                    "included": false,
                    "reason": "ticket_interval"
                }
            }
        }),
        Arc::clone(&captured),
    );
    let mut config = ZmqPipelineBackendConfig::local_tcp(command_endpoint, response_endpoint);
    config.request_timeout = std::time::Duration::from_secs(2);
    let client = ZmqPipelineBackendClient::new(config).expect("zmq client");

    let result = client
        .delivery_ticket_generate(crate::DeliveryTicketGenerateRequest {
            destination: "peer-ticket-suppressed".to_string(),
            ttl_secs: Some(1_800),
        })
        .expect("suppressed delivery ticket");

    assert_eq!(result.destination, "peer-ticket-suppressed");
    assert_eq!(result.ticket, None);
    assert_eq!(result.expires_at, None);
    assert_eq!(result.ttl_secs, 1_800);
    assert!(!result.included);
    assert_eq!(result.reason.as_deref(), Some("ticket_interval"));
    server.join().expect("server joined");
}
