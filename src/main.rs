use rust_web_tools_test::graphql_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    graphql_server::main().await
}
