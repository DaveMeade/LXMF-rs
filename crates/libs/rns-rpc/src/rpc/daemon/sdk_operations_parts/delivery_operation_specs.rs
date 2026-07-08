const DELIVERY_SDK_OPERATION_SPECS: &[SdkOperationSpec] = &[SdkOperationSpec {
    id: "app.delivery.link_available",
    group: "delivery",
    kind: "query",
    transport_variant: "legacy_rpc",
    description: "Return whether a direct or backchannel link exists for a delivery destination.",
    aliases: &["delivery_link_available"],
    required_capabilities: &[],
    rpc_method: "delivery_link_available",
}];
