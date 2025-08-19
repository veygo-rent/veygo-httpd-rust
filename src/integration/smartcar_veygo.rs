use std::env;
use dotenv::dotenv;
use smartcar::*;
use smartcar::response::Access;
use smartcar::auth_client::AuthClient;
use smartcar::vehicle::Vehicle;

#[allow(dead_code)]
pub async fn get_vehicle_complete_token_by(exchange_code: &str, vehicle_vin: &str) -> Option<String> {
    dotenv().ok();

    let sc_client_id = env::var("SMARTCAR_CLIENT_ID").ok()?;
    let sc_client_secret = env::var("SMARTCAR_CLIENT_SECRET").ok()?;
    // NOTE: last param: false = live; true = sandbox. Match your Connect flow env.
    let auth_client = AuthClient::new(&sc_client_id, &sc_client_secret, "veygo-admin://smartcar", false);

    let auth = auth_client.exchange_code(exchange_code.trim()).await.ok()?; // (access, meta)
    let access = auth.0;

    let mut offset = 0usize;
    let limit = 10usize;

    // helper to scan a page
    async fn find_vehicle_on_page(
        access: &Access,
        offset: usize,
        limit: usize,
        wanted_vin: &str,
    ) -> Option<String> {
        let page = get_vehicles(access, Some(limit as i32), Some(offset as i32)).await.ok()?;
        for vehicle_id in page.0.vehicles {
            let vehicle = Vehicle::new(&vehicle_id, &access.access_token);
            if let Ok(v) = vehicle.vin().await {
                if v.0.vin == wanted_vin {
                    return Some(vehicle_id);
                }
            }
        }
        None
    }

    // first page
    if let Some(vid) = find_vehicle_on_page(&access, offset, limit, vehicle_vin).await {
        return Some(format!("{}${}", access.refresh_token, vid));
    }

    // total count from the first page
    let first_page = get_vehicles(&access, Some(limit as i32), Some(offset as i32)).await.ok()?;
    let total = first_page.0.paging.count as usize;

    // advance through the remaining pages
    offset += limit;
    while offset < total {
        if let Some(vid) = find_vehicle_on_page(&access, offset, limit, vehicle_vin).await {
            return Some(format!("{}${}", access.refresh_token, vid));
        }
        offset += limit; // <-- advance!
    }

    None
}

#[allow(dead_code)]
pub async fn renew_access_token(refresh_token: &str) -> Option<Access> {
    dotenv().ok();

    let sc_client_id = env::var("SMARTCAR_CLIENT_ID").ok()?;
    let sc_client_secret = env::var("SMARTCAR_CLIENT_SECRET").ok()?;
    // NOTE: last param: false = live; true = sandbox. Match your Connect flow env.
    let auth_client = AuthClient::new(&sc_client_id, &sc_client_secret, "veygo-admin://smartcar", false);
    
    let access = auth_client.exchange_refresh_token(refresh_token).await;
    if let Ok(a) = access {
        Some(a.0)
    } else {
        None
    }
}