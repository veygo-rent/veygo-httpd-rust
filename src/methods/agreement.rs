use rand::Rng;
use diesel::prelude::*;
use crate::POOL;

pub fn generate_unique_agreement_confirmation() -> String {
    // Define the allowed characters: digits 0-9 and uppercase A-Z.
    let charset: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();

    loop {
        // Generate a random 8-character string.
        let confirmation: String = (0..8)
            .map(|_| {
                let idx = rng.gen_range(0, charset.len());
                charset[idx] as char
            })
            .collect();

        // Check if the generated confirmation already exists in the agreements table, synchronously.
        let exists = {
            let mut conn = POOL
                .clone()
                .get()
                .expect("Failed to get DB connection");

            // If there's an error performing the query, treat it as "exists = true" so we retry.
            diesel::select(diesel::dsl::exists(
                crate::schema::agreements::table
                    .filter(crate::schema::agreements::confirmation.eq(&confirmation))
            ))
                .get_result::<bool>(&mut conn)
                .unwrap_or_else(|e| {
                    eprintln!("Database error checking agreement confirmation: {:?}", e);
                    true
                })
        };

        // If the confirmation does not exist, return it.
        if !exists {
            return confirmation;
        }
        // Otherwise, loop again and generate a new one.
    }
}