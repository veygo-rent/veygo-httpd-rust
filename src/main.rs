mod api;
mod integration;
mod methods;
mod model;
mod scheduled_tasks;
mod schema;

use diesel::r2d2::{ConnectionManager, Pool};
use diesel::PgConnection;
use dotenv::dotenv;
use once_cell::sync::Lazy;
use std::env;
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
    // routing for the server
    let httpd = api::api().and(warp::path::end());
    let args: Vec<String> = env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(8080);
    println!("Starting server on port {}", port);
    let addr = IpAddr::from_str("::0").unwrap();
    integration::sendgrid_veygo::send_email(
        integration::sendgrid_veygo::make_email_obj("info@veygo.rent", Option::from("Server")),
        integration::sendgrid_veygo::make_email_obj(
            "szhou@veygo.rent",
            Option::from("Shenghong Zhou"),
        ),
        "Server Started",
        "Server Started",
        None,
        None,
    )
    .await
    .unwrap_or_else(|_| println!("Danny is stupid"));
    warp::serve(httpd)
        .tls()
        .cert_path("/app/cert/veygo.rent.pem")
        .key_path("/app/cert/veygo.rent.key")
        .run((addr, port))
        .await;
}
