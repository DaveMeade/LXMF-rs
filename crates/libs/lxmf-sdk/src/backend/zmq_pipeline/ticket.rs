use super::{code, ErrorCategory, SdkError, ZmqPipelineBackendClient};
use crate::app::{Envelope, EnvelopeResponse};
use crate::domain::{DeliveryTicketGenerateRequest, DeliveryTicketGenerateResult};

impl ZmqPipelineBackendClient {
    pub fn delivery_ticket_generate(
        &self,
        req: DeliveryTicketGenerateRequest,
    ) -> Result<DeliveryTicketGenerateResult, SdkError> {
        let params = serde_json::to_value(req).map_err(|err| {
            SdkError::new(code::INTERNAL, ErrorCategory::Internal, err.to_string())
        })?;
        let envelope = Envelope::command("app.delivery.ticket.generate", params);
        let params = serde_json::to_value(envelope).map_err(|err| {
            SdkError::new(code::INTERNAL, ErrorCategory::Internal, err.to_string())
        })?;
        let result = self.call_rpc("sdk_envelope_execute_v2", Some(params))?;
        let response: EnvelopeResponse =
            Self::decode_field_or_root(&result, "response", "delivery ticket response")?;
        Self::decode_value(response.payload, "delivery ticket payload")
    }
}
