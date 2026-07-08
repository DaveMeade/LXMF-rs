impl RpcDaemon {
    fn handle_rpc_legacy_propagation_request(
        &self,
        request_id: u64,
        method: &str,
        params: JsonValue,
    ) -> Result<RpcResponse, std::io::Error> {
        self.handle_rpc_legacy_propagation(RpcRequest {
            id: request_id,
            method: method.to_owned(),
            params: Some(params),
        })
    }
}
