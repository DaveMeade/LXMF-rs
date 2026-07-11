impl ResourceManager {
    pub fn new() -> Self {
        Self::new_with_config(
            Duration::from_secs(DEFAULT_RESOURCE_RETRY_INTERVAL_SECS),
            DEFAULT_RESOURCE_MAX_RETRIES,
        )
    }

    pub fn new_with_config(retry_interval: Duration, retry_limit: u8) -> Self {
        Self {
            pending_outgoing: HashMap::new(),
            outgoing: HashMap::new(),
            outgoing_segment_chains: HashMap::new(),
            incoming: HashMap::new(),
            incoming_segments: HashMap::new(),
            events: Vec::new(),
            retry_interval,
            retry_limit,
            link_stats: HashMap::new(),
        }
    }

    pub fn start_send(
        &mut self,
        link: &Link,
        data: Vec<u8>,
        metadata: Option<Vec<u8>>,
    ) -> Result<(Hash, Packet), RnsError> {
        self.start_segmented_send(
            link,
            data,
            metadata,
            None,
            false,
            DEFAULT_RESOURCE_INTERFACE_MTU,
        )
    }

    pub fn start_send_with_mtu(
        &mut self,
        link: &Link,
        data: Vec<u8>,
        metadata: Option<Vec<u8>>,
        interface_mtu: usize,
    ) -> Result<(Hash, Packet), RnsError> {
        self.start_segmented_send(link, data, metadata, None, false, interface_mtu)
    }

    pub fn start_send_with_options(
        &mut self,
        link: &Link,
        data: Vec<u8>,
        metadata: Option<Vec<u8>>,
        request_id: Option<Vec<u8>>,
        is_response: bool,
    ) -> Result<(Hash, Packet), RnsError> {
        self.start_segmented_send(
            link,
            data,
            metadata,
            request_id,
            is_response,
            DEFAULT_RESOURCE_INTERFACE_MTU,
        )
    }

    pub fn start_send_with_options_mtu(
        &mut self,
        link: &Link,
        data: Vec<u8>,
        metadata: Option<Vec<u8>>,
        request_id: Option<Vec<u8>>,
        is_response: bool,
        interface_mtu: usize,
    ) -> Result<(Hash, Packet), RnsError> {
        self.start_segmented_send(
            link,
            data,
            metadata,
            request_id,
            is_response,
            interface_mtu,
        )
    }

    fn start_segmented_send(
        &mut self,
        link: &Link,
        data: Vec<u8>,
        metadata: Option<Vec<u8>>,
        request_id: Option<Vec<u8>>,
        is_response: bool,
        interface_mtu: usize,
    ) -> Result<(Hash, Packet), RnsError> {
        let metadata_size = metadata
            .as_ref()
            .map(|value| value.len().saturating_add(3))
            .unwrap_or(0);
        let total_size = metadata_size.checked_add(data.len()).ok_or(RnsError::InvalidArgument)?;
        if total_size <= MAX_EFFICIENT_SIZE {
            let sender = ResourceSender::new_with_options_mtu(
                link,
                data,
                metadata,
                request_id,
                is_response,
                interface_mtu,
            )?;
            return self.track_sender(sender);
        }
        if metadata_size >= MAX_EFFICIENT_SIZE || total_size > AUTO_COMPRESS_MAX_SIZE {
            return Err(RnsError::InvalidArgument);
        }

        let total_segments = total_size.div_ceil(MAX_EFFICIENT_SIZE) as u32;
        let first_data_len = (MAX_EFFICIENT_SIZE - metadata_size).min(data.len());
        let first = ResourceSender::new_segment_with_options_mtu(
            link,
            data[..first_data_len].to_vec(),
            metadata,
            request_id.clone(),
            is_response,
            interface_mtu,
            None,
            1,
            total_segments,
            Some(total_size as u64),
        )?;
        let original_hash = first.original_hash;
        let mut remaining = VecDeque::new();
        let mut offset = first_data_len;
        for segment_index in 2..=total_segments {
            let end = offset.saturating_add(MAX_EFFICIENT_SIZE).min(data.len());
            remaining.push_back(ResourceSender::new_segment_with_options_mtu(
                link,
                data[offset..end].to_vec(),
                None,
                request_id.clone(),
                is_response,
                interface_mtu,
                Some(original_hash),
                segment_index,
                total_segments,
                Some(total_size as u64),
            )?);
            offset = end;
        }
        self.outgoing_segment_chains.insert(original_hash, remaining);
        self.track_sender(first)
    }

    fn track_sender(&mut self, sender: ResourceSender) -> Result<(Hash, Packet), RnsError> {
        let resource_hash = sender.resource_hash;
        let packet = sender.advertisement_packet();
        self.pending_outgoing.insert(resource_hash, sender);
        Ok((resource_hash, packet))
    }
}
