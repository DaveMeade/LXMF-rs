#[test]
fn parses_python_reticulum_runtime_policy_accessors() {
    let input = r#"
[reticulum]
link_mtu_discovery = false
enable_remote_management = true
respond_to_probes = true
use_implicit_proof = false
discover_interfaces = true
required_discovery_value = 17
publish_blackhole = true
blackhole_sources = ["00112233445566778899AABBCCDDEEFF"]
interface_discovery_sources = ["FFEEDDCCBBAA99887766554433221100"]
autoconnect_discovered_interfaces = 4
"#;
    DaemonConfig::from_toml(input).expect("validate daemon Reticulum runtime policy");
    let policy = reticulum_daemon::config::ReticulumRuntimePolicy::from_toml(input)
        .expect("parse Reticulum runtime policy");

    assert!(!policy.link_mtu_discovery);
    assert!(policy.remote_management_enabled);
    assert!(policy.respond_to_probes);
    assert!(!policy.use_implicit_proof);
    assert!(policy.discover_interfaces);
    assert_eq!(policy.required_discovery_value, Some(17));
    assert!(policy.publish_blackhole);
    assert_eq!(policy.blackhole_sources, ["00112233445566778899aabbccddeeff"]);
    assert_eq!(
        policy.interface_discovery_sources,
        ["ffeeddccbbaa99887766554433221100"]
    );
    assert_eq!(policy.max_autoconnected_interfaces, 4);
}

#[test]
fn reticulum_runtime_policy_defaults_match_pinned_python() {
    let policy = reticulum_daemon::config::ReticulumRuntimePolicy::from_toml("")
        .expect("parse default config");

    assert!(policy.link_mtu_discovery);
    assert!(!policy.remote_management_enabled);
    assert!(!policy.respond_to_probes);
    assert!(policy.use_implicit_proof);
    assert!(!policy.discover_interfaces);
    assert_eq!(policy.required_discovery_value, None);
    assert!(!policy.publish_blackhole);
    assert_eq!(policy.max_autoconnected_interfaces, 0);
}

#[test]
fn rejects_invalid_reticulum_discovery_identity_hash() {
    let err = DaemonConfig::from_toml(
        r#"
[reticulum]
interface_discovery_sources = ["not-a-hash"]
"#,
    )
    .expect_err("invalid identity hash must fail");

    assert!(err.to_string().contains("interface discovery source not-a-hash is invalid"));
}
