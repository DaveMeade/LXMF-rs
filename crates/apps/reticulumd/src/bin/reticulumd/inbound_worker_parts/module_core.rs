use reticulum_daemon::receipt_bridge::ReceiptEvent;

use crate::bridge::emit_receipt_event;
use crate::direct_backchannel::DirectBackchannelLinks;

use rns_rpc::{RpcDaemon, RpcRequest};

use rns_transport::destination::link::{Link, LinkEvent};

use rns_transport::destination::{DestinationName, SingleInputDestination};

use rns_transport::hash::{AddressHash, Hash};

use rns_transport::identity::{DecryptIdentity, Identity};

use rns_transport::packet::{
    ContextFlag, DestinationType, Header, HeaderType, IfacFlag, Packet, PacketContext,
    PacketDataBuffer, PacketType, PropagationType,
};

use rns_transport::resource::ResourceEventKind;

use rns_transport::transport::Transport;

use routing::InboundLxmfDestination;

use serde_json::{json, Value};

use sha2::Digest;

use std::sync::Arc;

pub(super) fn spawn_inbound_worker(
    daemon: Arc<RpcDaemon>,
    transport: Arc<Transport>,
    control: PropagationControlContext,
    direct_backchannel_links: Option<DirectBackchannelLinks>,
    receipt_tx: tokio::sync::mpsc::Sender<ReceiptEvent>,
    outbound_resource_map: OutboundResourceMap,
) {
    if control.enabled {
        control::spawn_control_worker(daemon.clone(), transport.clone(), control.clone());
    }
    let resource_control = control.clone();
    spawn_packet_inbound_worker(
        daemon.clone(),
        transport.clone(),
        control,
        direct_backchannel_links,
    );
    tokio::spawn(async move {
        let local_delivery_destination = routing::local_delivery_destination_hash(
            resource_control.delivery_destination.as_ref(),
        )
        .await;
        let mut rx = transport.resource_events();
        loop {
            if let Ok(event) = rx.recv().await {
                match event.kind {
                    ResourceEventKind::Complete(complete) => {
                        if let Some(destination) = routing::resolve_resource_destination(
                            transport.as_ref(),
                            &event.link_id,
                            local_delivery_destination,
                        )
                        .await
                        {
                            match destination {
                                InboundLxmfDestination::Delivery(destination) => {
                                    delivery_events::accept_delivery_resource(
                                        daemon.as_ref(),
                                        transport.as_ref(),
                                        destination,
                                        &complete.data,
                                    )
                                    .await;
                                }
                                InboundLxmfDestination::Propagation => {
                                    if complete.is_request {
                                        match resource_request_id(&complete.request_id) {
                                            Some(request_id) => {
                                                if let Err(error) =
                                                    control::handle_resource_control_request(
                                                        daemon.as_ref(),
                                                        transport.as_ref(),
                                                        &resource_control,
                                                        &event.link_id,
                                                        &complete.data,
                                                        request_id,
                                                        true,
                                                    )
                                                    .await
                                                {
                                                    log::error!(
                                                        "[daemon-control] failed to handle propagation resource request link={} error={}",
                                                        event.link_id,
                                                        error
                                                    );
                                                }
                                            }
                                            None => {
                                                log::warn!(
                                                    "[daemon-control] ignoring propagation resource request with invalid request id link={}",
                                                    event.link_id
                                                );
                                            }
                                        }
                                        continue;
                                    }
                                    if complete.is_response {
                                        continue;
                                    }
                                    let remote_peer = remote_propagation_peer_for_link(
                                        transport.as_ref(),
                                        &event.link_id,
                                    )
                                    .await;
                                    let peer_link_validated =
                                        match resource_control.validated_peer_links.lock() {
                                            Ok(guard) => guard.contains(&event.link_id),
                                            Err(err) => {
                                                log::warn!(
                                                    "[daemon-rx] failed to read validated peer links for link={}: {err}",
                                                    hex::encode(event.link_id.as_slice())
                                                );
                                                false
                                            }
                                        };
                                    if let Err(error) =
                                        propagation::ingest_propagation_resource_from_peer_with_transport(
                                            daemon.as_ref(),
                                            &complete.data,
                                            resource_control.delivery_destination.as_ref(),
                                            remote_peer.as_deref(),
                                            peer_link_validated,
                                            Some(transport.as_ref()),
                                        )
                                        .await
                                    {
                                        log::debug!(
                                            "[daemon-rx] dropping inbound propagation resource: {}",
                                            error
                                        );
                                    }
                                }
                            }
                        }
                    }
                    ResourceEventKind::OutboundComplete => {
                        handle_outbound_resource_completion(
                            daemon.as_ref(),
                            &outbound_resource_map,
                            &receipt_tx,
                            &event.hash,
                        );
                    }
                    ResourceEventKind::OutboundFailed => {
                        handle_outbound_resource_failure(
                            daemon.as_ref(),
                            &outbound_resource_map,
                            &receipt_tx,
                            &event.hash,
                        );
                    }
                    ResourceEventKind::OutboundCancelled => {
                        let resource_hash_hex = hex::encode(event.hash.as_slice());
                        let _ = take_outbound_resource_tracking(
                            &outbound_resource_map,
                            resource_hash_hex.as_str(),
                        );
                    }
                    ResourceEventKind::InboundFailed(failure) => {
                        log::warn!(
                            "[daemon-rx] inbound resource failed link={} hash={} reason={} received={}/{}",
                            event.link_id,
                            event.hash,
                            failure.reason,
                            failure.progress.received_parts,
                            failure.progress.total_parts
                        );
                    }
                    ResourceEventKind::SegmentComplete(segment) => {
                        log::debug!(
                            "[daemon-rx] resource segment complete link={} original_hash={} segment={}/{}",
                            event.link_id,
                            segment.original_hash,
                            segment.segment_index,
                            segment.total_segments
                        );
                    }
                    ResourceEventKind::Progress(_) => {}
                }
            }
        }
    });
}

fn resource_request_id(request_id: &Option<Vec<u8>>) -> Option<[u8; 16]> {
    let bytes = request_id.as_ref()?;
    if bytes.len() != 16 {
        return None;
    }
    let mut out = [0u8; 16];
    out.copy_from_slice(bytes.as_slice());
    Some(out)
}

async fn remote_propagation_peer_for_link(
    transport: &Transport,
    link_id: &AddressHash,
) -> Option<String> {
    if let Some(link) = transport.find_in_link(link_id).await {
        let guard = link.lock().await;
        let identity = guard.identified_peer_identity().unwrap_or_else(|| guard.peer_identity());
        return Some(propagation_destination_hash_for_identity(identity));
    }
    if let Some(link) = transport.find_out_link(link_id).await {
        let guard = link.lock().await;
        let identity = guard.identified_peer_identity().unwrap_or_else(|| guard.peer_identity());
        return Some(propagation_destination_hash_for_identity(identity));
    }
    None
}

fn propagation_destination_hash_for_identity(identity: &Identity) -> String {
    let name = DestinationName::new("lxmf", "propagation");
    let hash = sha2::Sha256::new()
        .chain_update(name.as_name_hash_slice())
        .chain_update(identity.address_hash.as_slice())
        .finalize();
    hex::encode(&hash[..16])
}

fn handle_outbound_resource_completion(
    daemon: &RpcDaemon,
    outbound_resource_map: &OutboundResourceMap,
    receipt_tx: &tokio::sync::mpsc::Sender<ReceiptEvent>,
    resource_hash: &Hash,
) {
    let resource_hash_hex = hex::encode(resource_hash.as_slice());
    match take_outbound_resource_tracking(outbound_resource_map, resource_hash_hex.as_str()) {
        Ok(tracking) => {
            daemon.record_outbound_peer_sent(&tracking.peer, tracking.bytes);
            emit_receipt_event(
                receipt_tx,
                ReceiptEvent::new(tracking.message_id, tracking.sent_status)
                    .with_resource_hash(resource_hash_hex)
                    .with_peer(tracking.peer)
                    .with_delivery_kind("resource-complete")
                    .with_bytes(tracking.bytes),
            );
        }
        Err(err) => {
            log::warn!("[daemon-rx] outbound resource completion without tracking hash={}: {err}", resource_hash_hex);
        }
    }
}

fn handle_outbound_resource_failure(
    daemon: &RpcDaemon,
    outbound_resource_map: &OutboundResourceMap,
    receipt_tx: &tokio::sync::mpsc::Sender<ReceiptEvent>,
    resource_hash: &Hash,
) {
    let resource_hash_hex = hex::encode(resource_hash.as_slice());
    match take_outbound_resource_tracking(outbound_resource_map, resource_hash_hex.as_str()) {
        Ok(tracking) => {
            daemon.record_outbound_peer_activity(&tracking.peer, tracking.bytes, false);
            emit_receipt_event(
                receipt_tx,
                ReceiptEvent::new(tracking.message_id, "failed: resource transfer timed out")
                    .with_resource_hash(resource_hash_hex)
                    .with_peer(tracking.peer)
                    .with_delivery_kind("resource-failed")
                    .with_bytes(tracking.bytes),
            );
        }
        Err(err) => {
            log::warn!("[daemon-rx] outbound resource failure without tracking hash={}: {err}", resource_hash_hex);
        }
    }
}

/// Whether the link identified by `link_id` is anchored on an
/// `lxmf/delivery` destination — the direct-backchannel cache should only
/// ever be populated from delivery links, never `lxmf/propagation`/
/// `propagation.control` ones (see `spawn_packet_inbound_worker`'s
/// `PeerIdentified` consumer for why). Checks in-links then out-links,
/// mirroring `routing::resolve_resource_destination`'s own lookup order;
/// an unresolvable `link_id` (already closed/evicted) is treated as
/// not-delivery, fail-closed rather than caching on a guess.
async fn is_delivery_backchannel_link(transport: &Transport, link_id: &AddressHash) -> bool {
    let link = match transport.find_in_link(link_id).await {
        Some(link) => Some(link),
        None => transport.find_out_link(link_id).await,
    };
    match link {
        Some(link) => routing::is_lxmf_delivery_destination(link.lock().await.destination()),
        None => false,
    }
}

fn spawn_packet_inbound_worker(
    daemon: Arc<RpcDaemon>,
    transport: Arc<Transport>,
    control: PropagationControlContext,
    direct_backchannel_links: Option<DirectBackchannelLinks>,
) {
    // meshage fork — `LinkIdentify` packets stopped surfacing as plain
    // `received_data_events()` payloads once #477 gave them their own
    // dedicated `LinkEvent::PeerIdentified` event (matching upstream
    // Python Reticulum's own separate identify handling), so the direct
    // backchannel cache has to be populated from that event directly
    // instead — the crate now hands back an already-parsed, already-
    // signature-verified `Identity`, so this no longer needs its own
    // `parse_link_identify_payload` reimplementation. A backchannel link
    // can be either side's outbound link to the other, so both event
    // streams are covered, mirroring how `Transport::new`'s own
    // `spawn_link_data_forwarder` covers both directions. See #477's
    // review discussion.
    //
    // `PeerIdentified` fires for ANY link, not just delivery ones — a peer
    // also sends `LinkIdentify` on `lxmf/propagation`/`propagation.control`
    // links (e.g. `bridge_remote_request.rs`, propagation download flows).
    // The old inline-packet path only ever recorded a backchannel entry
    // after `resolve_packet_destination` had resolved to
    // `InboundLxmfDestination::Delivery`; that filter has to be
    // reconstructed here by resolving the actual `Link` this event came
    // from and checking its destination aspect, or a propagation/control
    // link gets cached as if it were the peer's delivery backchannel — a
    // later direct send would then reuse that link and the receiver would
    // resolve the payload as propagation/control instead of delivery
    // (flagged in Codex review; see PR #477 discussion).
    if let Some(backchannel_links) = direct_backchannel_links.clone() {
        for mut rx in [transport.in_link_events(), transport.out_link_events()] {
            let backchannel_links = backchannel_links.clone();
            let link_transport = transport.clone();
            tokio::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            if let LinkEvent::PeerIdentified(identity) = event.event {
                                if !is_delivery_backchannel_link(&link_transport, &event.id).await {
                                    log::debug!(
                                        "[daemon-rx] skipping non-delivery backchannel identify destination={} link={}",
                                        identity.address_hash,
                                        event.id
                                    );
                                    continue;
                                }
                                backchannel_links.record_identified_link(&identity, event.id);
                                log::debug!(
                                    "[daemon-rx] direct backchannel available destination={} link={}",
                                    identity.address_hash,
                                    event.id
                                );
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    }
                }
            });
        }
    }

    let daemon_inbound = daemon;
    let inbound_transport = transport;
    tokio::spawn(async move {
        let local_delivery_destination =
            routing::local_delivery_destination_hash(control.delivery_destination.as_ref()).await;
        let mut rx = inbound_transport.received_data_events();
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if routing::should_skip_control_payload(&event, &control) {
                        continue;
                    }
                    let data = event.data.as_slice();
                    let raw_destination_hex = hex::encode(event.destination.as_slice());
                    let Some(resolved_destination) = routing::resolve_packet_destination(
                        inbound_transport.as_ref(),
                        &control,
                        &event.destination,
                        event.payload_mode,
                        local_delivery_destination,
                    )
                    .await
                    else {
                        log::debug!(
                            "[daemon-rx] skipping unresolved full-wire payload: dst={} len={} ctx={:?}",
                            raw_destination_hex,
                            data.len(),
                            event.context
                        );
                        continue;
                    };

                    if routing::should_skip_resolved_control_payload(
                        resolved_destination,
                        event.context,
                    ) {
                        continue;
                    }

                    delivery_events::log_resolved_packet(
                        &raw_destination_hex,
                        resolved_destination,
                        event.payload_mode,
                        event.ratchet_used,
                        data,
                    );

                    match resolved_destination {
                        InboundLxmfDestination::Propagation => {
                            if let Err(error) = propagation::ingest_propagation_envelope_with_transport(
                                daemon_inbound.as_ref(),
                                data,
                                control.delivery_destination.as_ref(),
                                Some(inbound_transport.as_ref()),
                            )
                            .await
                            {
                                log::debug!(
                                    "[daemon-rx] dropping inbound propagation payload: dst={} error={}",
                                    raw_destination_hex, error
                                );
                            }
                            continue;
                        }
                        InboundLxmfDestination::Delivery(destination) => {
                            delivery_events::accept_delivery_packet(
                                daemon_inbound.as_ref(),
                                inbound_transport.as_ref(),
                                &raw_destination_hex,
                                destination,
                                data,
                                event.payload_mode,
                            )
                            .await;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    log::debug!(
                        "[daemon-rx] received-data channel lagged; skipped {} events",
                        skipped
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

#[cfg(test)]
mod backchannel_link_classification_tests {
    use super::is_delivery_backchannel_link;
    use rand_core::OsRng;
    use rns_transport::destination::{DestinationDesc, DestinationName};
    use rns_transport::identity::PrivateIdentity;
    use rns_transport::transport::{Transport, TransportConfig};

    // Regression test for the Codex-flagged backchannel-caching bug in
    // PR #477: `PeerIdentified` fires for any link (delivery, propagation,
    // AND propagation.control), but only a delivery link's identify
    // should ever get cached as a peer's direct-delivery backchannel — a
    // propagation/control link cached under the same key would later get
    // reused for a direct delivery send, and the receiver would resolve
    // the payload as propagation/control instead of delivery. This drives
    // two real outbound links (one of each aspect) through an in-process
    // `Transport` — no identify handshake needed, since the gate only
    // inspects the link's own destination, not identify state.
    #[tokio::test]
    async fn only_a_delivery_aspect_link_is_treated_as_a_delivery_backchannel() {
        let local_identity = PrivateIdentity::new_from_rand(OsRng);
        let transport = Transport::new(TransportConfig::new(
            "backchannel-classification-test",
            &local_identity,
            true,
        ));

        let delivery_peer = PrivateIdentity::new_from_rand(OsRng);
        let delivery_destination = DestinationDesc {
            identity: *delivery_peer.as_identity(),
            address_hash: *delivery_peer.address_hash(),
            name: DestinationName::new("lxmf", "delivery"),
        };
        let delivery_link = transport.link(delivery_destination).await;
        let delivery_link_id = *delivery_link.lock().await.id();

        let control_peer = PrivateIdentity::new_from_rand(OsRng);
        let control_destination = DestinationDesc {
            identity: *control_peer.as_identity(),
            address_hash: *control_peer.address_hash(),
            name: DestinationName::new("lxmf", "propagation.control"),
        };
        let control_link = transport.link(control_destination).await;
        let control_link_id = *control_link.lock().await.id();

        assert!(is_delivery_backchannel_link(&transport, &delivery_link_id).await);
        assert!(!is_delivery_backchannel_link(&transport, &control_link_id).await);
    }

    #[tokio::test]
    async fn an_unresolvable_link_id_is_not_treated_as_a_delivery_backchannel() {
        let local_identity = PrivateIdentity::new_from_rand(OsRng);
        let transport = Transport::new(TransportConfig::new(
            "backchannel-classification-unresolvable-test",
            &local_identity,
            true,
        ));
        let bogus_link_id = rns_transport::hash::AddressHash::new([7u8; 16]);

        assert!(!is_delivery_backchannel_link(&transport, &bogus_link_id).await);
    }
}
