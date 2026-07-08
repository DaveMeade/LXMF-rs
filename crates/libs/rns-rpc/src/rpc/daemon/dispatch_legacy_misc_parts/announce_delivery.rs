impl RpcDaemon {
    pub(super) fn handle_announce_delivery(
        &self,
        request: RpcRequest,
    ) -> Result<RpcResponse, std::io::Error> {
        let params = request.params.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing params")
        })?;
        let parsed: AnnounceDeliveryParams = serde_json::from_value(params)
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        let destination_hash = normalize_destination_hash_param(&parsed.destination_hash)?;
        let Some(local_destination_hash) = self
            .delivery_destination_hash
            .lock()
            .expect("delivery_destination_hash mutex poisoned")
            .clone()
        else {
            return Ok(RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError::new(
                    "DELIVERY_DESTINATION_UNAVAILABLE",
                    "local delivery destination is not configured",
                )),
            });
        };
        if !local_destination_hash.eq_ignore_ascii_case(destination_hash.as_str()) {
            return Ok(RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError::new(
                    "DELIVERY_DESTINATION_NOT_FOUND",
                    "requested destination is not a local delivery destination",
                )),
            });
        }
        let Some(bridge) = &self.announce_bridge else {
            return Ok(RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError::new(
                    "ANNOUNCE_BRIDGE_UNAVAILABLE",
                    "announce bridge is not configured",
                )),
            });
        };
        if let Err(err) = bridge.announce_delivery(destination_hash.as_str()) {
            return Ok(RpcResponse {
                id: request.id,
                result: None,
                error: Some(RpcError::new("ANNOUNCE_DELIVERY_FAILED", err.to_string())),
            });
        }
        let timestamp = now_i64();
        self.publish_event(RpcEvent {
            event_type: "announce_sent".into(),
            payload: json!({
                "timestamp": timestamp,
                "announce_id": request.id,
                "scope": "delivery",
                "destination_hash": destination_hash,
            }),
        });
        Ok(RpcResponse {
            id: request.id,
            result: Some(json!({
                "announce_id": request.id,
                "scope": "delivery",
                "destination_hash": destination_hash,
            })),
            error: None,
        })
    }
}
