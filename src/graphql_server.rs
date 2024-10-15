use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{response, Extension};
use axum::{response::IntoResponse, routing::get, Router};
#[cfg(feature = "with-opentelemetry")]
use opentelemetry::trace::FutureExt;

use super::graphql;
#[cfg(feature = "with-opentelemetry")]
use super::tools::parent_trace_context::ParentTraceContext;
#[cfg(all(feature = "with-opentelemetry", feature = "with-sentry"))]
use super::tools::setup_tracing;
use super::tools::{db::Database, server};

async fn graphql_handler(
    schema: Extension<graphql::AppSchema>,
    #[cfg(feature = "with-opentelemetry")] parent_trace_context: ParentTraceContext,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let schema = schema.execute(req.into_inner());

    #[cfg(feature = "with-opentelemetry")]
    let schema = schema.with_context(parent_trace_context.get());

    schema.await.into()
}

#[cfg(debug_assertions)]
async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").finish())
}

#[cfg(not(debug_assertions))]
async fn graphiql() -> impl IntoResponse {
    use axum::http::StatusCode;
    StatusCode::BAD_REQUEST
}

pub async fn main() -> anyhow::Result<()> {
    #[cfg(all(feature = "with-opentelemetry", feature = "with-sentry"))]
    let guard = setup_tracing::setup()?;

    let schema_builder = graphql::build()
        .data(async_graphql::dataloader::DataLoader::new(
            Database::new_from_env().await?,
            tokio::task::spawn,
        ))
        .enable_federation()
        .extension(async_graphql::extensions::Logger);

    #[cfg(all(feature = "with-opentelemetry", feature = "with-sentry"))]
    let schema_builder = guard.add_extension(schema_builder);
    let schema = schema_builder.finish();

    let cors = tower_http::cors::CorsLayer::new()
        .allow_credentials(false)
        .allow_headers(tower_http::cors::Any)
        .allow_origin(tower_http::cors::AllowOrigin::mirror_request());
    let router = Router::new()
        .route("/", get(graphiql).post(graphql_handler))
        .layer(Extension(schema))
        .layer(cors);

    server::run(router, Some(8000)).await?;

    Ok(())
}
