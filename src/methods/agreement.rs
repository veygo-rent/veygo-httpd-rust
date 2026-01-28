use crate::POOL;
use crate::helper_model::VeygoError;
use diesel::prelude::*;
use rand::Rng;
use rand::seq::SliceRandom;

pub fn generate_unique_agreement_confirmation() -> Result<String, VeygoError> {
    // Define the allowed characters: digits 0-9 and uppercase A-Z.
    let mut charset: Vec<u8> = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".to_vec(); // Convert to Vec<u8>

    let mut rng = rand::rng();

    // Shuffle the character set
    (&mut charset).shuffle(&mut rng);
    let mut conn = POOL.get().unwrap();

    loop {
        // Generate a random 8-character string.
        let confirmation: String = (0..8)
            .map(|_| {
                let idx = rng.random_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        // Check if the generated confirmation already exists in the agreement table, synchronously.
        let exists = {

            // If there's an error performing the query, treat it as "exists = true" so we retry.
            diesel::select(diesel::dsl::exists(
                crate::schema::agreements::table
                    .filter(crate::schema::agreements::confirmation.eq(&confirmation)),
            ))
            .get_result::<bool>(&mut conn)
        };

        // If the confirmation does not exist, return it.
        match exists {
            Ok(result) => {
                if !result {
                    return Ok(confirmation);
                }
            }
            Err(_) => {
                return Err(VeygoError::InternalServerError);
            }
        }
    }
}
