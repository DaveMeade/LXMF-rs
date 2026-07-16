macro_rules! rns_operation {
    ($id:literal, $group:literal, $kind:literal, $capability:literal, $method:literal) => {
        SdkOperationSpec {
            id: $id,
            group: $group,
            kind: $kind,
            transport_variant: "rpc",
            description: "Typed SDK access to a Reticulum daemon operation.",
            aliases: &[$method],
            required_capabilities: &[$capability],
            rpc_method: $method,
        }
    };
}

const RNS_SDK_OPERATION_SPECS: &[SdkOperationSpec] = &[
    rns_operation!("rns.runtime.status", "rns_runtime", "query", "sdk.capability.rns_runtime", "daemon_status_ex"),
    rns_operation!("rns.runtime.clear.messages", "rns_runtime", "command", "sdk.capability.rns_runtime", "clear_messages"),
    rns_operation!("rns.runtime.clear.resources", "rns_runtime", "command", "sdk.capability.rns_runtime", "clear_resources"),
    rns_operation!("rns.runtime.clear.peers", "rns_runtime", "command", "sdk.capability.rns_runtime", "clear_peers"),
    rns_operation!("rns.runtime.clear.all", "rns_runtime", "command", "sdk.capability.rns_runtime", "clear_all"),
    rns_operation!("rns.transport.path.status", "rns_transport", "query", "sdk.capability.rns_transport", "path_status"),
    rns_operation!("rns.transport.path.request", "rns_transport", "command", "sdk.capability.rns_transport", "request_path"),
    rns_operation!("rns.transport.path.next_hop", "rns_transport", "query", "sdk.capability.rns_transport", "next_hop"),
    rns_operation!("rns.transport.path.next_hop_interface", "rns_transport", "query", "sdk.capability.rns_transport", "next_hop_if_name"),
    rns_operation!("rns.transport.path.first_hop_timeout", "rns_transport", "query", "sdk.capability.rns_transport", "first_hop_timeout"),
    rns_operation!("rns.transport.path.drop", "rns_transport", "command", "sdk.capability.rns_transport", "drop_path"),
    rns_operation!("rns.transport.path.drop_all_via", "rns_transport", "command", "sdk.capability.rns_transport", "drop_all_via"),
    rns_operation!("rns.transport.announce_queues.drop", "rns_transport", "command", "sdk.capability.rns_transport", "drop_announce_queues"),
    rns_operation!("rns.transport.rate_table", "rns_transport", "query", "sdk.capability.rns_transport", "get_rate_table"),
    rns_operation!("rns.interfaces.set", "rns_interfaces", "command", "sdk.capability.rns_interfaces", "set_interfaces"),
    rns_operation!("rns.interfaces.discovered", "rns_interfaces", "query", "sdk.capability.rns_interfaces", "discovered_interfaces"),
    rns_operation!("rns.data_plane.links.count", "rns_data_plane", "query", "sdk.capability.rns_data_plane", "link_count"),
    rns_operation!("rns.data_plane.packet.rssi", "rns_data_plane", "query", "sdk.capability.rns_data_plane", "get_packet_rssi"),
    rns_operation!("rns.data_plane.packet.snr", "rns_data_plane", "query", "sdk.capability.rns_data_plane", "get_packet_snr"),
    rns_operation!("rns.data_plane.packet.q", "rns_data_plane", "query", "sdk.capability.rns_data_plane", "get_packet_q"),
    rns_operation!("rns.data_plane.announce.now", "rns_data_plane", "command", "sdk.capability.rns_data_plane", "announce_now"),
    rns_operation!("rns.data_plane.announce.delivery", "rns_data_plane", "query", "sdk.capability.rns_data_plane", "announce_delivery"),
    rns_operation!("rns.data_plane.announce.received", "rns_data_plane", "query", "sdk.capability.rns_data_plane", "announce_received"),
    rns_operation!("rns.transport.blackholes.list", "rns_transport", "query", "sdk.capability.rns_transport", "get_blackholed_identities"),
    rns_operation!("rns.transport.blackholes.add", "rns_transport", "command", "sdk.capability.rns_transport", "blackhole_identity"),
    rns_operation!("rns.transport.blackholes.remove", "rns_transport", "command", "sdk.capability.rns_transport", "unblackhole_identity"),
    rns_operation!("app.router.stats", "router", "query", "sdk.capability.router_management", "router_stats"),
    rns_operation!("app.router.storage_policy.get", "router", "query", "sdk.capability.router_management", "router_storage_policy_get"),
    rns_operation!("app.router.storage_policy.set", "router", "command", "sdk.capability.router_management", "router_storage_policy_set"),
];
