use crate::model;
use std::env;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::OnceCell;
use stripe::{
    Client, CreateCustomer, CreatePaymentIntent, CreatePaymentIntentAutomaticPaymentMethods,
    CreatePaymentIntentAutomaticPaymentMethodsAllowRedirects, PaymentMethodId, SetupIntent,
    CreatePaymentIntentPaymentMethodOptions, CreatePaymentIntentPaymentMethodOptionsCard,
    CreatePaymentIntentPaymentMethodOptionsCardRequestExtendedAuthorization, PaymentMethod,
    CreatePaymentIntentPaymentMethodOptionsCardRequestIncrementalAuthorization, Currency,
    CreatePaymentIntentPaymentMethodOptionsCardRequestMulticapture, CreateSetupIntent,
    CreateSetupIntentAutomaticPaymentMethods, StripeError, CancelPaymentIntent, Customer,
    CreateSetupIntentAutomaticPaymentMethodsAllowRedirects, CustomerId, RefundReasonFilter,
    PaymentIntent, PaymentIntentCaptureMethod, PaymentIntentOffSession, CreateRefund, Refund,
    PaymentIntentId
};

static STRIPE_CLIENT: OnceCell<Arc<Client>> = OnceCell::const_new();

async fn stripe_client() -> Arc<Client> {
    STRIPE_CLIENT
        .get_or_init(|| async {
            let stripe_secret_key = env::var("STRIPE_SECRET_KEY")
                .expect("STRIPE_SECRET_KEY must be set");
            Arc::new(Client::new(stripe_secret_key))
        })
        .await
        .clone()
}

pub async fn create_new_payment_method(
    pm_id: &str,
    cardholder_name: &String, // Required as Stripe does not return the full name
    renter_id: &i32,          // Must be provided
    nickname: &Option<String>, // Optional user-defined alias
) -> Result<model::NewPaymentMethod, StripeError> {
    let client = stripe_client().await;
    let payment_id = PaymentMethodId::from_str(pm_id).unwrap();
    let payment_method = PaymentMethod::retrieve(&client, &payment_id, &[]).await;

    match payment_method {
        Ok(payment_method) => {
            let card = payment_method.card.unwrap();
            let accepted_cards: &[&str] = &[ "amex", "mastercard", "visa", "discover" ];
            if !accepted_cards.contains(&card.brand.as_str()) {
                return Err(StripeError::Stripe(Default::default()))
            }
            let mut masked_card_number = format!("**** **** **** {}", card.last4);
            if card.brand == "amex" {
                masked_card_number = format!("**** ****** *{}", card.last4);
            }
            let network = card.brand; // Visa, Mastercard, etc.
            let expiration = format!("{:02}/{}", card.exp_month, card.exp_year);

            Ok(model::NewPaymentMethod {
                cardholder_name: cardholder_name.to_string(),
                masked_card_number,
                network,
                expiration,
                token: pm_id.to_string(),
                fingerprint: card.fingerprint.unwrap(),
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
    let client = stripe_client().await;
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

#[allow(dead_code)]
pub async fn create_stripe_refund(
    customer_id: &CustomerId,
    payment_intent_id: &PaymentIntentId,
    amount: &i64,
) -> Result<Refund, StripeError> {
    let client = stripe_client().await;
    Refund::create(
        &client,
        CreateRefund {
            amount: Some(amount.clone()),
            charge: None,
            currency: Some(Currency::USD),
            customer: Some(customer_id.clone()),
            expand: &[],
            instructions_email: None,
            metadata: None,
            origin: None,
            payment_intent: Some(payment_intent_id.clone()),
            reason: Some(RefundReasonFilter::RequestedByCustomer),
            refund_application_fee: None,
            reverse_transfer: None,
        }
    ).await
}

pub async fn attach_payment_method_to_stripe_customer(
    stripe_customer_id: &String,
    pm_id: &String,
) -> Result<SetupIntent, StripeError> {
    let client = stripe_client().await;
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
    let client = stripe_client().await;
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

pub async fn drop_auth(intent_id: &PaymentIntentId) -> Result<PaymentIntent, StripeError> {
    let client = stripe_client().await;
    PaymentIntent::cancel(
        &client,
        &intent_id,
        CancelPaymentIntent {
            cancellation_reason: None,
        }
    ).await
}
