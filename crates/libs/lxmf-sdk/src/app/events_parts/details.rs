fn inbound_message_details(payload: &JsonValue) -> InboundMessageDetails {
    let message = payload.get("message").unwrap_or(payload);
    InboundMessageDetails {
        message_id: json_str(message, "id").or_else(|| json_str(payload, "message_id")),
        source_hash: json_str(message, "source").or_else(|| json_str(payload, "source_hash")),
        destination_hash: json_str(message, "destination")
            .or_else(|| json_str(payload, "destination_hash")),
        delivery_kind: json_str(payload, "delivery_kind"),
        lxmf_bytes_hex: json_str(payload, "lxmf_bytes_hex"),
        receipt_status: json_str(message, "receipt_status")
            .or_else(|| json_str(payload, "receipt_status")),
        signature_checked: nested_json_bool(message, &["fields", "_lxmf", "signature_checked"]),
        signature_status: nested_json_str(message, &["fields", "_lxmf", "signature_status"]),
        stamp_status: nested_json_str(message, &["fields", "_lxmf", "stamp_status"]),
    }
}

fn inbound_drop_details(payload: &JsonValue) -> InboundDropDetails {
    InboundDropDetails {
        reason: json_str(payload, "reason"),
        delivery_kind: json_str(payload, "delivery_kind"),
        raw_destination_hash: json_str(payload, "raw_destination_hash"),
        resolved_destination_hash: json_str(payload, "resolved_destination_hash"),
        source_hash: json_str(payload, "source_hash"),
        destination_hash: json_str(payload, "destination_hash"),
        dropped_message_id: json_str(payload, "dropped_message_id"),
        payload_mode: json_str(payload, "payload_mode"),
        bytes_len: payload.get("bytes_len").and_then(JsonValue::as_u64),
        detail: json_str(payload, "detail"),
        operation: json_str(payload, "operation"),
        transient_id: json_str(payload, "transient_id"),
        peer: json_str(payload, "peer").or_else(|| json_str(payload, "peer_id")),
    }
}

fn delivery_lifecycle_details(payload: &JsonValue) -> DeliveryLifecycleDetails {
    let message = payload.get("message").unwrap_or(payload);
    DeliveryLifecycleDetails {
        state: json_str(payload, "state")
            .or_else(|| normalized_receipt_state(payload).ok().flatten()),
        from: json_str(payload, "from"),
        to: json_str(payload, "to"),
        receipt_status: json_str(message, "receipt_status")
            .or_else(|| json_str(payload, "receipt_status"))
            .or_else(|| json_str(payload, "status")),
        delivery_kind: json_str(payload, "delivery_kind"),
        packet_hash: json_str(payload, "packet_hash"),
        resource_hash: json_str(payload, "resource_hash"),
        peer: json_str(payload, "peer").or_else(|| json_str(payload, "peer_id")),
        method: json_str(payload, "method"),
        bytes: payload.get("bytes").and_then(JsonValue::as_u64),
        link_id: json_str(payload, "link_id"),
        reason: json_str(payload, "reason").or_else(|| json_str(payload, "detail")),
    }
}
