#![recursion_limit = "256"]
mod api;
mod integration;
mod methods;
mod model;
mod scheduled_tasks;
mod schema;
mod proj_config;

use diesel::{PgConnection, RunQueryDsl};
use diesel::r2d2::{ConnectionManager, Pool};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::env;
use tokio::spawn;
use warp::Filter;

use std::net::IpAddr;
use std::str::FromStr;
use diesel::query_builder::AsQuery;
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
    // delete all objects in the bucket if there are NO users (fresh installation)
    {
        let mut pool = POOL.get().unwrap();
        use crate::schema::renters::dsl as renter_query;

        // SELECT COUNT(*) FROM renters
        let renter_exists =
            diesel::select(diesel::dsl::exists(renter_query::renters.as_query())).get_result::<bool>(&mut pool);

        match renter_exists {
            Ok(false) => {
                // No users present -> treat as fresh install and wipe prior user storage
                if let Err(e) = integration::gcloud_storage_veygo::delete_all_objects().await {
                    eprintln!("Bucket wipe failed: {e}");
                } else {
                    eprintln!("Bucket wiped: no renters found (fresh install)");
                }
            }
            Ok(_) => {
                // At least one user exists; do nothing
            }
            Err(e) => {
                // On DB error, DO NOT delete it; just log
                eprintln!("DB check for renters failed; not deleting bucket: {e}");
            }
        }
    }

    let httpd = api::api().and(warp::path::end());
    let args: Vec<String> = env::args().collect();
    let port: u16 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(8080);
    println!("Starting server on port {}", port);
    let addr = IpAddr::from_str("::0").unwrap();
    // add routines
    spawn(scheduled_tasks::nightly_task());
    // starting the server
    warp::serve(httpd)
        .run((addr, port))
        .await;
}
