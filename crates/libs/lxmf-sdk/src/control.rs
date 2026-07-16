use crate::SdkError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SdkControlRequest<O> {
    pub operation: O,
    #[serde(default)]
    pub params: JsonValue,
}

impl<O> SdkControlRequest<O> {
    pub fn new(operation: O, params: JsonValue) -> Self {
        Self { operation, params }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SdkControlResult {
    pub operation_id: String,
    pub accepted: bool,
    pub value: JsonValue,
}

macro_rules! control_operations {
    ($name:ident { $($variant:ident => ($id:literal, $command:literal)),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        #[non_exhaustive]
        pub enum $name { $($variant),+ }

        impl $name {
            pub(crate) fn operation_id(self) -> &'static str {
                match self { $(Self::$variant => $id),+ }
            }

            pub(crate) fn is_command(self) -> bool {
                match self { $(Self::$variant => $command),+ }
            }
        }
    };
}

control_operations!(RnsRuntimeOperation {
    Status => ("rns.runtime.status", false),
    ClearMessages => ("rns.runtime.clear.messages", true),
    ClearResources => ("rns.runtime.clear.resources", true),
    ClearPeers => ("rns.runtime.clear.peers", true),
    ClearAll => ("rns.runtime.clear.all", true),
});

control_operations!(RnsTransportOperation {
    PathStatus => ("rns.transport.path.status", false),
    RequestPath => ("rns.transport.path.request", true),
    NextHop => ("rns.transport.path.next_hop", false),
    NextHopInterface => ("rns.transport.path.next_hop_interface", false),
    FirstHopTimeout => ("rns.transport.path.first_hop_timeout", false),
    DropPath => ("rns.transport.path.drop", true),
    DropAllVia => ("rns.transport.path.drop_all_via", true),
    DropAnnounceQueues => ("rns.transport.announce_queues.drop", true),
    RateTable => ("rns.transport.rate_table", false),
    BlackholesList => ("rns.transport.blackholes.list", false),
    BlackholeAdd => ("rns.transport.blackholes.add", true),
    BlackholeRemove => ("rns.transport.blackholes.remove", true),
});

control_operations!(RnsInterfacesOperation {
    Set => ("rns.interfaces.set", true),
    Discovered => ("rns.interfaces.discovered", false),
});

control_operations!(RnsDataPlaneOperation {
    LinkCount => ("rns.data_plane.links.count", false),
    PacketRssi => ("rns.data_plane.packet.rssi", false),
    PacketSnr => ("rns.data_plane.packet.snr", false),
    PacketQuality => ("rns.data_plane.packet.q", false),
    AnnounceNow => ("rns.data_plane.announce.now", true),
    AnnounceDelivery => ("rns.data_plane.announce.delivery", false),
    AnnounceReceived => ("rns.data_plane.announce.received", false),
});

control_operations!(LxmfRouterOperation {
    Stats => ("app.router.stats", false),
    StoragePolicyGet => ("app.router.storage_policy.get", false),
    StoragePolicySet => ("app.router.storage_policy.set", true),
});

control_operations!(LxmfPropagationOperation {
    Status => ("app.propagation.status", false),
    Enable => ("app.propagation.enable", true),
    NodeGet => ("app.propagation.node.get", false),
    NodeSet => ("app.propagation.node.set", true),
    NodeList => ("app.propagation.node.list", false),
    PeerSync => ("app.propagation.peer_sync", true),
    PeerMaintenance => ("app.propagation.peer_maintenance", true),
    RemoteStatus => ("app.propagation.remote_status", false),
    RemoteFetch => ("app.propagation.remote_fetch", true),
    RemoteDownload => ("app.propagation.remote_download", true),
    RemoteSync => ("app.propagation.remote_sync", true),
    RemoteUnpeer => ("app.propagation.remote_unpeer", true),
    Ingest => ("app.propagation.ingest", true),
    Fetch => ("app.propagation.fetch", true),
    DeliveryPolicyGet => ("app.propagation.delivery_policy.get", false),
    DeliveryPolicySet => ("app.propagation.delivery_policy.set", true),
    ControlAllow => ("app.propagation.control.allow", true),
    ControlDisallow => ("app.propagation.control.disallow", true),
});

pub trait RnsSdkRuntime {
    fn rns_runtime(
        &self,
        request: SdkControlRequest<RnsRuntimeOperation>,
    ) -> Result<SdkControlResult, SdkError>;
}

pub trait RnsSdkTransport {
    fn rns_transport(
        &self,
        request: SdkControlRequest<RnsTransportOperation>,
    ) -> Result<SdkControlResult, SdkError>;
}

pub trait RnsSdkInterfaces {
    fn rns_interfaces(
        &self,
        request: SdkControlRequest<RnsInterfacesOperation>,
    ) -> Result<SdkControlResult, SdkError>;
}

pub trait RnsSdkDataPlane {
    fn rns_data_plane(
        &self,
        request: SdkControlRequest<RnsDataPlaneOperation>,
    ) -> Result<SdkControlResult, SdkError>;
}

pub trait LxmfSdkRouter {
    fn lxmf_router(
        &self,
        request: SdkControlRequest<LxmfRouterOperation>,
    ) -> Result<SdkControlResult, SdkError>;
}

pub trait LxmfSdkPropagation {
    fn lxmf_propagation(
        &self,
        request: SdkControlRequest<LxmfPropagationOperation>,
    ) -> Result<SdkControlResult, SdkError>;
}
