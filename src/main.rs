mod api;
mod model;
mod schema;
mod methods;
mod integration;

use std::env;
use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use once_cell::sync::Lazy;
use warp::Filter;

type PgPool = Pool<ConnectionManager<PgConnection>>;

fn get_connection_pool() -> PgPool {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    // Refer to the `r2d2` documentation for more methods to use
    // when building a connection pool
    Pool::builder()
        .build(manager)
        .expect("Could not build connection pool")
}

// Global pool initialized once at first access
static POOL: Lazy<PgPool> = Lazy::new(|| {
    get_connection_pool()
});

#[tokio::main]
async fn main() {
    // routing for the server
    let httpd = api::api().and(warp::path::end());
    // TODO: tls
    let args: Vec<String> = env::args().collect();
    let port: u16 = args.get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    println!("Starting server on port {}", port);
    warp::serve(httpd).run(([127, 0, 0, 1], port)).await;
}