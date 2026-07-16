use super::Client;
use crate::app::{Envelope, EnvelopeResponse};
use crate::control::*;
use crate::{SdkBackend, SdkError};

fn execute<B: SdkBackend>(
    client: &Client<B>,
    operation_id: &str,
    command: bool,
    params: serde_json::Value,
) -> Result<SdkControlResult, SdkError> {
    let envelope = if command {
        Envelope::command(operation_id, params)
    } else {
        Envelope::query(operation_id, params)
    };
    let EnvelopeResponse { accepted, payload, .. } = client.backend.envelope_execute(envelope)?;
    Ok(SdkControlResult { operation_id: operation_id.to_owned(), accepted, value: payload })
}

macro_rules! impl_control_trait {
    ($trait:ident, $method:ident, $operation:ty) => {
        impl<B: SdkBackend> $trait for Client<B> {
            fn $method(
                &self,
                request: SdkControlRequest<$operation>,
            ) -> Result<SdkControlResult, SdkError> {
                execute(
                    self,
                    request.operation.operation_id(),
                    request.operation.is_command(),
                    request.params,
                )
            }
        }
    };
}

impl_control_trait!(RnsSdkRuntime, rns_runtime, RnsRuntimeOperation);
impl_control_trait!(RnsSdkTransport, rns_transport, RnsTransportOperation);
impl_control_trait!(RnsSdkInterfaces, rns_interfaces, RnsInterfacesOperation);
impl_control_trait!(RnsSdkDataPlane, rns_data_plane, RnsDataPlaneOperation);
impl_control_trait!(LxmfSdkRouter, lxmf_router, LxmfRouterOperation);
impl_control_trait!(LxmfSdkPropagation, lxmf_propagation, LxmfPropagationOperation);
