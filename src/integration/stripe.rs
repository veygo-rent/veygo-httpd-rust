use crate::model::NewPaymentMethod;
use dotenv::dotenv;
use std::env;
use std::str::FromStr;
use stripe::{Client, PaymentMethod, PaymentMethodId, StripeError, Customer, CreateCustomer};

pub async fn create_new_payment_method(
    pm_id: &str,
    md5: String,
    cardholder_name: String,  // Required as Stripe does not return full name
    renter_id: i32,           // Must be provided
    nickname: Option<String>, // Optional user-defined alias
) -> Result<NewPaymentMethod, StripeError> {
    dotenv().ok();
    let stripe_secret_key = env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
    let client = Client::new(stripe_secret_key);
    let payment_id = PaymentMethodId::from_str(pm_id).unwrap();
    let payment_method = PaymentMethod::retrieve(&client, &payment_id, &[]).await;

    match payment_method {
        Ok(payment_method) => {
            let card = payment_method.card.unwrap();
            let mut masked_card_number = format!("**** **** **** {}", card.last4);
            if card.brand == "amex" {
                masked_card_number = format!("**** ****** *{}", card.last4);
            }
            let network = card.brand; // Visa, Mastercard, etc.
            let expiration = format!("{:02}/{}", card.exp_month, card.exp_year);

            Ok(NewPaymentMethod {
                cardholder_name,
                masked_card_number,
                network,
                expiration,
                token: pm_id.to_string(),
                md5,
                nickname,
                is_enabled: true,
                renter_id,
                last_used_date_time: None,
            })
        }
        Err(error) => {
            Err(error)
        }
    }
}

pub async fn create_stripe_customer(
    name_data: String,
    phone_data: String,
    email_data: String
) -> Result<Customer, StripeError> {
    dotenv().ok();
    let stripe_secret_key = env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
    let client = Client::new(stripe_secret_key);
    Customer::create(
        &client,
        CreateCustomer {
            name: Some(name_data.as_str()),
            email: Some(email_data.as_str()),
            phone: Some(phone_data.as_str()),
            metadata: Some(std::collections::HashMap::from([(
                String::from("async-stripe"),
                String::from("true"),
            )])),

            ..Default::default()
        },
    ).await
}
