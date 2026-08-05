#[derive(Debug)]
struct InboundSegmentAssembly {
    link_id: AddressHash,
    next_segment: u32,
    total_segments: u32,
    total_data_size: u64,
    data: Vec<u8>,
    metadata: Option<Vec<u8>>,
    request_id: Option<Vec<u8>>,
    is_request: bool,
    is_response: bool,
}

impl ResourceManager {
    /// Abandon a partially assembled split resource and tell the caller why.
    ///
    /// Every path that drops an assembly has to come through here. Returning
    /// quietly instead leaves whoever is awaiting the resource blocked until
    /// its own timeout expires, with nothing on either side saying the transfer
    /// is already dead (issue #369).
    fn fail_inbound_segments(&mut self, original_hash: Hash, reason: &str) {
        let Some(assembly) = self.incoming_segments.remove(&original_hash) else {
            return;
        };
        log::warn!("split resource assembly failed hash={original_hash} reason={reason}");
        self.events.push(ResourceEvent {
            hash: original_hash,
            link_id: assembly.link_id,
            kind: ResourceEventKind::InboundFailed(ResourceFailure {
                reason: reason.to_string(),
                progress: ResourceProgress {
                    received_bytes: assembly.data.len() as u64,
                    total_bytes: assembly.total_data_size,
                    received_parts: assembly.next_segment.saturating_sub(1) as usize,
                    total_parts: assembly.total_segments as usize,
                },
            }),
        });
    }

    pub fn confirm_outbound_dispatch(&mut self, resource_hash: Hash, sent: bool) {
        let Some(mut sender) = self.pending_outgoing.remove(&resource_hash) else {
            return;
        };

        if sent {
            sender.mark_advertised(self.retry_limit);
            self.outgoing.insert(resource_hash, sender);
        } else {
            self.outgoing_segment_chains.remove(&sender.original_hash);
            self.events.push(ResourceEvent {
                hash: resource_hash,
                link_id: sender.link_id,
                kind: ResourceEventKind::OutboundFailed,
            });
        }
    }

    fn finish_inbound_payload(
        &mut self,
        segment_hash: Hash,
        link_id: AddressHash,
        payload: ResourcePayload,
    ) {
        let segment = payload.segmentation;
        if segment.total_segments == 1 {
            self.events.push(ResourceEvent {
                hash: segment_hash,
                link_id,
                kind: ResourceEventKind::Complete(ResourceComplete {
                    data: payload.data,
                    metadata: payload.metadata,
                    request_id: payload.request_id,
                    is_request: payload.is_request,
                    is_response: payload.is_response,
                }),
            });
            return;
        }

        let assembly = self
            .incoming_segments
            .entry(segment.original_hash)
            .or_insert_with(|| InboundSegmentAssembly {
                link_id,
                next_segment: 1,
                total_segments: segment.total_segments,
                total_data_size: segment.total_data_size,
                data: Vec::new(),
                metadata: payload.metadata.clone(),
                request_id: payload.request_id.clone(),
                is_request: payload.is_request,
                is_response: payload.is_response,
            });
        // Deliberately a drop rather than a failure: issue #520 established that
        // an out-of-order segment must leave the assembly intact so the transfer
        // resumes when the expected segment arrives.
        if assembly.link_id != link_id
            || assembly.total_segments != segment.total_segments
            || assembly.total_data_size != segment.total_data_size
            || assembly.next_segment != segment.segment_index
        {
            log::warn!(
                "discarding inconsistent resource segment original_hash={} segment={}/{}",
                segment.original_hash,
                segment.segment_index,
                segment.total_segments
            );
            return;
        }
        let metadata_size = assembly
            .metadata
            .as_ref()
            .map(|metadata| metadata.len().saturating_add(3))
            .unwrap_or(0);
        if assembly
            .data
            .len()
            .saturating_add(payload.data.len())
            .saturating_add(metadata_size)
            > assembly.total_data_size as usize
        {
            self.fail_inbound_segments(segment.original_hash, "oversized_segment");
            return;
        }
        assembly.data.extend_from_slice(&payload.data);
        assembly.next_segment = assembly.next_segment.saturating_add(1);
        self.events.push(ResourceEvent {
            hash: segment.original_hash,
            link_id,
            kind: ResourceEventKind::SegmentComplete(segment),
        });

        if segment.segment_index == segment.total_segments {
            let Some(assembly) = self.incoming_segments.get(&segment.original_hash) else {
                return;
            };
            let assembled_size = assembly.data.len().saturating_add(
                assembly.metadata.as_ref().map(|metadata| metadata.len() + 3).unwrap_or(0),
            ) as u64;
            if assembled_size != assembly.total_data_size {
                self.fail_inbound_segments(segment.original_hash, "assembled_size_mismatch");
                return;
            }
            if let Some(assembly) = self.incoming_segments.remove(&segment.original_hash) {
                self.events.push(ResourceEvent {
                    hash: segment.original_hash,
                    link_id,
                    kind: ResourceEventKind::Complete(ResourceComplete {
                        data: assembly.data,
                        metadata: assembly.metadata,
                        request_id: assembly.request_id,
                        is_request: assembly.is_request,
                        is_response: assembly.is_response,
                    }),
                });
            }
        }
    }
}
