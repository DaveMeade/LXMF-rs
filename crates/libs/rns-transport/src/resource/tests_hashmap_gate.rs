/// A request dispatched in response to a hashmap update must refresh the
/// timestamp the exhaustion gate measures against.
///
/// A hashmap update can arrive and still leave the fragments the receiver
/// wants next unmapped — small hashmap segments, or a window grown past one
/// segment — in which case that follow-up request is itself another hashmap
/// request. If `last_request` still points at the *previous* one, the gate's
/// wait looks already expired and the next arriving part emits a duplicate.
/// Each duplicate walks a reference sender's serving window forward by a whole
/// hashmap segment while the fragment frontier barely moves, which is the
/// silent stall this gate was added to prevent.
#[test]
fn a_hashmap_update_reply_refreshes_the_exhaustion_gate() {
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

    // Two hashmap slots per segment over eight parts, so the map is still
    // exhausted after the first update lands.
    let segment_len = 2;
    let random_hash = [7u8; RANDOM_HASH_SIZE];
    let bodies: Vec<Vec<u8>> =
        (0..8usize).map(|i| format!("part-{i:05}").into_bytes()).collect();
    let mut first_segment = Vec::with_capacity(MAPHASH_LEN * segment_len);
    for body in bodies.iter().take(segment_len) {
        first_segment.extend_from_slice(&map_hash(body, &random_hash));
    }
    let adv = ResourceAdvertisement {
        transfer_size: bodies.iter().map(|body| body.len() as u64).sum(),
        data_size: bodies.iter().map(|body| body.len() as u64).sum(),
        parts: bodies.len() as u32,
        hash: Hash::new_from_slice(&[11u8; 32]),
        random_hash,
        original_hash: Hash::new_from_slice(&[11u8; 32]),
        segment_index: 1,
        total_segments: 1,
        request_id: None,
        flags: 0,
        hashmap: first_segment,
    };
    let adv_packet = resource_packet(
        PacketContext::ResourceAdvrtisement,
        &adv.pack().expect("advertisement"),
        *link.id(),
    );
    assert_eq!(manager.handle_packet(&adv_packet, &mut link).len(), 1);

    let before = manager.incoming.get(&adv.hash).expect("receiver").last_request;

    let update = ResourceHashUpdate {
        resource_hash: adv.hash,
        segment: 1,
        hashmap: vec![0u8; MAPHASH_LEN * segment_len],
    };
    let update_packet =
        resource_packet(PacketContext::ResourceHashUpdate, &update.encode().expect("hash update encodes"), *link.id());
    assert_eq!(manager.handle_packet(&update_packet, &mut link).len(), 1);

    let after = manager.incoming.get(&adv.hash).expect("receiver").last_request;
    assert!(
        after > before,
        "the reply to a hashmap update is a send, and must restart the gate's wait"
    );
}
