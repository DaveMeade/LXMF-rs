use super::*;

fn client_with_response(
    response: JsonValue,
) -> (ZmqPipelineBackendClient, Arc<Mutex<Option<CapturedZmqRequest>>>, std::thread::JoinHandle<()>)
{
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let captured = Arc::new(Mutex::new(None));
    let server =
        spawn_single_response_zmq_server(command_endpoint.clone(), response, Arc::clone(&captured));
    let mut config = ZmqPipelineBackendConfig::local_tcp(command_endpoint, response_endpoint);
    config.request_timeout = std::time::Duration::from_secs(2);
    let client = ZmqPipelineBackendClient::new(config).expect("zmq client");
    (client, captured, server)
}

#[test]
fn topic_domain_uses_typed_zmq_method() {
    let (client, captured, server) = client_with_response(json!({
        "topic": {
            "topic_id": "topic-1",
            "topic_path": "ops/alpha",
            "created_ts_ms": 1,
            "updated_ts_ms": 1,
            "revision": 1,
            "extensions": {}
        }
    }));
    let request: crate::domain::TopicCreateRequest =
        serde_json::from_value(json!({ "topic_path": "ops/alpha" })).expect("topic request");

    let topic = client.topic_create(request).expect("topic create");

    assert_eq!(topic.topic_id.0, "topic-1");
    assert_eq!(
        captured.lock().expect("captured").as_ref().expect("request").method,
        "sdk_topic_create_v2"
    );
    server.join().expect("server joined");
}

#[test]
fn attachment_domain_uses_typed_zmq_method() {
    let (client, captured, server) = client_with_response(json!({
        "attachment": {
            "attachment_id": "attachment-1",
            "name": "report.txt",
            "content_type": "text/plain",
            "byte_len": 6,
            "checksum_sha256": "abc123",
            "created_ts_ms": 1,
            "expires_ts_ms": null,
            "topic_ids": [],
            "extensions": {}
        }
    }));
    let request: crate::domain::AttachmentStoreRequest = serde_json::from_value(json!({
        "name": "report.txt",
        "content_type": "text/plain",
        "bytes_base64": "cmVwb3J0",
        "expires_ts_ms": null,
        "topic_ids": [],
        "extensions": {}
    }))
    .expect("attachment request");

    let attachment = client.attachment_store(request).expect("attachment store");

    assert_eq!(attachment.attachment_id.0, "attachment-1");
    assert_eq!(
        captured.lock().expect("captured").as_ref().expect("request").method,
        "sdk_attachment_store_v2"
    );
    server.join().expect("server joined");
}

#[test]
fn remote_command_domain_uses_typed_zmq_method() {
    let (client, captured, server) = client_with_response(json!({
        "response": {
            "accepted": true,
            "payload": { "correlation_id": "corr-1" },
            "extensions": {}
        }
    }));
    let request: crate::domain::RemoteCommandRequest = serde_json::from_value(json!({
        "command": "app.runtime.status",
        "target": null,
        "payload": {},
        "timeout_ms": 1000,
        "extensions": { "correlation_id": "corr-1" }
    }))
    .expect("command request");

    let response = client.command_invoke(request).expect("command invoke");

    assert!(response.accepted);
    assert_eq!(response.payload["correlation_id"], json!("corr-1"));
    assert_eq!(
        captured.lock().expect("captured").as_ref().expect("request").method,
        "sdk_command_invoke_v2"
    );
    server.join().expect("server joined");
}

#[test]
fn manual_tick_remains_capability_gated_on_zmq() {
    let command_endpoint = unused_loopback_endpoint();
    let response_endpoint = unused_loopback_endpoint();
    let client = ZmqPipelineBackendClient::new(ZmqPipelineBackendConfig::local_tcp(
        command_endpoint,
        response_endpoint,
    ))
    .expect("zmq client");

    let error = client.tick(crate::types::TickBudget::new(1)).expect_err("capability required");

    assert_eq!(error.category, ErrorCategory::Capability);
}
