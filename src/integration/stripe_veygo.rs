use crate::model::NewPaymentMethod;
use dotenv::dotenv;
use std::env;
use std::str::FromStr;
use stripe::{Client, PaymentMethod, PaymentMethodId, StripeError, Customer, CreateCustomer, CustomerId, SetupIntent, CreateSetupIntent};

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

pub async fn attach_payment_method_to_stripe_customer(
    stripe_customer_id: String,
    pm_id: String
) -> Result<SetupIntent, StripeError> {
    dotenv().ok();
    let stripe_secret_key = env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
    let client = Client::new(stripe_secret_key);
    let payment_method_id = PaymentMethodId::from_str(pm_id.as_str()).unwrap();
    let customer_id = CustomerId::from_str(stripe_customer_id.as_str()).unwrap();
    SetupIntent::create(
        &client,
        CreateSetupIntent {
            attach_to_self: Some(false),
            automatic_payment_methods: None,
            confirm: Some(true),
            customer: Some(customer_id),
            description: None,
            expand: &[],
            flow_directions: None,
            mandate_data: None,
            metadata: None,
            on_behalf_of: None,
            payment_method: Some(payment_method_id),
            payment_method_configuration: None,
            payment_method_data: None,
            payment_method_options: None,
            payment_method_types: None,
            return_url: None,
            single_use: None,
            use_stripe_sdk: None,
        }
    ).await
}
