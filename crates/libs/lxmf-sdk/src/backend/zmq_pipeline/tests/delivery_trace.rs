use super::*;

#[test]
fn envelope_execute_uses_zmq_sdk_method_and_preserves_delivery_trace_query() {
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let captured = Arc::new(Mutex::new(None));
    let server = spawn_single_response_zmq_server(
        command_endpoint.clone(),
        json!({
            "response": {
                "operation_id": "app.delivery.trace",
                "kind": "result",
                "accepted": true,
                "correlation_id": "trace-corr",
                "payload": {
                    "message_id": "trace-msg-1",
                    "transitions": [
                        { "status": "queued", "timestamp": 1 },
                        { "status": "sending", "timestamp": 2 }
                    ]
                },
                "extensions": {
                    "trace_source": "daemon"
                }
            }
        }),
        Arc::clone(&captured),
    );
    let mut config = ZmqPipelineBackendConfig::local_tcp(command_endpoint, response_endpoint);
    config.request_timeout = std::time::Duration::from_secs(2);
    let client = ZmqPipelineBackendClient::new(config).expect("zmq client");

    let response = client
        .envelope_execute(
            crate::app::Envelope::query(
                "app.delivery.trace",
                json!({
                    "message_id": "trace-msg-1"
                }),
            )
            .with_correlation_id("trace-corr"),
        )
        .expect("delivery trace envelope");

    assert_eq!(response.operation_id.as_str(), "app.delivery.trace");
    assert!(response.accepted);
    assert_eq!(response.correlation_id.as_deref(), Some("trace-corr"));
    assert_eq!(response.payload["message_id"], json!("trace-msg-1"));
    assert_eq!(response.payload["transitions"][1]["status"], json!("sending"));
    assert_eq!(response.extensions["trace_source"], json!("daemon"));
    let captured = captured.lock().expect("captured request");
    let request = captured.as_ref().expect("zmq request");
    assert_eq!(request.method, "sdk_envelope_execute_v2");
    let params = request.params.as_ref().expect("params");
    assert_eq!(params["operation_id"], json!("app.delivery.trace"));
    assert_eq!(params["kind"], json!("query"));
    assert_eq!(params["correlation_id"], json!("trace-corr"));
    assert_eq!(params["payload"]["message_id"], json!("trace-msg-1"));
    server.join().expect("server joined");
}
