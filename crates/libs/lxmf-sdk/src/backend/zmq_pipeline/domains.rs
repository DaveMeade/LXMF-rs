use super::{code, json, ErrorCategory, JsonValue, SdkError, ZmqPipelineBackendClient};
use crate::domain::{
    AttachmentDownloadChunk, AttachmentDownloadChunkRequest, AttachmentId, AttachmentListRequest,
    AttachmentListResult, AttachmentMeta, AttachmentStoreRequest, AttachmentUploadChunkAck,
    AttachmentUploadChunkRequest, AttachmentUploadCommitRequest, AttachmentUploadSession,
    AttachmentUploadStartRequest, MarkerCreateRequest, MarkerDeleteRequest, MarkerListRequest,
    MarkerListResult, MarkerRecord, MarkerUpdatePositionRequest, RemoteCommandRequest,
    RemoteCommandResponse, RemoteCommandSession, RemoteCommandSessionListRequest,
    RemoteCommandSessionListResult, TelemetryPoint, TelemetryQuery, TopicCreateRequest, TopicId,
    TopicListRequest, TopicListResult, TopicPublishRequest, TopicRecord, TopicSubscriptionRequest,
    VoiceSessionId, VoiceSessionOpenRequest, VoiceSessionState, VoiceSessionUpdateRequest,
};
use crate::types::Ack;
use serde::de::DeserializeOwned;
use serde::Serialize;

impl ZmqPipelineBackendClient {
    fn serialize_domain<T: Serialize>(value: T) -> Result<JsonValue, SdkError> {
        serde_json::to_value(value)
            .map_err(|err| SdkError::new(code::INTERNAL, ErrorCategory::Internal, err.to_string()))
    }

    fn call_domain<T: DeserializeOwned>(
        &self,
        method: &str,
        params: JsonValue,
        field: &str,
        context: &'static str,
    ) -> Result<T, SdkError> {
        let result = self.call_rpc(method, Some(params))?;
        Self::decode_field_or_root(&result, field, context)
    }

    fn call_domain_ack(&self, method: &str, params: JsonValue) -> Result<Ack, SdkError> {
        let result = self.call_rpc(method, Some(params))?;
        Ok(Self::parse_ack(&result))
    }

    pub(super) fn topic_create_impl(
        &self,
        req: TopicCreateRequest,
    ) -> Result<TopicRecord, SdkError> {
        self.call_domain(
            "sdk_topic_create_v2",
            Self::serialize_domain(req)?,
            "topic",
            "topic_create response",
        )
    }

    pub(super) fn topic_get_impl(
        &self,
        topic_id: TopicId,
    ) -> Result<Option<TopicRecord>, SdkError> {
        let result = self.call_rpc("sdk_topic_get_v2", Some(json!({ "topic_id": topic_id.0 })))?;
        if let Some(topic) = result.get("topic") {
            if topic.is_null() {
                return Ok(None);
            }
            return Self::decode_value(topic.clone(), "topic_get response").map(Some);
        }
        if result.is_null() {
            return Ok(None);
        }
        Self::decode_value(result, "topic_get response").map(Some)
    }

    pub(super) fn topic_list_impl(
        &self,
        req: TopicListRequest,
    ) -> Result<TopicListResult, SdkError> {
        self.call_domain(
            "sdk_topic_list_v2",
            Self::serialize_domain(req)?,
            "topic_list",
            "topic_list response",
        )
    }

    pub(super) fn topic_subscribe_impl(
        &self,
        req: TopicSubscriptionRequest,
    ) -> Result<Ack, SdkError> {
        self.call_domain_ack("sdk_topic_subscribe_v2", Self::serialize_domain(req)?)
    }

    pub(super) fn topic_unsubscribe_impl(&self, topic_id: TopicId) -> Result<Ack, SdkError> {
        self.call_domain_ack("sdk_topic_unsubscribe_v2", json!({ "topic_id": topic_id.0 }))
    }

    pub(super) fn topic_publish_impl(&self, req: TopicPublishRequest) -> Result<Ack, SdkError> {
        self.call_domain_ack("sdk_topic_publish_v2", Self::serialize_domain(req)?)
    }

    pub(super) fn telemetry_query_impl(
        &self,
        query: TelemetryQuery,
    ) -> Result<Vec<TelemetryPoint>, SdkError> {
        let result =
            self.call_rpc("sdk_telemetry_query_v2", Some(Self::serialize_domain(query)?))?;
        if let Some(points) = result.get("points") {
            return Self::decode_value(points.clone(), "telemetry_query points");
        }
        Self::decode_value(result, "telemetry_query points")
    }

    pub(super) fn telemetry_subscribe_impl(&self, query: TelemetryQuery) -> Result<Ack, SdkError> {
        self.call_domain_ack("sdk_telemetry_subscribe_v2", Self::serialize_domain(query)?)
    }

    pub(super) fn attachment_store_impl(
        &self,
        req: AttachmentStoreRequest,
    ) -> Result<AttachmentMeta, SdkError> {
        self.call_domain(
            "sdk_attachment_store_v2",
            Self::serialize_domain(req)?,
            "attachment",
            "attachment_store response",
        )
    }

    pub(super) fn attachment_get_impl(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Option<AttachmentMeta>, SdkError> {
        let result = self
            .call_rpc("sdk_attachment_get_v2", Some(json!({ "attachment_id": attachment_id.0 })))?;
        if let Some(attachment) = result.get("attachment") {
            if attachment.is_null() {
                return Ok(None);
            }
            return Self::decode_value(attachment.clone(), "attachment_get response").map(Some);
        }
        if result.is_null() {
            return Ok(None);
        }
        Self::decode_value(result, "attachment_get response").map(Some)
    }

    pub(super) fn attachment_list_impl(
        &self,
        req: AttachmentListRequest,
    ) -> Result<AttachmentListResult, SdkError> {
        self.call_domain(
            "sdk_attachment_list_v2",
            Self::serialize_domain(req)?,
            "attachment_list",
            "attachment_list response",
        )
    }

    pub(super) fn attachment_delete_impl(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Ack, SdkError> {
        self.call_domain_ack(
            "sdk_attachment_delete_v2",
            json!({ "attachment_id": attachment_id.0 }),
        )
    }

    pub(super) fn attachment_download_impl(
        &self,
        attachment_id: AttachmentId,
    ) -> Result<Ack, SdkError> {
        self.call_domain_ack(
            "sdk_attachment_download_v2",
            json!({ "attachment_id": attachment_id.0 }),
        )
    }

    pub(super) fn attachment_upload_start_impl(
        &self,
        req: AttachmentUploadStartRequest,
    ) -> Result<AttachmentUploadSession, SdkError> {
        self.call_domain(
            "sdk_attachment_upload_start_v2",
            Self::serialize_domain(req)?,
            "upload",
            "attachment_upload_start response",
        )
    }

    pub(super) fn attachment_upload_chunk_impl(
        &self,
        req: AttachmentUploadChunkRequest,
    ) -> Result<AttachmentUploadChunkAck, SdkError> {
        self.call_domain(
            "sdk_attachment_upload_chunk_v2",
            Self::serialize_domain(req)?,
            "upload_chunk",
            "attachment_upload_chunk response",
        )
    }

    pub(super) fn attachment_upload_commit_impl(
        &self,
        req: AttachmentUploadCommitRequest,
    ) -> Result<AttachmentMeta, SdkError> {
        self.call_domain(
            "sdk_attachment_upload_commit_v2",
            Self::serialize_domain(req)?,
            "attachment",
            "attachment_upload_commit response",
        )
    }

    pub(super) fn attachment_download_chunk_impl(
        &self,
        req: AttachmentDownloadChunkRequest,
    ) -> Result<AttachmentDownloadChunk, SdkError> {
        self.call_domain(
            "sdk_attachment_download_chunk_v2",
            Self::serialize_domain(req)?,
            "download_chunk",
            "attachment_download_chunk response",
        )
    }

    pub(super) fn attachment_associate_topic_impl(
        &self,
        attachment_id: AttachmentId,
        topic_id: TopicId,
    ) -> Result<Ack, SdkError> {
        self.call_domain_ack(
            "sdk_attachment_associate_topic_v2",
            json!({ "attachment_id": attachment_id.0, "topic_id": topic_id.0 }),
        )
    }

    pub(super) fn marker_create_impl(
        &self,
        req: MarkerCreateRequest,
    ) -> Result<MarkerRecord, SdkError> {
        self.call_domain(
            "sdk_marker_create_v2",
            Self::serialize_domain(req)?,
            "marker",
            "marker_create response",
        )
    }

    pub(super) fn marker_list_impl(
        &self,
        req: MarkerListRequest,
    ) -> Result<MarkerListResult, SdkError> {
        self.call_domain(
            "sdk_marker_list_v2",
            Self::serialize_domain(req)?,
            "marker_list",
            "marker_list response",
        )
    }

    pub(super) fn marker_update_position_impl(
        &self,
        req: MarkerUpdatePositionRequest,
    ) -> Result<MarkerRecord, SdkError> {
        self.call_domain(
            "sdk_marker_update_position_v2",
            Self::serialize_domain(req)?,
            "marker",
            "marker_update_position response",
        )
    }

    pub(super) fn marker_delete_impl(&self, req: MarkerDeleteRequest) -> Result<Ack, SdkError> {
        self.call_domain_ack("sdk_marker_delete_v2", Self::serialize_domain(req)?)
    }

    pub(super) fn command_invoke_impl(
        &self,
        req: RemoteCommandRequest,
    ) -> Result<RemoteCommandResponse, SdkError> {
        self.call_domain(
            "sdk_command_invoke_v2",
            Self::serialize_domain(req)?,
            "response",
            "command_invoke response",
        )
    }

    pub(super) fn command_reply_impl(
        &self,
        correlation_id: String,
        reply: RemoteCommandResponse,
    ) -> Result<Ack, SdkError> {
        let mut params = Self::serialize_domain(reply)?;
        let object = params.as_object_mut().ok_or_else(|| {
            SdkError::new(
                code::INTERNAL,
                ErrorCategory::Internal,
                "command_reply payload serialization did not produce an object",
            )
        })?;
        object.insert("correlation_id".to_owned(), JsonValue::String(correlation_id));
        self.call_domain_ack("sdk_command_reply_v2", params)
    }

    pub(super) fn command_session_get_impl(
        &self,
        correlation_id: String,
    ) -> Result<Option<RemoteCommandSession>, SdkError> {
        let result = self.call_rpc(
            "sdk_command_session_get_v2",
            Some(json!({ "correlation_id": correlation_id })),
        )?;
        let payload = Self::decode_field_or_root::<JsonValue>(
            &result,
            "session",
            "command_session_get response",
        )?;
        if payload.is_null() {
            return Ok(None);
        }
        Self::decode_value(payload, "command_session_get response").map(Some)
    }

    pub(super) fn command_session_list_impl(
        &self,
        req: RemoteCommandSessionListRequest,
    ) -> Result<RemoteCommandSessionListResult, SdkError> {
        self.call_domain(
            "sdk_command_session_list_v2",
            Self::serialize_domain(req)?,
            "session_list",
            "command_session_list response",
        )
    }

    pub(super) fn voice_session_open_impl(
        &self,
        req: VoiceSessionOpenRequest,
    ) -> Result<VoiceSessionId, SdkError> {
        let result =
            self.call_rpc("sdk_voice_session_open_v2", Some(Self::serialize_domain(req)?))?;
        if let Some(session_id) = result.get("session_id").and_then(JsonValue::as_str) {
            return Ok(VoiceSessionId(session_id.to_owned()));
        }
        Self::decode_value(result, "voice_session_open response")
    }

    pub(super) fn voice_session_update_impl(
        &self,
        req: VoiceSessionUpdateRequest,
    ) -> Result<VoiceSessionState, SdkError> {
        let result =
            self.call_rpc("sdk_voice_session_update_v2", Some(Self::serialize_domain(req)?))?;
        if let Some(state) = result.get("state") {
            return Self::decode_value(state.clone(), "voice_session_update response");
        }
        Self::decode_value(result, "voice_session_update response")
    }

    pub(super) fn voice_session_close_impl(
        &self,
        session_id: VoiceSessionId,
    ) -> Result<Ack, SdkError> {
        self.call_domain_ack("sdk_voice_session_close_v2", json!({ "session_id": session_id.0 }))
    }

    pub(super) fn tick_impl(
        &self,
        budget: crate::types::TickBudget,
    ) -> Result<crate::types::TickResult, SdkError> {
        if !self.has_capability("sdk.capability.manual_tick") {
            return Err(SdkError::capability_disabled("sdk.capability.manual_tick"));
        }
        if budget.max_work_items == 0 {
            return Err(SdkError::new(
                code::VALIDATION_INVALID_ARGUMENT,
                ErrorCategory::Validation,
                "tick budget max_work_items must be greater than zero",
            )
            .with_user_actionable(true));
        }

        let started = std::time::Instant::now();
        let mut cursor =
            self.manual_tick_cursor.read().expect("manual_tick_cursor rwlock poisoned").clone();
        let mut processed_items = 0_usize;
        while processed_items < budget.max_work_items {
            if budget
                .max_duration_ms
                .is_some_and(|max_ms| started.elapsed() >= std::time::Duration::from_millis(max_ms))
            {
                break;
            }
            let remaining = budget.max_work_items.saturating_sub(processed_items);
            let batch = <Self as crate::backend::SdkBackend>::poll_events(
                self,
                cursor.clone(),
                remaining.min(256),
            )?;
            let next_cursor = batch.next_cursor.clone();
            processed_items = processed_items.saturating_add(batch.events.len());
            let no_progress = batch.events.is_empty()
                || cursor.as_ref().is_some_and(|current| current == &next_cursor);
            cursor = Some(next_cursor);
            if no_progress {
                break;
            }
        }
        *self.manual_tick_cursor.write().expect("manual_tick_cursor rwlock poisoned") = cursor;

        let yielded = processed_items >= budget.max_work_items;
        Ok(crate::types::TickResult {
            processed_items,
            yielded,
            next_recommended_delay_ms: Some(if yielded { 0 } else { 10 }),
        })
    }
}
