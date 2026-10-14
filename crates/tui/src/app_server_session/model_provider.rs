use super::AppServerSession;
use color_eyre::eyre::Result;
use ruxex_app_server_protocol::ClientRequest;
use ruxex_app_server_protocol::ModelProviderCapabilitiesReadParams;
use ruxex_app_server_protocol::ModelProviderCapabilitiesReadResponse;
use ruxex_app_server_protocol::ModelProviderReadParams;
use ruxex_app_server_protocol::ModelProviderReadResponse;

impl AppServerSession {
    pub(crate) async fn model_provider_read(&mut self) -> Result<ModelProviderReadResponse> {
        let request_id = self.next_request_id();
        self.client
            .request_typed(ClientRequest::ModelProviderRead {
                request_id,
                params: ModelProviderReadParams {},
            })
            .await
            .map_err(|err| {
                super::bootstrap_request_error(
                    "modelProvider/read failed during TUI bootstrap",
                    err,
                )
            })
    }

    pub(crate) async fn model_provider_capabilities_read(
        &mut self,
    ) -> Result<ModelProviderCapabilitiesReadResponse> {
        let request_id = self.next_request_id();
        self.client
            .request_typed(ClientRequest::ModelProviderCapabilitiesRead {
                request_id,
                params: ModelProviderCapabilitiesReadParams {},
            })
            .await
            .map_err(|err| {
                super::bootstrap_request_error(
                    "modelProvider/capabilities/read failed during TUI bootstrap",
                    err,
                )
            })
    }
}
