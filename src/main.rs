#![recursion_limit = "256"]
mod api;
mod integration;
mod methods;
mod model;
mod scheduled_tasks;
mod schema;

use diesel::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use once_cell::sync::Lazy;
use std::env;
use tokio::spawn;
use warp::Filter;

use std::net::IpAddr;
use std::str::FromStr;

type PgPool = Pool<ConnectionManager<PgConnection>>;

fn get_connection_pool() -> PgPool {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .expect("Could not build connection pool")
}

// Global pool initialized once at first access
static POOL: Lazy<PgPool> = Lazy::new(|| get_connection_pool());

#[tokio::main]

async fn main() {
    let _ = integration::sendgrid_veygo::send_email(
        Option::from("Veygo Server"),
        integration::sendgrid_veygo::make_email_obj("szhou@veygo.rent", "Danny"),
        "Main thread now running",
        "Main thread is now running",
        None,
        None,
    )
    .await;
    // routing for the server
    let react_app =
        warp::fs::dir("/app/www").or(warp::any().and(warp::fs::file("/app/www/index.html")));
    let httpd = api::api().and(warp::path::end()).or(react_app);
    let args: Vec<String> = env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(8080);
    println!("Starting server on port {}", port);
    let addr = IpAddr::from_str("::0").unwrap();
    // add routines
    spawn(scheduled_tasks::nightly_task());
    // starting the server
    warp::serve(httpd)
        .tls()
        .cert_path("/app/cert/httpd/veygo.rent.pem")
        .key_path("/app/cert/httpd/veygo.rent.key")
        .run((addr, port))
        .await;
}
