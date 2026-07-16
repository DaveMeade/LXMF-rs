use crate::error::{code, ErrorCategory, SdkError};
use hmac::{Hmac, Mac};
use rns_rpc::RpcError;
use sha2::Sha256;

pub(super) fn sdk_error(category: ErrorCategory, message: impl Into<String>) -> SdkError {
    SdkError::new(code::INTERNAL, category, message)
}

pub(super) fn token_signature(secret: &str, payload: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("token shared secret must be non-empty");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

pub(super) fn map_rpc_error(error: RpcError) -> SdkError {
    let category = match error.category.as_deref() {
        Some("Validation") => ErrorCategory::Validation,
        Some("Capability") => ErrorCategory::Capability,
        Some("Config") => ErrorCategory::Config,
        Some("Policy") => ErrorCategory::Policy,
        Some("Security") => ErrorCategory::Security,
        Some("Transport") => ErrorCategory::Transport,
        Some("Timeout") => ErrorCategory::Timeout,
        Some("Runtime") => ErrorCategory::Runtime,
        _ => ErrorCategory::Internal,
    };
    SdkError::new(
        error.machine_code.as_deref().unwrap_or(error.code.as_str()),
        category,
        error.message,
    )
}
