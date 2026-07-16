macro_rules! zmq_backend_domain_methods {
    () => {
fn topic_create(
    &self,
    req: crate::domain::TopicCreateRequest,
) -> Result<crate::domain::TopicRecord, SdkError> {
    self.topic_create_impl(req)
}

fn topic_get(
    &self,
    topic_id: crate::domain::TopicId,
) -> Result<Option<crate::domain::TopicRecord>, SdkError> {
    self.topic_get_impl(topic_id)
}

fn topic_list(
    &self,
    req: crate::domain::TopicListRequest,
) -> Result<crate::domain::TopicListResult, SdkError> {
    self.topic_list_impl(req)
}

fn topic_subscribe(&self, req: crate::domain::TopicSubscriptionRequest) -> Result<Ack, SdkError> {
    self.topic_subscribe_impl(req)
}

fn topic_unsubscribe(&self, topic_id: crate::domain::TopicId) -> Result<Ack, SdkError> {
    self.topic_unsubscribe_impl(topic_id)
}

fn topic_publish(&self, req: crate::domain::TopicPublishRequest) -> Result<Ack, SdkError> {
    self.topic_publish_impl(req)
}

fn telemetry_query(
    &self,
    query: crate::domain::TelemetryQuery,
) -> Result<Vec<crate::domain::TelemetryPoint>, SdkError> {
    self.telemetry_query_impl(query)
}

fn telemetry_subscribe(&self, query: crate::domain::TelemetryQuery) -> Result<Ack, SdkError> {
    self.telemetry_subscribe_impl(query)
}

fn attachment_store(
    &self,
    req: crate::domain::AttachmentStoreRequest,
) -> Result<crate::domain::AttachmentMeta, SdkError> {
    self.attachment_store_impl(req)
}

fn attachment_get(
    &self,
    attachment_id: crate::domain::AttachmentId,
) -> Result<Option<crate::domain::AttachmentMeta>, SdkError> {
    self.attachment_get_impl(attachment_id)
}

fn attachment_list(
    &self,
    req: crate::domain::AttachmentListRequest,
) -> Result<crate::domain::AttachmentListResult, SdkError> {
    self.attachment_list_impl(req)
}

fn attachment_delete(&self, attachment_id: crate::domain::AttachmentId) -> Result<Ack, SdkError> {
    self.attachment_delete_impl(attachment_id)
}

fn attachment_download(
    &self,
    attachment_id: crate::domain::AttachmentId,
) -> Result<Ack, SdkError> {
    self.attachment_download_impl(attachment_id)
}

fn attachment_upload_start(
    &self,
    req: crate::domain::AttachmentUploadStartRequest,
) -> Result<crate::domain::AttachmentUploadSession, SdkError> {
    self.attachment_upload_start_impl(req)
}

fn attachment_upload_chunk(
    &self,
    req: crate::domain::AttachmentUploadChunkRequest,
) -> Result<crate::domain::AttachmentUploadChunkAck, SdkError> {
    self.attachment_upload_chunk_impl(req)
}

fn attachment_upload_commit(
    &self,
    req: crate::domain::AttachmentUploadCommitRequest,
) -> Result<crate::domain::AttachmentMeta, SdkError> {
    self.attachment_upload_commit_impl(req)
}

fn attachment_download_chunk(
    &self,
    req: crate::domain::AttachmentDownloadChunkRequest,
) -> Result<crate::domain::AttachmentDownloadChunk, SdkError> {
    self.attachment_download_chunk_impl(req)
}

fn attachment_associate_topic(
    &self,
    attachment_id: crate::domain::AttachmentId,
    topic_id: crate::domain::TopicId,
) -> Result<Ack, SdkError> {
    self.attachment_associate_topic_impl(attachment_id, topic_id)
}

fn marker_create(
    &self,
    req: crate::domain::MarkerCreateRequest,
) -> Result<crate::domain::MarkerRecord, SdkError> {
    self.marker_create_impl(req)
}

fn marker_list(
    &self,
    req: crate::domain::MarkerListRequest,
) -> Result<crate::domain::MarkerListResult, SdkError> {
    self.marker_list_impl(req)
}

fn marker_update_position(
    &self,
    req: crate::domain::MarkerUpdatePositionRequest,
) -> Result<crate::domain::MarkerRecord, SdkError> {
    self.marker_update_position_impl(req)
}

fn marker_delete(&self, req: crate::domain::MarkerDeleteRequest) -> Result<Ack, SdkError> {
    self.marker_delete_impl(req)
}

fn command_invoke(
    &self,
    req: crate::domain::RemoteCommandRequest,
) -> Result<crate::domain::RemoteCommandResponse, SdkError> {
    self.command_invoke_impl(req)
}

fn command_reply(
    &self,
    correlation_id: String,
    reply: crate::domain::RemoteCommandResponse,
) -> Result<Ack, SdkError> {
    self.command_reply_impl(correlation_id, reply)
}

fn command_session_get(
    &self,
    correlation_id: String,
) -> Result<Option<crate::domain::RemoteCommandSession>, SdkError> {
    self.command_session_get_impl(correlation_id)
}

fn command_session_list(
    &self,
    req: crate::domain::RemoteCommandSessionListRequest,
) -> Result<crate::domain::RemoteCommandSessionListResult, SdkError> {
    self.command_session_list_impl(req)
}

fn voice_session_open(
    &self,
    req: crate::domain::VoiceSessionOpenRequest,
) -> Result<crate::domain::VoiceSessionId, SdkError> {
    self.voice_session_open_impl(req)
}

fn voice_session_update(
    &self,
    req: crate::domain::VoiceSessionUpdateRequest,
) -> Result<crate::domain::VoiceSessionState, SdkError> {
    self.voice_session_update_impl(req)
}

fn voice_session_close(
    &self,
    session_id: crate::domain::VoiceSessionId,
) -> Result<Ack, SdkError> {
    self.voice_session_close_impl(session_id)
}

fn tick(&self, budget: crate::types::TickBudget) -> Result<crate::types::TickResult, SdkError> {
    self.tick_impl(budget)
}
    };
}
