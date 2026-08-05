impl ResourceManager {
    /// Concludes a segment and advertises the next one.
    ///
    /// Split out of `manager.rs` to keep it inside the module-size budget once
    /// the next segment started being *built* here rather than popped off a
    /// pre-built queue.
    fn handle_proof_into(&mut self, packet: &Packet, link: &Link, responses: &mut Vec<Packet>) {
        let Ok(proof) = ResourceProof::decode(packet.data.as_slice()) else {
            return;
        };
        if !self
            .outgoing
            .get_mut(&proof.resource_hash)
            .is_some_and(|sender| sender.handle_proof(&proof))
        {
            return;
        }
        let Some(sender) = self.outgoing.remove(&proof.resource_hash) else {
            return;
        };

        if sender.segment_index < sender.total_segments {
            // Built here rather than up front in `start_send`: the transfer is
            // idle at this exact point, waiting to advertise the next segment,
            // so this is where the work belongs. See `PendingSegments`.
            let next = self
                .outgoing_segment_chains
                .get_mut(&sender.original_hash)
                .and_then(|pending| pending.build_next(link));
            match next {
                Some(Ok(mut next)) => {
                    let advertisement = next.advertisement_packet();
                    next.mark_advertised(self.retry_limit);
                    self.outgoing.insert(next.resource_hash, next);
                    responses.push(advertisement);
                    return;
                }
                // Nothing is waiting on a return value this deep in the packet
                // path, so a build failure has to be reported as an event or
                // the transfer simply stops with both ends believing it is
                // still running — the same trap #557 closed for inbound
                // assemblies.
                Some(Err(error)) => {
                    log::warn!(
                        "split resource segment build failed hash={} segment={}/{} error={error:?}",
                        sender.original_hash,
                        sender.segment_index.saturating_add(1),
                        sender.total_segments
                    );
                    self.outgoing_segment_chains.remove(&sender.original_hash);
                    self.events.push(ResourceEvent {
                        hash: sender.original_hash,
                        link_id: sender.link_id,
                        kind: ResourceEventKind::OutboundFailed,
                    });
                    return;
                }
                None => {}
            }
        }

        self.outgoing_segment_chains.remove(&sender.original_hash);
        self.events.push(ResourceEvent {
            hash: sender.original_hash,
            link_id: packet.destination,
            kind: ResourceEventKind::OutboundComplete,
        });
    }
}
