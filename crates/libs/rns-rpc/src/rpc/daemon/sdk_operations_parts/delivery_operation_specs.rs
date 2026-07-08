const DELIVERY_SDK_OPERATION_SPECS: &[SdkOperationSpec] = &[
    SdkOperationSpec {
        id: "app.delivery.stamp_policy.get",
        group: "delivery",
        kind: "query",
        transport_variant: "legacy_rpc",
        description: "Return the local delivery stamp policy.",
        aliases: &["stamp_policy_get"],
        required_capabilities: &[],
        rpc_method: "stamp_policy_get",
    },
    SdkOperationSpec {
        id: "app.delivery.stamp_policy.set",
        group: "delivery",
        kind: "command",
        transport_variant: "legacy_rpc",
        description: "Update the local delivery stamp policy and return the resulting policy.",
        aliases: &["stamp_policy_set"],
        required_capabilities: &[],
        rpc_method: "stamp_policy_set",
    },
];
