use anyhow::Result;
use async_graphql::{Context, EmptySubscription, MergedObject, Object, SDLExportOptions, Schema};

#[derive(Default)]
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn say_hello(&self, _ctx: &Context<'_>) -> Result<&str> {
        Ok("hello")
    }
}

#[derive(Default)]
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn set_hello(&self, _ctx: &Context<'_>) -> Result<&str> {
        Ok("hello")
    }
}

#[derive(MergedObject, Default)]
pub struct Query(QueryRoot);

#[derive(MergedObject, Default)]
pub struct Mutation(MutationRoot);

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn build() -> async_graphql::SchemaBuilder<Query, Mutation, EmptySubscription> {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
}

#[allow(dead_code)]
pub fn export_sdl() -> String {
    let schema = build().enable_federation().finish();
    schema.sdl_with_options(SDLExportOptions::new())
}
