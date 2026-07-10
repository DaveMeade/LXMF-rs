use super::super::with_interface_runtime_metadata;
use rns_rpc::InterfaceRecord;
use rns_transport::transport::ReticulumPathTableRestoreSkipped;
use serde_json::{json, Value as JsonValue};

#[derive(Clone, Debug)]
pub(super) enum PathTableRestoreStatus {
    Ok { restored_active_paths: usize, skipped: ReticulumPathTableRestoreSkipped },
    Error { message: String },
}

impl PathTableRestoreStatus {
    fn runtime_json(&self) -> JsonValue {
        match self {
            Self::Ok { restored_active_paths, skipped } => {
                json!({
                    "status": "ok",
                    "restored_active_paths": restored_active_paths,
                    "skipped": skipped_runtime_json(skipped),
                })
            }
            Self::Error { message } => {
                json!({
                    "status": "error",
                    "error": message,
                })
            }
        }
    }
}

fn skipped_runtime_json(skipped: &ReticulumPathTableRestoreSkipped) -> JsonValue {
    json!({
        "active_unmapped_interface": skipped.active_unmapped_interface,
        "active_expired": skipped.active_expired,
        "active_missing_cached_announce": skipped.active_missing_cached_announce,
        "active_invalid_cached_announce": skipped.active_invalid_cached_announce,
        "active_mismatched_cached_announce": skipped.active_mismatched_cached_announce,
        "active_identity_conflict": skipped.active_identity_conflict,
        "tunnel_duplicate_packet_hash": skipped.tunnel_duplicate_packet_hash,
        "tunnel_expired": skipped.tunnel_expired,
        "tunnel_missing_cached_announce": skipped.tunnel_missing_cached_announce,
        "tunnel_invalid_cached_announce": skipped.tunnel_invalid_cached_announce,
        "tunnel_mismatched_cached_announce": skipped.tunnel_mismatched_cached_announce,
        "tunnel_identity_conflict": skipped.tunnel_identity_conflict,
    })
}

pub(super) fn mark_path_table_restore_status(
    record: &mut InterfaceRecord,
    status: &PathTableRestoreStatus,
) {
    with_interface_runtime_metadata(record, |runtime| {
        let reticulum = runtime.entry("reticulum".to_string()).or_insert_with(|| json!({}));
        if !reticulum.is_object() {
            *reticulum = json!({});
        }
        if let Some(reticulum) = reticulum.as_object_mut() {
            reticulum.insert("path_table_restore".to_string(), status.runtime_json());
        }
    });
}

pub(super) fn mark_path_table_restore_status_on_enabled_interfaces(
    records: &mut [InterfaceRecord],
    status: &PathTableRestoreStatus,
) {
    for record in records {
        if record.enabled {
            mark_path_table_restore_status(record, status);
        }
    }
}
