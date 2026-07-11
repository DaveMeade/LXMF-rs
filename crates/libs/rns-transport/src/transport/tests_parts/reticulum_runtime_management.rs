#[tokio::test]
async fn packet_signal_cache_returns_latest_value_and_evicts_like_python() {
    let identity = PrivateIdentity::new_from_rand(OsRng);
    let transport = Transport::new(TransportConfig::new("signal-cache", &identity, true));
    let repeated = Hash::new_from_slice(b"repeated");

    transport
        .record_packet_signal(
            repeated,
            PacketSignal { rssi: Some(-70.0), snr: Some(3.0), q: Some(40.0) },
        )
        .await;
    transport
        .record_packet_signal(
            repeated,
            PacketSignal { rssi: Some(-65.0), snr: Some(5.0), q: Some(55.0) },
        )
        .await;
    assert_eq!(transport.packet_signal(&repeated).await.expect("signal").rssi, Some(-65.0));

    for index in 0..512_u64 {
        transport
            .record_packet_signal(
                Hash::new_from_slice(&index.to_be_bytes()),
                PacketSignal { q: Some(index as f64), ..Default::default() },
            )
            .await;
    }

    assert_eq!(transport.packet_signal(&repeated).await, None);
    let newest = Hash::new_from_slice(&511_u64.to_be_bytes());
    assert_eq!(transport.packet_signal(&newest).await.expect("newest").q, Some(511.0));
}
