use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::{response, Extension, TypedHeader};
use axum::{response::IntoResponse, routing::get, Router};
use opentelemetry::trace::FutureExt;
use std::net::SocketAddr;

use rust_web_tools_test::graphql;
use rust_web_tools_test::tools::{
    access_token::AccessToken, setup::setup_tracing_and_opentelemetry, trace::Trace,
};
use tracing::{debug, info};

async fn graphql_handler(
    schema: Extension<graphql::AppSchema>,
    token: AccessToken,
    trace: Trace,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let parent_cx = opentelemetry::global::get_text_map_propagator(|prop| prop.extract(&trace));
    schema
        .execute(req.into_inner().data(token))
        .with_context(parent_cx)
        .await
        .into()
}

async fn graphiql() -> impl IntoResponse {
    response::Html(GraphiQLSource::build().endpoint("/").finish())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _guard = if let Ok(sentry_dsn) = std::env::var("SENTRY_DSN") {
        info!("sentry initialized");
        Some(sentry::init((
            sentry_dsn,
            sentry::ClientOptions {
                before_send: Some(std::sync::Arc::new(|event| {
                    tracing::debug!("Sending event to Sentry: {}", event.event_id);
                    Some(event)
                })),
                ..Default::default()
            },
        )))
    } else {
        None
    };

    let tracer = setup_tracing_and_opentelemetry("this-service-name", true)?;

    let schema_builder = graphql::build()
        .enable_federation()
        .extension(async_graphql::extensions::Logger);
    let schema_builder = if let Some(tracer) = tracer {
        schema_builder.extension(async_graphql::extensions::OpenTelemetry::new(tracer))
    } else {
        schema_builder
    };

    let schema = schema_builder.finish();

    let cors = tower_http::cors::CorsLayer::new()
        .allow_credentials(false)
        .allow_headers(tower_http::cors::Any)
        .allow_origin(tower_http::cors::AllowOrigin::mirror_request());
    let router = Router::new()
        .route("/", get(graphiql).post(graphql_handler))
        .layer(Extension(schema))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    let server = axum::Server::bind(&addr).serve(router.into_make_service());

    info!("server listening {:?}", addr);

    server
        .with_graceful_shutdown(async {
            use tokio::signal::{
                ctrl_c,
                unix::{signal, SignalKind},
            };

            let mut sig_int = signal(SignalKind::interrupt()).unwrap();
            let mut sig_term = signal(SignalKind::terminate()).unwrap();
            tokio::select! {
                _ = sig_int.recv() => debug!("SIGINT received"),
                _ = sig_term.recv() => debug!("SIGTERM received"),
                _ = ctrl_c() => debug!("'Ctrl C' received"),
            }
            debug!("gracefully shutting down");
        })
        .await?;

    info!("server shutdown");

    Ok(())
}
