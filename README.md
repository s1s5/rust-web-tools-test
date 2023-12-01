# rust-web-tools
- `git remote add -f tools https://github.com/s1s5/rust-web-tools.git`
- `git subtree add --prefix=src/tools --squash tools main`
- `git subtree push --prefix=src/tools tools main`
- `git subtree pull --prefix=src/tools --squash tools main`

## use graphql with opentelemetry
```rust
async fn graphql_handler(
    schema: Extension<graphql::AppSchema>,
    token: AccessToken,
    request_id: RequestId,
    trace: Trace,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let parent_cx = opentelemetry::global::get_text_map_propagator(|prop| prop.extract(&trace));
     schema
        .execute(req.into_inner().data(token).data(request_id))
        .with_context(parent_cx)
        .await
        .into()
}
```

## setup sentry
```rust
let _guard = if let Ok(sentry_dsn) = std::env::var("SENTRY_DSN") {
    info!("sentry initialized");
    Some(sentry::init((
        sentry_dsn,
        sentry::ClientOptions {
            before_send: Some(Arc::new(|event| {
                tracing::debug!("Sending event to Sentry: {}", event.event_id);
                Some(event)
            })),
            ..Default::default()
        },
    )))
} else {
    None
};
let schema_builder = if _guard.is_some() {
    schema_builder.extension(rust_web_tools::async_graphql_sentry_extension::Sentry)
} else {
    schema_builder
};
```

## setup schema with opentelemetry
```rust
let tracer = setup_tracing_and_opentelemetry();
let schema_builder = graphql::build()
    .data(SecretKey(
        std::env::var("SECRET_KEY").unwrap_or("secret".to_string()),
    ))
    .enable_federation()
    .extension(async_graphql::extensions::Logger);
let schema_builder = if let Some(tracer) = tracer {
    schema_builder.extension(async_graphql::extensions::OpenTelemetry::new(tracer))
} else {
    schema_builder
};
let schema = schema_builder.finish();
// ...
```

## setup server
```rust
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
server
    .with_graceful_shutdown(async {
        let mut sig_int = signal(SignalKind::interrupt()).unwrap();
        let mut sig_term = signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = sig_int.recv() => debug!("receive SIGINT"),
            _ = sig_term.recv() => debug!("receive SIGTERM"),
            _ = ctrl_c() => debug!("receive Ctrl C"),
        }
        debug!("gracefully shutting down");
    })
    .await?;
```
