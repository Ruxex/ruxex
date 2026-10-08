use super::AppServerSession;
use color_eyre::eyre::Result;
use genex_app_server_protocol::ClientRequest;
use genex_app_server_protocol::ModelProviderListParams;
use genex_app_server_protocol::ModelProviderListResponse;

impl AppServerSession {
    pub(crate) async fn model_provider_list(&mut self) -> Result<ModelProviderListResponse> {
        let request_id = self.next_request_id();
        self.client
            .request_typed(ClientRequest::ModelProviderList {
                request_id,
                params: ModelProviderListParams {},
            })
            .await
            .map_err(|err| {
                super::bootstrap_request_error(
                    "modelProvider/list failed during TUI bootstrap",
                    err,
                )
            })
    }
}
