use super::AppServerSession;
use color_eyre::eyre::Result;
use genex_app_server_protocol::ClientRequest;
use genex_app_server_protocol::ModelProviderReadParams;
use genex_app_server_protocol::ModelProviderReadResponse;

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
}
