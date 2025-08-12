use crate::model::{NewPaymentMethod, PaymentType};
use dotenv::dotenv;
use std::env;
use std::str::FromStr;
use stripe::{
    Client, CreateCustomer, CreatePaymentIntent, CreatePaymentIntentAutomaticPaymentMethods,
    CreatePaymentIntentAutomaticPaymentMethodsAllowRedirects, PaymentMethodId, SetupIntent,
    CreatePaymentIntentPaymentMethodOptions, CreatePaymentIntentPaymentMethodOptionsCard,
    CreatePaymentIntentPaymentMethodOptionsCardRequestExtendedAuthorization, PaymentMethod,
    CreatePaymentIntentPaymentMethodOptionsCardRequestIncrementalAuthorization, Currency,
    CreatePaymentIntentPaymentMethodOptionsCardRequestMulticapture, CreateSetupIntent,
    CreateSetupIntentAutomaticPaymentMethods, StripeError, CancelPaymentIntent, Customer,
    CreateSetupIntentAutomaticPaymentMethodsAllowRedirects, CustomerId, PaymentIntentStatus,
    PaymentIntent, PaymentIntentCaptureMethod, PaymentIntentOffSession,
};

pub async fn create_new_payment_method(
    pm_id: &str,
    cardholder_name: &String, // Required as Stripe does not return the full name
    renter_id: &i32,          // Must be provided
    nickname: &Option<String>, // Optional user-defined alias
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
                cardholder_name: cardholder_name.to_string(),
                masked_card_number,
                network,
                expiration,
                token: pm_id.to_string(),
                md5: card.fingerprint.unwrap(),
                nickname: nickname.clone(),
                is_enabled: true,
                renter_id: renter_id.clone(),
                last_used_date_time: None,
            })
        }
        Err(error) => Err(error),
    }
}

pub async fn create_stripe_customer(
    name_data: &String,
    phone_data: &String,
    email_data: &String,
) -> Result<Customer, StripeError> {
    dotenv().ok();
    let stripe_secret_key = env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
    let client = Client::new(stripe_secret_key);
    Customer::create(
        &client,
        CreateCustomer {
            name: Some(name_data),
            email: Some(email_data),
            phone: Some(phone_data),

            ..Default::default()
        },
    )
    .await
}

pub async fn attach_payment_method_to_stripe_customer(
    stripe_customer_id: &String,
    pm_id: &String,
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
            automatic_payment_methods: Some(CreateSetupIntentAutomaticPaymentMethods {
                allow_redirects: Some(
                    CreateSetupIntentAutomaticPaymentMethodsAllowRedirects::Never,
                ),
                enabled: true,
            }),
            confirm: Some(true),
            customer: Some(customer_id),
            expand: &[],
            payment_method: Some(payment_method_id),
            ..Default::default()
        },
    )
    .await
}

pub async fn create_payment_intent(
    description: &String,
    customer_id_data: &String,
    payment_id_data: &String,
    amount: &i64,
    capture_method: PaymentIntentCaptureMethod,
    statement_descriptor_suffix: Option<&str>,
) -> Result<PaymentIntent, StripeError> {
    dotenv().ok();
    let stripe_secret_key = env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
    let client = Client::new(stripe_secret_key);
    let customer_id = CustomerId::from_str(customer_id_data).unwrap();
    let payment_method_id = PaymentMethodId::from_str(payment_id_data).unwrap();
    PaymentIntent::create(
        &client,
        CreatePaymentIntent {
            amount: amount.clone(),
            application_fee_amount: None,
            automatic_payment_methods: Some(CreatePaymentIntentAutomaticPaymentMethods{
                allow_redirects: Some(CreatePaymentIntentAutomaticPaymentMethodsAllowRedirects::Never),
                enabled: true,
            }),
            capture_method: Some(capture_method),
            confirm: Some(true),
            confirmation_method: None,
            currency: Currency::USD,
            customer: Some(customer_id),
            description: Some(description),
            error_on_requires_action: None,
            expand: &["latest_charge"],
            mandate: None,
            mandate_data: None,
            metadata: None,
            off_session: Some(PaymentIntentOffSession::Exists(true)),
            on_behalf_of: None,
            payment_method: Some(payment_method_id),
            payment_method_configuration: None,
            payment_method_data: None,
            payment_method_options: Some(CreatePaymentIntentPaymentMethodOptions{
                card: Some(CreatePaymentIntentPaymentMethodOptionsCard{
                    request_extended_authorization: Some(CreatePaymentIntentPaymentMethodOptionsCardRequestExtendedAuthorization::IfAvailable),
                    request_incremental_authorization: Some(CreatePaymentIntentPaymentMethodOptionsCardRequestIncrementalAuthorization::IfAvailable),
                    request_multicapture: Some(CreatePaymentIntentPaymentMethodOptionsCardRequestMulticapture::IfAvailable),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            payment_method_types: None,
            radar_options: None,
            receipt_email: None,
            return_url: None,
            setup_future_usage: None,
            shipping: None,
            statement_descriptor: None,
            statement_descriptor_suffix,
            transfer_data: None,
            transfer_group: None,
            use_stripe_sdk: None,
        }
    ).await
}

impl PaymentType {
    pub fn from_stripe_payment_intent_status(pis: PaymentIntentStatus) -> Self {
        match pis {
            PaymentIntentStatus::Canceled => PaymentType::Canceled,
            PaymentIntentStatus::Processing => PaymentType::Processing,
            PaymentIntentStatus::RequiresAction => PaymentType::RequiresAction,
            PaymentIntentStatus::RequiresCapture => PaymentType::RequiresCapture,
            PaymentIntentStatus::RequiresConfirmation => PaymentType::RequiresConfirmation,
            PaymentIntentStatus::RequiresPaymentMethod => PaymentType::RequiresPaymentMethod,
            PaymentIntentStatus::Succeeded => PaymentType::Succeeded,
        }
    }
}

pub async fn drop_auth(intent: &PaymentIntent) -> Result<PaymentIntent, StripeError> {
    dotenv().ok();
    let stripe_secret_key = env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
    let client = Client::new(stripe_secret_key);
    PaymentIntent::cancel(
        &client,
        &intent.id,
        CancelPaymentIntent {
            cancellation_reason: None,
        }
    ).await
}
