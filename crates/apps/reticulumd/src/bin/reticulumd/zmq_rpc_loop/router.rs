use super::{
    handle_zmq_request_envelope, is_local_zmq_endpoint, is_recoverable_zmq_transport_error,
    validate_zmq_bind_security, zmq_io_error, ZMQ_RPC_WORKER_CONCURRENCY,
};
use rns_rpc::rpc::zmq;
use rns_rpc::RpcDaemon;
use std::io;
use std::sync::Arc;
use tokio::sync::{watch, Semaphore};
use zeromq::{RouterSocket, Socket, SocketRecv, SocketSend, ZmqMessage};

pub(crate) async fn run_zmq_router_loop_until(
    endpoint: String,
    require_auth_for_remote: bool,
    daemon: Arc<RpcDaemon>,
    mut shutdown: watch::Receiver<bool>,
) -> io::Result<()> {
    validate_zmq_bind_security(endpoint.as_str(), require_auth_for_remote, daemon.as_ref())?;
    let endpoint_requires_auth = require_auth_for_remote && !is_local_zmq_endpoint(&endpoint);
    let mut router = RouterSocket::new();
    router.bind(endpoint.as_str()).await.map_err(zmq_io_error)?;
    let (response_socket, mut request_socket) = router.split();
    let permits = Arc::new(Semaphore::new(ZMQ_RPC_WORKER_CONCURRENCY));
    log::info!("reticulumd listening on canonical zmq {}", endpoint);

    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            message = request_socket.recv() => {
                let message = match message {
                    Ok(message) => message,
                    Err(err) if is_recoverable_zmq_transport_error(&err) => {
                        log::warn!("[daemon] zmq router dropped client connection: {err}");
                        continue;
                    }
                    Err(err) => return Err(zmq_io_error(err)),
                };
                let frames = message.into_vec();
                if frames.len() != 2 {
                    log::warn!("[daemon] zmq router rejected {}-frame request", frames.len());
                    continue;
                }
                let identity = frames[0].clone();
                let encoded = frames[1].to_vec();
                let daemon = Arc::clone(&daemon);
                let permits = Arc::clone(&permits);
                let mut response_socket = response_socket.clone();
                tokio::spawn(async move {
                    let Ok(_permit) = permits.acquire_owned().await else {
                        return;
                    };
                    let envelope = match zmq::decode_envelope(&encoded) {
                        Ok(envelope) => envelope,
                        Err(err) => {
                            log::warn!("[daemon] zmq router envelope decode failed: {err}");
                            return;
                        }
                    };
                    let response = match handle_zmq_request_envelope(
                        daemon.as_ref(),
                        envelope,
                        endpoint_requires_auth,
                        true,
                    ) {
                        Ok(response) => response,
                        Err(reason) => {
                            log::warn!("[daemon] zmq router request rejected: {reason}");
                            return;
                        }
                    };
                    let encoded = match zmq::encode_envelope(&response) {
                        Ok(encoded) => encoded,
                        Err(err) => {
                            log::warn!("[daemon] zmq router response encode failed: {err}");
                            return;
                        }
                    };
                    let mut message = ZmqMessage::from(encoded);
                    message.push_front(identity);
                    if let Err(err) = response_socket.send(message).await {
                        log::warn!("[daemon] zmq router response send failed: {err}");
                    }
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rns_rpc::e2e_harness::{build_rpc_frame, parse_rpc_frame};
    use zeromq::DealerSocket;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn canonical_router_serves_concurrent_sdk_requests() {
        let reserved = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve zmq port");
        let port = reserved.local_addr().expect("reserved address").port();
        drop(reserved);
        let endpoint = format!("tcp://localhost:{port}");
        let daemon = Arc::new(RpcDaemon::test_instance());
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let server = tokio::spawn(run_zmq_router_loop_until(
            endpoint.clone(),
            true,
            Arc::clone(&daemon),
            shutdown_rx,
        ));
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (first, second) = tokio::join!(snapshot(&endpoint, 1), snapshot(&endpoint, 2));

        assert!(first.is_ok(), "first concurrent request failed: {first:?}");
        assert!(second.is_ok(), "second concurrent request failed: {second:?}");
        shutdown_tx.send(true).expect("request router shutdown");
        server.await.expect("router task join").expect("router shutdown");
    }

    async fn snapshot(endpoint: &str, request_id: u64) -> Result<serde_json::Value, String> {
        let mut client = DealerSocket::new();
        client.connect(endpoint).await.map_err(|error| error.to_string())?;
        let payload = build_rpc_frame(request_id, "sdk_snapshot_v2", Some(serde_json::json!({})))
            .map_err(|error| error.to_string())?;
        let mut envelope = rns_rpc::rpc::zmq::ZmqRpcEnvelope::request(
            format!("test-session-{request_id}"),
            request_id,
            "",
            payload,
            None,
        );
        envelope.response_endpoint = None;
        let encoded =
            rns_rpc::rpc::zmq::encode_envelope(&envelope).map_err(|error| error.to_string())?;
        client.send(ZmqMessage::from(encoded)).await.map_err(|error| error.to_string())?;
        let message = client.recv().await.map_err(|error| error.to_string())?;
        let bytes = Vec::<u8>::try_from(message).map_err(str::to_owned)?;
        let response =
            rns_rpc::rpc::zmq::decode_envelope(&bytes).map_err(|error| error.to_string())?;
        let rpc = parse_rpc_frame(&response.payload).map_err(|error| error.to_string())?;
        rpc.result.ok_or_else(|| format!("snapshot error: {:?}", rpc.error))
    }
}
