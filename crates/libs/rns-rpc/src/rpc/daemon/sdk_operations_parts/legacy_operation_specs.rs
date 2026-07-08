const LEGACY_SDK_OPERATION_SPECS: &[SdkOperationSpec] = &[
    SdkOperationSpec {
        id: "app.message.history.list",
        group: "messaging",
        kind: "query",
        transport_variant: "legacy_rpc",
        description: "List message history records for app chat flows.",
        aliases: &["list_messages"],
        required_capabilities: &[],
        rpc_method: "list_messages",
    },
    SdkOperationSpec {
        id: "app.delivery.destination_hash",
        group: "identity",
        kind: "query",
        transport_variant: "legacy_rpc",
        description: "Resolve the runtime delivery destination hash.",
        aliases: &["status"],
        required_capabilities: &[],
        rpc_method: "status",
    },
];
