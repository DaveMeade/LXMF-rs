use super::path::send_to_next_hop;
use super::resource_wire;
use super::wire_encryption::should_encrypt_packet;
use super::*;
use crate::packet::Header;
use ed25519_dalek::{Signature, SIGNATURE_LENGTH};

fn validate_destination_receipt_proof(
    identity: &Identity,
    packet: &Packet,
) -> Result<Hash, RnsError> {
    if packet.header.packet_type != PacketType::Proof
        || packet.context == PacketContext::LinkRequestProof
        || packet.data.len() < HASH_SIZE + SIGNATURE_LENGTH
    {
        return Err(RnsError::PacketError);
    }

    let mut hash = [0u8; HASH_SIZE];
    hash.copy_from_slice(&packet.data.as_slice()[..HASH_SIZE]);
    let signature =
        Signature::from_slice(&packet.data.as_slice()[HASH_SIZE..HASH_SIZE + SIGNATURE_LENGTH])
            .map_err(|_| RnsError::CryptoError)?;
    identity.verify(&hash, &signature)?;

    Ok(Hash::new(hash))
}

fn validate_destination_receipt_signature(
    identity: &Identity,
    receipt_hash: &Hash,
    signature_bytes: &[u8],
) -> Result<Hash, RnsError> {
    if signature_bytes.len() < SIGNATURE_LENGTH {
        return Err(RnsError::PacketError);
    }
    let signature = Signature::from_slice(&signature_bytes[..SIGNATURE_LENGTH])
        .map_err(|_| RnsError::CryptoError)?;
    identity.verify(receipt_hash.as_slice(), &signature)?;

    Ok(*receipt_hash)
}

pub(super) async fn validated_receipt_hash(
    packet: &Packet,
    handler: &TransportHandler,
) -> Result<Option<[u8; HASH_SIZE]>, RnsError> {
    if packet.header.packet_type != PacketType::Proof {
        return Ok(None);
    }

    if packet.header.destination_type == DestinationType::Link
        && matches!(packet.context, PacketContext::LinkProof | PacketContext::None)
    {
        let mut link = handler
            .in_links
            .get(&packet.destination)
            .cloned()
            .or_else(|| handler.out_links.get(&packet.destination).cloned());
        if link.is_none() {
            for candidate in handler.out_links.values() {
                if *candidate.lock().await.id() == packet.destination {
                    link = Some(candidate.clone());
                    break;
                }
            }
        }
        if let Some(link) = link {
            let link = link.lock().await;
            return match link.validate_packet_proof(packet) {
                Ok(hash) => Ok(Some(hash.to_bytes())),
                Err(_) => Err(RnsError::CryptoError),
            };
        }
        return Ok(None);
    }

    if packet.data.len() == SIGNATURE_LENGTH {
        let proof_context = {
            let packet_cache = handler.packet_cache.lock().await;
            packet_cache.proof_context_for_destination(&packet.destination)
        };
        if let Some((receipt_hash, proved_destination, _)) = proof_context {
            let mut destination_checked = false;
            if let Some(destination) =
                handler.single_out_destinations.get(&proved_destination).cloned()
            {
                destination_checked = true;
                let destination = destination.lock().await;
                if let Ok(hash) = validate_destination_receipt_signature(
                    &destination.identity,
                    &receipt_hash,
                    packet.data.as_slice(),
                ) {
                    return Ok(Some(hash.to_bytes()));
                }
            }
            if let Some(destination) =
                handler.single_in_destinations.get(&proved_destination).cloned()
            {
                destination_checked = true;
                let destination = destination.lock().await;
                if let Ok(hash) = validate_destination_receipt_signature(
                    destination.identity.as_identity(),
                    &receipt_hash,
                    packet.data.as_slice(),
                ) {
                    return Ok(Some(hash.to_bytes()));
                }
            }
            if destination_checked {
                return Err(RnsError::CryptoError);
            }
        }
    }

    // meshage fork — explicit proofs (`packet_hash(32B) ++ signature(64B)`)
    // are addressed the same way implicit ones are: to
    // `AddressHash::new_from_hash(&original_packet_hash)`
    // (`RNS/Packet.py::ProofDestination`, `RNS/Identity.py::prove` —
    // confirmed applies to both proof shapes, not just implicit), so a
    // direct lookup of `packet.destination` against this instance's own
    // registered destinations (the old behavior here) can never match a
    // real proof again — no real Reticulum peer, nor this crate's own
    // corrected proof-generation in `handle_data`, ever addresses a proof
    // to a real destination hash.
    //
    // Two cases, matching who is receiving this proof, both resolved the same
    // way: via the packet_cache reverse lookup the implicit branch above
    // already uses, cross-checking the embedded hash against the tracked one
    // first (mirroring Python's own `receipt.hash == proof_hash` gate), then
    // verifying *only* against the destination we actually tracked for that
    // hash.
    //
    // - We relayed the original packet: `handle_data`'s unconditional
    //   `packet_cache.note_source` call ran for it, whether or not we also
    //   host a local destination it happened to match.
    // - We are the *original sender*, receiving this proof directly from its
    //   prover: `PacketCache::update` (run for every outbound packet we send)
    //   records the same reverse mapping for our own Data sends, so this
    //   case has a cache entry too.
    //
    // Without a tracked entry, there is no cached record of us ever sending
    // or relaying a packet with this hash, so there's nothing to check the
    // proof against — do NOT fall back to trying every known destination's
    // identity against the embedded hash. That would accept a signature from
    // *any* peer we happen to know, over a hash they could have observed on
    // a packet addressed to someone else entirely, letting them forge a
    // delivery receipt for a message they never received.
    if packet.data.len() == HASH_SIZE + SIGNATURE_LENGTH {
        let proof_context = {
            let packet_cache = handler.packet_cache.lock().await;
            packet_cache.proof_context_for_destination(&packet.destination)
        };
        if let Some((receipt_hash, proved_destination, _)) = proof_context {
            let mut embedded_hash = [0u8; HASH_SIZE];
            embedded_hash.copy_from_slice(&packet.data.as_slice()[..HASH_SIZE]);
            if embedded_hash == receipt_hash.to_bytes() {
                let mut destination_checked = false;
                if let Some(destination) =
                    handler.single_out_destinations.get(&proved_destination).cloned()
                {
                    destination_checked = true;
                    let destination = destination.lock().await;
                    if let Ok(hash) =
                        validate_destination_receipt_proof(&destination.identity, packet)
                    {
                        return Ok(Some(hash.to_bytes()));
                    }
                }
                if let Some(destination) =
                    handler.single_in_destinations.get(&proved_destination).cloned()
                {
                    destination_checked = true;
                    let destination = destination.lock().await;
                    if let Ok(hash) = validate_destination_receipt_proof(
                        destination.identity.as_identity(),
                        packet,
                    ) {
                        return Ok(Some(hash.to_bytes()));
                    }
                }
                if destination_checked {
                    return Err(RnsError::CryptoError);
                }
            }
        }
    }

    Ok(None)
}

async fn should_forward_link_request_proof(
    packet: &Packet,
    handler: &TransportHandler,
    iface: AddressHash,
) -> bool {
    if packet.context != PacketContext::LinkRequestProof {
        return true;
    }

    let Some((original_destination, expected_iface)) =
        handler.link_table.proof_validation_context(&packet.destination)
    else {
        log::debug!(
            "[tp-diag] lrproof_forward_skip node={} reason=no_link_table_entry link={} iface={}",
            handler.config.name,
            packet.destination,
            iface
        );
        return false;
    };
    if expected_iface != iface {
        log::debug!(
            "[tp-diag] lrproof_forward_skip node={} reason=wrong_iface link={} expected={} got={}",
            handler.config.name,
            packet.destination,
            expected_iface,
            iface
        );
        return false;
    }

    let Some(destination) = handler.single_out_destinations.get(&original_destination).cloned()
    else {
        log::debug!(
            "[tp-diag] lrproof_forward_skip node={} reason=missing_destination_identity link={} dst={}",
            handler.config.name,
            packet.destination,
            original_destination
        );
        return false;
    };
    let destination = destination.lock().await;

    let valid = crate::destination::link::validate_link_request_proof_packet(
        &destination.desc,
        &packet.destination,
        packet,
    )
    .is_ok();
    log::debug!(
        "[tp-diag] lrproof_forward_validate node={} link={} dst={} iface={} valid={}",
        handler.config.name,
        packet.destination,
        original_destination,
        iface,
        valid
    );
    valid
}

pub(super) async fn handle_proof(
    packet: Packet,
    handler: Arc<Mutex<TransportHandler>>,
    iface: AddressHash,
) {
    if resource_wire::is_link_resource_proof(&packet) {
        resource_wire::handle_resource_proof(packet, handler, iface).await;
        return;
    }
    log::trace!("[tp] proof dst={} ctx={:02x}", packet.destination, packet.context as u8);
    let receipt_hash = {
        let handler = handler.lock().await;
        validated_receipt_hash(&packet, &handler).await
    };
    let receipt_hash = receipt_hash.unwrap_or_else(|err| {
        log::warn!("[tp] proof crypto validation failed dst={}: {:?}", packet.destination, err);
        None
    });
    if let Some(receipt_hash) = receipt_hash {
        let receipt = DeliveryReceipt::new(receipt_hash);
        let receipt_handler = {
            let handler = handler.lock().await;
            log::trace!("tp({}): handle proof for {}", handler.config.name, packet.destination);
            handler.receipt_handler.clone()
        };

        if let Some(receipt_handler) = receipt_handler {
            receipt_handler.on_receipt(&receipt);
        }
    }

    let mut handler = handler.lock().await;

    if packet.header.destination_type != DestinationType::Link {
        let source_iface = {
            let packet_cache = handler.packet_cache.lock().await;
            if packet.data.len() == SIGNATURE_LENGTH {
                packet_cache
                    .source_iface_for_proof_destination(&packet.destination)
                    .map(|(_, source_iface)| source_iface)
            } else if packet.data.len() >= HASH_SIZE {
                let mut proof_hash = [0u8; HASH_SIZE];
                proof_hash.copy_from_slice(&packet.data.as_slice()[..HASH_SIZE]);
                packet_cache.source_iface_for_hash(&Hash::new(proof_hash))
            } else {
                None
            }
        };
        if let Some(source_iface) = source_iface {
            if source_iface != iface {
                log::debug!(
                    "[tp-diag] destination_proof_reverse_forward node={} proof_dst={} source_iface={} ingress_iface={}",
                    handler.config.name,
                    packet.destination,
                    source_iface,
                    iface
                );
                handler
                    .send(TxMessage { tx_type: TxMessageType::Direct(source_iface), packet })
                    .await;
                return;
            }
        }
    }

    let mut rtt_messages = Vec::new();
    for link in handler.out_links.values() {
        let mut link = link.lock().await;
        if let LinkHandleResult::Activated = link.handle_packet(&packet, iface) {
            rtt_messages.push(TxMessage {
                tx_type: TxMessageType::Direct(iface),
                packet: link.create_rtt(),
            });
        }
    }
    for message in rtt_messages {
        let dispatch = handler.send(message).await;
        if dispatch.sent_ifaces == 0 {
            log::warn!(
                "tp({}): failed to dispatch link RTT packet matched={} failed={}",
                handler.config.name,
                dispatch.matched_ifaces,
                dispatch.failed_ifaces
            );
        }
    }

    let maybe_packet = if should_forward_link_request_proof(&packet, &handler, iface).await {
        handler.link_table.handle_proof(&packet)
    } else {
        None
    };

    if let Some((packet, iface)) = maybe_packet {
        log::debug!(
            "[tp-diag] lrproof_forward node={} link={} iface={}",
            handler.config.name,
            packet.destination,
            iface
        );
        handler.send(TxMessage { tx_type: TxMessageType::Direct(iface), packet }).await;
    } else if packet.context == PacketContext::LinkRequestProof {
        log::debug!(
            "[tp-diag] lrproof_not_forwarded node={} link={} ingress_iface={}",
            handler.config.name,
            packet.destination,
            iface
        );
    }
}

pub(super) async fn handle_keepalive_response<'a>(
    packet: &Packet,
    handler: &mut MutexGuard<'a, TransportHandler>,
) -> bool {
    if packet.context == PacketContext::KeepAlive
        && packet.data.as_slice()[0] == KEEP_ALIVE_RESPONSE
    {
        let lookup = handler.link_table.handle_keepalive(packet);

        if let Some((propagated, iface)) = lookup {
            handler
                .send(TxMessage { tx_type: TxMessageType::Direct(iface), packet: propagated })
                .await;
        }

        return true;
    }

    false
}

pub(super) async fn handle_data<'a>(
    packet: &Packet,
    iface: AddressHash,
    mut handler: MutexGuard<'a, TransportHandler>,
) {
    handler.packet_cache.lock().await.note_source(packet, iface);
    let mut data_handled = false;

    if packet.header.destination_type == DestinationType::Link {
        if resource_wire::is_link_resource_packet(packet)
            && resource_wire::handle_link_resource_packet(packet, iface, &mut handler).await
        {
            return;
        }

        log::trace!(
            "[tp] link_data dst={} ctx={:02x} len={}",
            packet.destination,
            packet.context as u8,
            packet.data.len()
        );
        let mut link_packets = Vec::new();
        if let Some(link) = handler.in_links.get(&packet.destination).cloned() {
            let mut link = link.lock().await;
            let result = link.handle_packet(packet, iface);
            if let LinkHandleResult::KeepAlive = result {
                link_packets.push(link.keep_alive_packet(KEEP_ALIVE_RESPONSE));
            } else if let LinkHandleResult::Proof(proof_packet) = result {
                link_packets.push(proof_packet);
            }
        }

        let mut proof_packets = Vec::new();
        for link in handler.out_links.values() {
            let mut link = link.lock().await;
            let result = link.handle_packet(packet, iface);
            if let LinkHandleResult::Proof(proof_packet) = result {
                proof_packets.push(proof_packet);
            }
            data_handled = true;
        }

        for packet in link_packets {
            handler.send(TxMessage { tx_type: TxMessageType::Direct(iface), packet }).await;
        }
        for packet in proof_packets {
            handler.send(TxMessage { tx_type: TxMessageType::Direct(iface), packet }).await;
        }

        if handle_keepalive_response(packet, &mut handler).await {
            return;
        }

        if let Some((packet, iface)) = handler.link_table.handle_reverse_link_packet(packet, iface)
        {
            log::debug!(
                "[resource-diag] wire_resource_reverse_forward node={} link={} iface={}",
                handler.config.name,
                packet.destination,
                iface
            );
            handler.send(TxMessage { tx_type: TxMessageType::Direct(iface), packet }).await;
            return;
        }

        let lookup = handler.link_table.original_destination(&packet.destination);
        if lookup.is_some() {
            let sent = send_to_next_hop(packet, &handler, lookup).await;

            log::trace!(
                "tp({}): {} packet to remote link {}",
                handler.config.name,
                if sent { "forwarded" } else { "could not forward" },
                packet.destination
            );
        }
    }

    if packet.header.destination_type == DestinationType::Single {
        let has_local_destination =
            handler.single_in_destinations.contains_key(&packet.destination);
        log::info!(
            "[tp-diag] inbound_single_data node={} dst={} iface={} local_destination={} ctx={:02x} len={}",
            handler.config.name,
            packet.destination,
            iface,
            has_local_destination,
            packet.context as u8,
            packet.data.len(),
        );
        if let Some(destination) = handler.single_in_destinations.get(&packet.destination).cloned()
        {
            data_handled = true;
            let mut ratchet_used = false;
            let payload = if should_encrypt_packet(packet) {
                let mut destination = destination.lock().await;
                match destination.decrypt_with_ratchets(packet.data.as_slice()) {
                    Ok((plaintext, used)) => {
                        ratchet_used = used;
                        plaintext
                    }
                    Err(err) => {
                        log::warn!(
                            "tp({}): decrypt failed for {}: {:?}",
                            handler.config.name,
                            packet.destination,
                            err
                        );
                        return;
                    }
                }
            } else {
                packet.data.as_slice().to_vec()
            };
            let mut buffer = PacketDataBuffer::new();
            if buffer.write(&payload).is_err() {
                log::warn!(
                    "tp({}): decrypted payload too large for {}",
                    handler.config.name,
                    packet.destination
                );
                return;
            }
            handler
                .received_data_tx
                .send(ReceivedData {
                    destination: packet.destination,
                    data: buffer,
                    payload_mode: ReceivedPayloadMode::DestinationStripped,
                    ratchet_used,
                    context: Some(packet.context),
                    request_id: if matches!(
                        packet.context,
                        PacketContext::Request | PacketContext::Response
                    ) {
                        let hash = packet.hash().to_bytes();
                        let mut request_id = [0u8; 16];
                        request_id.copy_from_slice(&hash[..16]);
                        Some(request_id)
                    } else {
                        None
                    },
                    hops: Some(packet.header.hops),
                    interface: packet.transport.map(|value| value.as_slice().to_vec()),
                })
                .ok();

            // Generates the automatic delivery proof this branch was
            // missing: it decrypts and forwards a plain `Single`/`Data`
            // packet (e.g. a direct LXMF message) above, but never told
            // the sender it arrived. The receive-side validation this
            // proof round-trips through
            // (`validated_receipt_hash`/`validate_destination_receipt_proof`,
            // same file) already handles it correctly, so this is the one
            // missing half. Wire format matches what that validation
            // function already expects: `packet_hash(32B) ++
            // Ed25519_signature(64B)`, signed by this destination's own
            // private identity over the hash of the packet just received.
            // Replied on the same interface the original packet arrived
            // on — this is correct, not a shortcut: Reticulum's own proof
            // relay is hop-by-hop reversal (each node along a path replies
            // on the interface it received on), not a fresh path-table
            // lookup by the proof's own destination (which here is *this*
            // destination's own hash, not a remote one any path table
            // would have an entry for). See #479.
            //
            // Whether a proof actually gets sent is gated by this
            // destination's own `proof_strategy` (mirrors Python
            // Reticulum's `PROVE_NONE`/`PROVE_APP`/`PROVE_ALL` —
            // see `ProofStrategy`'s doc comment) — a receiver-owned policy
            // decision, not something the crate imposes unconditionally.
            // `context: None` is still checked first regardless of
            // strategy: `Request`/`Response` already have the response
            // itself as their own delivery acknowledgement;
            // `Resource`/`KeepAlive`/`CacheRequest` (the same set
            // `should_encrypt_packet` above already excludes, for the same
            // underlying reason — they're each a sub-protocol with its own
            // semantics, not a plain opportunistic app message) have their
            // own dedicated completion/ack mechanisms elsewhere in this
            // crate. Proving those too would be redundant at best and
            // could plausibly confuse a peer expecting exactly one
            // specific ack shape per context.
            if packet.context == PacketContext::None {
                let packet_hash = packet.hash();
                let signature = {
                    let destination_guard = destination.lock().await;
                    let should_prove = match destination_guard.proof_strategy {
                        ProofStrategy::None => false,
                        ProofStrategy::All => true,
                        ProofStrategy::App => destination_guard
                            .proof_requested_callback
                            .as_ref()
                            .is_some_and(|cb| cb.proof_requested(packet)),
                    };
                    should_prove.then(|| destination_guard.identity.sign(&packet_hash.to_bytes()))
                };
                if let Some(signature) = signature {
                    let mut proof_data = Vec::with_capacity(HASH_SIZE + SIGNATURE_LENGTH);
                    proof_data.extend_from_slice(&packet_hash.to_bytes());
                    proof_data.extend_from_slice(&signature.to_bytes());
                    let proof_packet = Packet {
                        header: Header { packet_type: PacketType::Proof, ..Default::default() },
                        ifac: None,
                        // Real Reticulum always addresses a proof to
                        // `Packet.generate_proof_destination()` — the
                        // truncated hash of the *proved* packet, not the
                        // proving destination's own real address hash —
                        // for both explicit and implicit proof shapes
                        // alike (`RNS/Identity.py::prove`,
                        // `RNS/Packet.py::ProofDestination`). This crate's
                        // own reverse-routing table for proofs
                        // (`PacketCache::note_source`/
                        // `by_proof_destination`) is keyed the same way.
                        // Addressing this to our own real destination hash
                        // instead "worked" for direct connections (Python's
                        // local receipt validation in `Transport.py`
                        // matches by scanning `Transport.receipts`, not by
                        // this field), but silently broke reachability the
                        // moment the proof needed to traverse any
                        // intermediate Transport/relay hop back to the
                        // original sender — that hop's own reverse-routing
                        // table would never have an entry under our real
                        // address hash. Confirmed against
                        // `RNS/Identity.py::prove` and
                        // `RNS/Transport.py`'s inbound-proof handling
                        // directly.
                        destination: AddressHash::new_from_hash(&packet_hash),
                        transport: None,
                        context: PacketContext::None,
                        data: PacketDataBuffer::new_from_slice(&proof_data),
                    };
                    handler
                        .send(TxMessage {
                            tx_type: TxMessageType::Direct(iface),
                            packet: proof_packet,
                        })
                        .await;
                }
            }
        } else {
            data_handled = send_to_next_hop(packet, &handler, None).await;
        }
    }

    if data_handled {
        log::trace!(
            "tp({}): handle data request for {} dst={:2x} ctx={:2x}",
            handler.config.name,
            packet.destination,
            packet.header.destination_type as u8,
            packet.context as u8,
        );
    }
}
