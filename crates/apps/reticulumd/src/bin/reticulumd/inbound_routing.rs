use crate::bootstrap::PropagationControlContext;
use rns_transport::destination::{DestinationDesc, DestinationName, SingleInputDestination};
use rns_transport::hash::AddressHash;
use rns_transport::packet::PacketContext;
use rns_transport::transport::{ReceivedData, ReceivedPayloadMode, Transport};
use std::sync::Arc;

use super::propagation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InboundLxmfDestination {
    Delivery([u8; 16]),
    Propagation,
}

pub(super) async fn resolve_resource_destination(
    transport: &Transport,
    link_id: &AddressHash,
    local_delivery_destination: Option<[u8; 16]>,
) -> Option<InboundLxmfDestination> {
    if let Some(link) = transport.find_in_link(link_id).await {
        let guard = link.lock().await;
        if let Some(destination) = lxmf_destination_from_desc(guard.destination()) {
            return Some(destination);
        }
    }
    if let Some(link) = transport.find_out_link(link_id).await {
        let guard = link.lock().await;
        return destination_for_outbound_link(guard.destination(), local_delivery_destination);
    }
    None
}

pub(super) async fn resolve_packet_destination(
    transport: &Transport,
    control: &PropagationControlContext,
    destination: &AddressHash,
    payload_mode: ReceivedPayloadMode,
    local_delivery_destination: Option<[u8; 16]>,
) -> Option<InboundLxmfDestination> {
    match payload_mode {
        ReceivedPayloadMode::DestinationStripped => {
            if let Some(resolved) = resolve_link_packet_destination(transport, destination).await {
                return Some(resolved);
            }
            if let Some(link) = transport.find_out_link(destination).await {
                let guard = link.lock().await;
                if let Some(resolved) =
                    destination_for_outbound_link(guard.destination(), local_delivery_destination)
                {
                    return Some(resolved);
                }
            }
            if propagation::is_lxmf_propagation_destination(destination, control) {
                Some(InboundLxmfDestination::Propagation)
            } else {
                Some(InboundLxmfDestination::Delivery(destination_hash(destination)))
            }
        }
        ReceivedPayloadMode::FullWire => {
            if let Some(resolved) = resolve_link_packet_destination(transport, destination).await {
                return Some(resolved);
            }
            if let Some(link) = transport.find_out_link(destination).await {
                let guard = link.lock().await;
                if let Some(resolved) =
                    destination_for_outbound_link(guard.destination(), local_delivery_destination)
                {
                    return Some(resolved);
                }
            }
            local_delivery_destination
                .filter(|local| local.as_slice() == destination.as_slice())
                .map(InboundLxmfDestination::Delivery)
        }
    }
}

fn destination_for_outbound_link(
    remote_destination: &DestinationDesc,
    local_delivery_destination: Option<[u8; 16]>,
) -> Option<InboundLxmfDestination> {
    if is_lxmf_delivery_destination(remote_destination) {
        return local_delivery_destination.map(InboundLxmfDestination::Delivery);
    }
    lxmf_destination_from_desc(remote_destination)
}

async fn resolve_link_packet_destination(
    transport: &Transport,
    link_id: &AddressHash,
) -> Option<InboundLxmfDestination> {
    let link = transport.find_in_link(link_id).await?;
    let guard = link.lock().await;
    lxmf_destination_from_desc(guard.destination())
}

pub(super) async fn local_delivery_destination_hash(
    destination: Option<&Arc<tokio::sync::Mutex<SingleInputDestination>>>,
) -> Option<[u8; 16]> {
    let destination = destination?;
    let guard = destination.lock().await;
    Some(destination_hash(&guard.desc.address_hash))
}

pub(super) fn should_skip_control_payload(
    event: &ReceivedData,
    control: &PropagationControlContext,
) -> bool {
    let Some(control_hash) = control.control_destination_hash_hex.as_deref() else {
        return false;
    };
    if hex::encode(event.destination.as_slice()) != control_hash {
        return false;
    }
    matches!(
        event.context,
        Some(PacketContext::Request | PacketContext::Response | PacketContext::LinkIdentify)
    )
}

pub(super) fn should_skip_resolved_control_payload(
    destination: InboundLxmfDestination,
    context: Option<PacketContext>,
) -> bool {
    matches!(destination, InboundLxmfDestination::Propagation)
        && matches!(
            context,
            Some(PacketContext::Request | PacketContext::Response | PacketContext::LinkIdentify)
        )
}

fn lxmf_destination_from_desc(destination: &DestinationDesc) -> Option<InboundLxmfDestination> {
    if is_lxmf_delivery_destination(destination) {
        return Some(InboundLxmfDestination::Delivery(destination_hash(&destination.address_hash)));
    }
    if is_lxmf_propagation_link_destination(destination) {
        return Some(InboundLxmfDestination::Propagation);
    }
    None
}

fn destination_hash(destination: &AddressHash) -> [u8; 16] {
    let mut hash = [0u8; 16];
    hash.copy_from_slice(destination.as_slice());
    hash
}

// `pub(super)` — the direct-backchannel `PeerIdentified` consumer in
// `module_core.rs` (included into this same `inbound_worker` module via
// `include!`) needs this to gate which links get cached as a peer's
// delivery backchannel (see its call site's doc comment for why).
pub(super) fn is_lxmf_delivery_destination(destination: &DestinationDesc) -> bool {
    destination.name.hash == DestinationName::new("lxmf", "delivery").hash
}

fn is_lxmf_propagation_link_destination(destination: &DestinationDesc) -> bool {
    destination.name.hash == DestinationName::new("lxmf", "propagation").hash
}

#[cfg(test)]
mod tests {
    use super::{
        destination_for_outbound_link, is_lxmf_delivery_destination,
        is_lxmf_propagation_link_destination, should_skip_resolved_control_payload,
        InboundLxmfDestination,
    };
    use rand_core::OsRng;
    use rns_transport::destination::{DestinationDesc, DestinationName};
    use rns_transport::identity::PrivateIdentity;
    use rns_transport::packet::PacketContext;

    #[test]
    fn lxmf_delivery_destination_is_accepted_for_resource_decode() {
        let signer = PrivateIdentity::new_from_rand(OsRng);
        let destination = DestinationDesc {
            identity: *signer.as_identity(),
            address_hash: *signer.address_hash(),
            name: DestinationName::new("lxmf", "delivery"),
        };

        assert!(is_lxmf_delivery_destination(&destination));
    }

    #[test]
    fn non_delivery_destination_is_rejected_for_resource_decode() {
        let signer = PrivateIdentity::new_from_rand(OsRng);
        let destination = DestinationDesc {
            identity: *signer.as_identity(),
            address_hash: *signer.address_hash(),
            name: DestinationName::new("lxmf", "propagation.control"),
        };

        assert!(!is_lxmf_delivery_destination(&destination));
    }

    #[test]
    fn propagation_destination_is_detected_for_resource_decode() {
        let signer = PrivateIdentity::new_from_rand(OsRng);
        let destination = DestinationDesc {
            identity: *signer.as_identity(),
            address_hash: *signer.address_hash(),
            name: DestinationName::new("lxmf", "propagation"),
        };

        assert!(is_lxmf_propagation_link_destination(&destination));
    }

    #[test]
    fn propagation_link_control_context_is_skipped_from_payload_ingest() {
        assert!(should_skip_resolved_control_payload(
            InboundLxmfDestination::Propagation,
            Some(PacketContext::LinkIdentify)
        ));
        assert!(should_skip_resolved_control_payload(
            InboundLxmfDestination::Propagation,
            Some(PacketContext::Request)
        ));
    }

    #[test]
    fn outbound_delivery_link_backchannel_resolves_to_local_delivery_destination() {
        let remote = PrivateIdentity::new_from_rand(OsRng);
        let remote_destination = DestinationDesc {
            identity: *remote.as_identity(),
            address_hash: *remote.address_hash(),
            name: DestinationName::new("lxmf", "delivery"),
        };
        let local_destination = [7_u8; 16];

        assert_eq!(
            destination_for_outbound_link(&remote_destination, Some(local_destination)),
            Some(InboundLxmfDestination::Delivery(local_destination))
        );
    }
}
