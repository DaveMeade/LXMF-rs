/// A sender flags *every* segment of a split resource as metadata-bearing — it
/// threads the metadata size into each one so the receiver can account for
/// those bytes in `total_data_size` — but only segment 1 carries the actual
/// 3-byte-length-prefixed block. Stripping a block from later segments reads
/// three bytes of file data as a length and deletes that many bytes.
///
/// Nothing else catches it: the resource hash is computed over the payload
/// *before* the metadata split, so every fragment and every segment verifies
/// while the assembled file is quietly missing bytes from its middle.
#[test]
fn resource_receiver_strips_split_metadata_from_the_first_segment_only() {
    let signer = PrivateIdentity::new_from_rand(OsRng);
    let identity = *signer.as_identity();
    let destination = DestinationDesc {
        identity,
        address_hash: identity.address_hash,
        name: DestinationName::new("lxmf", "resource"),
    };
    let (tx, _) = tokio::sync::broadcast::channel(1);
    let mut link = Link::new(destination, tx);
    link.request();
    let mut manager = ResourceManager::new_with_config(Duration::from_secs(1), 2);

    // Segment 1 carries `[len:3][metadata]` ahead of its share of the file.
    let metadata = b"meta";
    let first_file_data = b"first-segment";
    let mut first_wire = vec![0x00, 0x00, metadata.len() as u8];
    first_wire.extend_from_slice(metadata);
    first_wire.extend_from_slice(first_file_data);

    // Segment 2 is file data end to end. It opens with bytes that *look* like a
    // 4-byte length header, which is the whole hazard: an unfixed receiver reads
    // them as one and swallows the following seven bytes.
    let second_wire: &[u8] = &[0x00, 0x00, 0x04, b's', b'e', b'c', b'o', b'n', b'd'];

    // `total_data_size` is the file plus the metadata block, as the reference
    // computes it: `total_size = data_size + metadata_size`.
    let total_data_size = (3 + metadata.len() + first_file_data.len() + second_wire.len()) as u64;

    let (mut first_adv, first_part) =
        split_test_segment(&first_wire, None, 1, 2, total_data_size);
    first_adv.flags |= FLAG_METADATA;
    let original_hash = first_adv.hash;
    let (mut second_adv, second_part) =
        split_test_segment(second_wire, Some(original_hash), 2, 2, total_data_size);
    second_adv.flags |= FLAG_METADATA;

    for (adv, part) in [(first_adv, first_part), (second_adv, second_part)] {
        let adv_packet = resource_packet(
            PacketContext::ResourceAdvrtisement,
            &adv.pack().expect("pack advertisement"),
            *link.id(),
        );
        assert_eq!(manager.handle_packet(&adv_packet, &mut link).len(), 1);
        let part_packet = resource_packet(PacketContext::Resource, &part, *link.id());
        assert_eq!(manager.handle_packet(&part_packet, &mut link).len(), 1);
    }

    let events = manager.drain_events();
    let complete = events
        .into_iter()
        .find_map(|event| match event.kind {
            ResourceEventKind::Complete(complete) if event.hash == original_hash => Some(complete),
            _ => None,
        })
        .expect("assembled split resource");
    assert_eq!(complete.metadata.as_deref(), Some(metadata.as_slice()));
    assert_eq!(complete.data, [first_file_data.as_slice(), second_wire].concat());
}
