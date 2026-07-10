fn delivery_operation_entries() -> Vec<OperationEntry> {
    vec![
        OperationEntry::new(
            "app.delivery.send",
            "delivery",
            OperationKind::Command,
            TransportVariant::Rpc,
            "Queue one outbound message for delivery.",
        )
        .with_alias("sdk_send_v2"),
        OperationEntry::new(
            "app.delivery.send_batch",
            "delivery",
            OperationKind::Command,
            TransportVariant::Rpc,
            "Queue a batch of outbound messages for delivery.",
        )
        .with_alias("sdk_send_batch_v2")
        .with_required_capability("sdk.capability.batch_send"),
        OperationEntry::new(
            "app.delivery.status",
            "delivery",
            OperationKind::Query,
            TransportVariant::Rpc,
            "Return delivery state for a specific message id.",
        )
        .with_alias("sdk_status_v2"),
        OperationEntry::new(
            "app.delivery.trace",
            "delivery",
            OperationKind::Query,
            TransportVariant::Rpc,
            "Return delivery trace transitions for a specific message id.",
        )
        .with_alias("message_delivery_trace"),
        OperationEntry::new(
            "app.delivery.cancel",
            "delivery",
            OperationKind::Command,
            TransportVariant::Rpc,
            "Cancel a queued outbound message when it has not reached a terminal state.",
        )
        .with_alias("sdk_cancel_message_v2"),
        OperationEntry::new(
            "app.delivery.ticket.generate",
            "delivery",
            OperationKind::Command,
            TransportVariant::Rpc,
            "Generate or reuse an outbound delivery ticket for a destination.",
        )
        .with_alias("ticket_generate"),
        OperationEntry::new(
            "app.delivery.stamp_policy.get",
            "delivery",
            OperationKind::Query,
            TransportVariant::LegacyRpc,
            "Return the local delivery stamp policy.",
        )
        .with_alias("stamp_policy_get"),
        OperationEntry::new(
            "app.delivery.stamp_policy.set",
            "delivery",
            OperationKind::Command,
            TransportVariant::LegacyRpc,
            "Update the local delivery stamp policy and return the resulting policy.",
        )
        .with_alias("stamp_policy_set"),
    ]
}
