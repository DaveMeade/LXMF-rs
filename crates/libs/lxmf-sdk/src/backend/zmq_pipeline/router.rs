use super::*;

impl ZmqPipelineBackendClient {
    pub(super) fn router_stats_impl(&self) -> Result<crate::RouterStats, SdkError> {
        self.router_call("router_stats", json!({}), "router stats")
    }

    pub(super) fn router_storage_policy_impl(
        &self,
    ) -> Result<crate::RouterStoragePolicy, SdkError> {
        self.router_call("router_storage_policy_get", json!({}), "router storage policy")
    }

    pub(super) fn set_router_storage_policy_impl(
        &self,
        patch: crate::RouterStoragePolicyPatch,
    ) -> Result<crate::RouterStoragePolicy, SdkError> {
        let params = serde_json::to_value(patch).map_err(|error| {
            SdkError::new(code::INTERNAL, ErrorCategory::Internal, error.to_string())
        })?;
        self.router_call("router_storage_policy_set", params, "router storage policy")
    }

    fn router_call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: JsonValue,
        label: &str,
    ) -> Result<T, SdkError> {
        let result = self.call_rpc(method, Some(params))?;
        serde_json::from_value(result).map_err(|error| {
            SdkError::new(
                code::INTERNAL,
                ErrorCategory::Internal,
                format!("invalid {label}: {error}"),
            )
        })
    }
}
