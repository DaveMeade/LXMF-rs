const TICKET_SDK_OPERATION_SPECS: &[SdkOperationSpec] = &[SdkOperationSpec {
    id: "app.delivery.ticket.generate",
    group: "delivery",
    kind: "command",
    transport_variant: "rpc",
    description: "Generate or reuse an outbound delivery ticket for a destination.",
    aliases: &["ticket_generate"],
    required_capabilities: &[],
    rpc_method: "ticket_generate",
}];
