use super::build_mesh_client_config;

#[test]
fn mesh_nodes_enable_forwarding_with_one_tcp_connection_per_ring_edge() {
    let config = build_mesh_client_config(0, &[41_000, 41_001, 41_002, 41_003]);

    assert!(config.starts_with("[reticulum]\nenable_transport = true\n"));
    assert!(config.contains("port = 41001"));
    assert!(!config.contains("port = 41003"));
    assert_eq!(config.matches("[[interfaces]]").count(), 1);
}
