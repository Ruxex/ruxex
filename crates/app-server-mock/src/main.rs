use ruxex_app_server_mock::MockAppServer;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    MockAppServer::builder().build()?.run_stdio().await
}
