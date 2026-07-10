#[test]
fn parses_simulated_meshtastic_interface() {
    let input = r#"
interfaces = [
  { type = "MeshtasticInterface", enabled = true, name = "mesh-sim", simulation_loopback = true, simulation_node_id = 42, modem_preset = 8, hop_limit = 3, bitrate = 1200, max_payload_bytes = 180, send_delay_ms = 25, destination_cache_size = 32 }
]
"#;
    let cfg = DaemonConfig::from_toml(input).expect("parse simulated Meshtastic config");
    let iface = &cfg.interfaces[0];

    assert_eq!(iface.kind, "meshtastic");
    assert_eq!(iface.name.as_deref(), Some("mesh-sim"));
    assert_eq!(iface.simulation_loopback, Some(true));
    assert_eq!(iface.simulation_node_id, Some(42));
    assert_eq!(iface.modem_preset, Some(8));
    assert_eq!(iface.hop_limit, Some(3));
    assert_eq!(iface.bitrate, Some(1_200));
    assert_eq!(iface.max_payload_bytes, Some(180));
    assert_eq!(iface.send_delay_ms, Some(25));
    assert_eq!(iface.destination_cache_size, Some(32));

    let settings = iface.settings_json().expect("Meshtastic settings");
    assert_eq!(settings["simulation_loopback"], true);
    assert_eq!(settings["simulation_node_id"], 42);
    assert_eq!(settings["modem_preset"], 8);
    assert_eq!(settings["hardware_unverified"], true);
}

#[test]
fn rejects_meshtastic_daemon_config_without_explicit_simulation() {
    let input = r#"
interfaces = [
  { type = "meshtastic", enabled = true, name = "mesh-ambiguous" }
]
"#;
    let err = DaemonConfig::from_toml(input)
        .expect_err("Meshtastic daemon config must identify its simulated bearer");

    assert!(
        err.to_string().contains("simulation_loopback must be true"),
        "unexpected validation error: {err}"
    );
}
