mod api;
mod model;
mod schema;
mod db;
mod gen_token;

use warp::Filter;

#[tokio::main]
async fn main() {
    // routing for the server
    let httpd = api::api().and(warp::path::end());
    // TODO: tls
    warp::serve(httpd).run(([127, 0, 0, 1], 3030)).await;
}