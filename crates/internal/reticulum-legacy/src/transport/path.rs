use super::*;

const PATH_RESPONSE_GRACE: Duration = Duration::from_secs(1);

pub(super) async fn send_to_next_hop<'a>(
    packet: &Packet,
    handler: &MutexGuard<'a, TransportHandler>,
    lookup: Option<AddressHash>,
) -> bool {
    let (packet, maybe_iface) = handler.path_table.handle_inbound_packet(packet, lookup);

    if let Some(iface) = maybe_iface {
        handler.send(TxMessage { tx_type: TxMessageType::Direct(iface), packet }).await;
    }

    maybe_iface.is_some()
}

pub(super) async fn handle_path_request<'a>(
    packet: &Packet,
    handler: &mut MutexGuard<'a, TransportHandler>,
    iface: AddressHash,
) {
    if let Some(request) = handler.path_requests.decode(packet.data.as_slice()) {
        if let Some(dest) = handler.single_in_destinations.get(&request.destination) {
            let response =
                dest.lock().await.path_response(OsRng, None).expect("valid path response");

            handler
                .send(TxMessage { tx_type: TxMessageType::Direct(iface), packet: response })
                .await;

            log::trace!("tp({}): send direct path response over {}", handler.config.name, iface);

            return;
        }

        if handler.config.retransmit {
            if let Some(entry) = handler.path_table.get(&request.destination) {
                if let Some(requestor_id) = request.requesting_transport {
                    if requestor_id == entry.received_from {
                        log::trace!(
                            "tp({}): dropping circular path request from {}",
                            handler.config.name,
                            request.destination
                        );
                        return;
                    }
                }

                let hops = entry.hops;
                let response_delay = if hops <= 1 { Duration::ZERO } else { PATH_RESPONSE_GRACE };
                if handler
                    .announce_table
                    .add_response(request.destination, iface, hops, response_delay)
                    && response_delay.is_zero()
                {
                    let transport_id = *handler.config.identity.address_hash();
                    let messages = handler.announce_table.take_ready_responses(&transport_id);
                    for message in messages {
                        handler.send(message).await;
                    }
                }

                log::trace!(
                    "tp({}): scheduled remote path response to {} ({} hops) over {}",
                    handler.config.name,
                    request.destination,
                    hops,
                    iface
                );

                return;
            }
        }

        if handler.config.retransmit {
            if let Some(packet) =
                handler.path_requests.generate_recursive(&request.destination, Some(iface), None)
            {
                handler
                    .send(TxMessage { tx_type: TxMessageType::Broadcast(Some(iface)), packet })
                    .await;
            }
        }
    }
}

pub(super) async fn handle_fixed_destinations<'a>(
    packet: &Packet,
    handler: &mut MutexGuard<'a, TransportHandler>,
    iface: AddressHash,
) -> bool {
    if packet.destination == handler.fixed_dest_path_requests {
        handle_path_request(packet, handler, iface).await;
        true
    } else {
        false
    }
}

pub(super) async fn handle_link_request_as_destination<'a>(
    destination: Arc<Mutex<SingleInputDestination>>,
    packet: &Packet,
    iface: AddressHash,
    mut handler: MutexGuard<'a, TransportHandler>,
) {
    let mut destination = destination.lock().await;
    match destination.handle_packet(packet) {
        DestinationHandleStatus::LinkProof => {
            let link_id = LinkId::from(packet);
            if !handler.in_links.contains_key(&link_id) {
                log::trace!("tp({}): send proof to {}", handler.config.name, packet.destination);

                let link = Link::new_from_request(
                    packet,
                    destination.sign_key().clone(),
                    destination.desc,
                    handler.link_in_event_tx.clone(),
                );

                if let Ok(mut link) = link {
                    // Link-request proofs must go back over the interface that delivered
                    // the request so multi-hop requestors can activate the link.
                    handler
                        .send(TxMessage {
                            tx_type: TxMessageType::Direct(iface),
                            packet: link.prove(),
                        })
                        .await;

                    log::debug!(
                        "tp({}): save input link {} for destination {}",
                        handler.config.name,
                        link.id(),
                        link.destination().address_hash
                    );

                    handler.in_links.insert(*link.id(), Arc::new(Mutex::new(link)));
                }
            }
        }
        DestinationHandleStatus::None => {}
    }
}

pub(super) async fn handle_link_request_as_intermediate<'a>(
    received_from: AddressHash,
    next_hop: AddressHash,
    next_hop_iface: AddressHash,
    packet: &Packet,
    mut handler: MutexGuard<'a, TransportHandler>,
) {
    handler.link_table.add(packet, packet.destination, received_from, next_hop, next_hop_iface);

    send_to_next_hop(packet, &handler, None).await;
}

pub(super) async fn handle_link_request<'a>(
    packet: &Packet,
    iface: AddressHash,
    handler: MutexGuard<'a, TransportHandler>,
) {
    if let Some(destination) = handler.single_in_destinations.get(&packet.destination).cloned() {
        log::trace!("tp({}): handle link request for {}", handler.config.name, packet.destination);

        handle_link_request_as_destination(destination, packet, iface, handler).await;
    } else if let Some(entry) = handler.path_table.next_hop_full(&packet.destination) {
        log::trace!(
            "tp({}): handle link request for remote destination {}",
            handler.config.name,
            packet.destination
        );

        let (next_hop, next_iface) = entry;
        handle_link_request_as_intermediate(iface, next_hop, next_iface, packet, handler).await;
    } else {
        log::trace!(
            "tp({}): dropping link request to unknown destination {}",
            handler.config.name,
            packet.destination
        );
    }
}
