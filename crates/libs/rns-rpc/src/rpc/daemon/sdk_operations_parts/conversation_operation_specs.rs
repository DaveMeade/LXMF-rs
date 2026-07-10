const CONVERSATION_SDK_OPERATION_SPECS: &[SdkOperationSpec] = &[SdkOperationSpec {
    id: "app.message.conversation.list",
    group: "messaging",
    kind: "query",
    transport_variant: "legacy_rpc",
    description: "List durable conversation summaries for app chat flows.",
    aliases: &["list_conversations"],
    required_capabilities: &[],
    rpc_method: "list_conversations",
}];
