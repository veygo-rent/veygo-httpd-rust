use crate::{model, helper_model};
use std::env;
use tokio::sync::OnceCell;

use stripe::{Client, StripeError, ApiErrorsType, ApiErrorsCode};
use stripe_types::Currency;

use stripe_core::payment_intent::{
    CapturePaymentIntent, CancelPaymentIntent, CreatePaymentIntent, CreatePaymentIntentOffSession,
    CreatePaymentIntentPaymentMethodOptions, CreatePaymentIntentPaymentMethodOptionsCardRequestExtendedAuthorization,
    CreatePaymentIntentPaymentMethodOptionsCardRequestIncrementalAuthorization,
    CreatePaymentIntentPaymentMethodOptionsCardRequestMulticapture, CreatePaymentIntentPaymentMethodOptionsCard
};
use stripe_core::setup_intent::{
    CreateSetupIntent, CreateSetupIntentAutomaticPaymentMethods, CreateSetupIntentAutomaticPaymentMethodsAllowRedirects
};
use stripe_core::refund::{CreateRefund};
use stripe_core::customer::{CreateCustomer};

use stripe_core::{PaymentIntent, PaymentIntentCaptureMethod, SetupIntent, Refund, Customer, SetupIntentStatus};

use stripe_payment::payment_method::{RetrievePaymentMethod};

static STRIPE_CLIENT: OnceCell<Client> = OnceCell::const_new();

async fn stripe_client() -> &'static Client {
    STRIPE_CLIENT
        .get_or_init(|| async {
            let stripe_secret_key = env::var("STRIPE_SECRET_KEY")
                .expect("STRIPE_SECRET_KEY must be set");
            Client::new(stripe_secret_key)
        })
        .await
}

pub async fn retrieve_payment_method_from_stripe(
    pi_id: &str,
    cardholder_name: &String,   // Required as Stripe does not return the full name
    renter_id: &i32,            // Must be provided
    nickname: &Option<String>,  // Optional user-defined alias
) -> Result<model::NewPaymentMethod, helper_model::VeygoError> {
    let client = stripe_client().await;
    let payment_method = RetrievePaymentMethod::new(pi_id).send(client).await;
    match payment_method {
        Ok(payment_method) => {
            let card = payment_method.card.unwrap();
            let accepted_cards: &[&str] = &[ "amex", "mastercard", "visa", "discover" ];
            let accepted_funding_methods: &[&str] = &[ "credit", "debit" ];
            if !accepted_cards.contains(&card.brand.as_str()) &&
                !accepted_funding_methods.contains(&card.funding.as_str())
            {
                return Err(helper_model::VeygoError::CardNotSupported)
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
                token: pi_id.to_string(),
                fingerprint: card.fingerprint.unwrap(),
                nickname: nickname.clone(),
                is_enabled: true,
                renter_id: renter_id.clone(),
                last_used_date_time: None,
            })
        }
        Err(e) => {
            match e {
                StripeError::Stripe(err, _) => {
                    if err.type_ == ApiErrorsType::InvalidRequestError {
                        return Err(helper_model::VeygoError::InputDataError)
                    }
                    Err(helper_model::VeygoError::InternalServerError)
                }
                _ => {
                    Err(helper_model::VeygoError::InternalServerError)
                }
            }
        }
    }
}

pub async fn create_stripe_customer(
    name_data: &String,
    phone_data: &String,
    email_data: &String,
) -> Result<Customer, helper_model::VeygoError> {
    let client = stripe_client().await;
    let result = CreateCustomer::new()
        .name(name_data)
        .email(email_data)
        .phone(phone_data)
        .send(client)
        .await;

    match result {
        Ok(cus) => { Ok(cus) }
        Err(_) => { Err(helper_model::VeygoError::InternalServerError) }
    }
}

#[allow(dead_code)]
pub async fn create_stripe_refund(
    customer_id: &String,
    payment_intent_id: &String,
    amount: i64,
) -> Result<Refund, helper_model::VeygoError> {
    let client = stripe_client().await;
    let result = CreateRefund::new()
        .amount(amount)
        .customer(customer_id)
        .payment_intent(payment_intent_id)
        .currency(Currency::USD)
        .send(client)
        .await;

    match result {
        Ok(refund) => { Ok(refund) }
        Err(err) => {
            match err {
                StripeError::Stripe(stp_err, _) => {
                    if let Some(code) = stp_err.code &&
                        vec![
                            ApiErrorsCode::ChargeAlreadyRefunded,
                            ApiErrorsCode::ChargeDisputed,
                        ].contains(&code) {
                        return Err(helper_model::VeygoError::CanNotRefund)
                    } else if stp_err.type_ == ApiErrorsType::InvalidRequestError {
                        return Err(helper_model::VeygoError::CanNotRefund)
                    }
                    Err(helper_model::VeygoError::InternalServerError)
                }
                _ => {
                    Err(helper_model::VeygoError::InternalServerError)
                }
            }
        }
    }
}

pub async fn attach_payment_method_to_stripe_customer(
    stripe_customer_id: &String,
    pm_id: &String,
) -> Result<SetupIntent, helper_model::VeygoError> {
    let client = stripe_client().await;
    let result = CreateSetupIntent::new()
        .attach_to_self(false)
        .customer(stripe_customer_id)
        .payment_method(pm_id)
        .confirm(true)
        .automatic_payment_methods(CreateSetupIntentAutomaticPaymentMethods {
            allow_redirects: Some(CreateSetupIntentAutomaticPaymentMethodsAllowRedirects::Never),
            enabled: true
        })
        .send(client)
        .await;

    match result {
        Ok(si) => {
            if si.status == SetupIntentStatus::Succeeded {
                Ok(si)
            } else {
                Err(helper_model::VeygoError::CardDeclined)
            }
        }
        Err(e) => {
            match e {
                StripeError::Stripe(api_err, _) => {
                    let api_error = api_err.type_;
                    if api_error == ApiErrorsType::CardError {
                        return Err(helper_model::VeygoError::CardDeclined)
                    }
                    Err(helper_model::VeygoError::InternalServerError)
                }
                _ => {
                    Err(helper_model::VeygoError::InternalServerError)
                }
            }
        }
    }
}

pub async fn create_payment_intent(
    customer_id_data: &String,
    payment_id_data: &String,
    amount: i64,
    capture_method: PaymentIntentCaptureMethod,
    description: &String,
) -> Result<PaymentIntent, helper_model::VeygoError> {
    let client = stripe_client().await;
    let result = CreatePaymentIntent::new(amount, Currency::USD)
        .expand(vec![String::from("latest_charge")])
        .capture_method(capture_method)
        .confirm(true)
        .customer(customer_id_data)
        .payment_method(payment_id_data)
        .description(description)
        .off_session(CreatePaymentIntentOffSession::Bool(true))
        .payment_method_types(vec![String::from("card")])
        .payment_method_options(CreatePaymentIntentPaymentMethodOptions {
            card: Some(CreatePaymentIntentPaymentMethodOptionsCard {
                request_extended_authorization: Some(CreatePaymentIntentPaymentMethodOptionsCardRequestExtendedAuthorization::IfAvailable),
                request_incremental_authorization: Some(CreatePaymentIntentPaymentMethodOptionsCardRequestIncrementalAuthorization::IfAvailable),
                request_multicapture: Some(CreatePaymentIntentPaymentMethodOptionsCardRequestMulticapture::IfAvailable),
                ..Default::default()
            }),
            ..Default::default()
        })
        .send(client)
        .await;

    match result {
        Ok(pi) => {
            let pi_status: model::PaymentType = pi.status.clone().into();
            if vec![model::PaymentType::Succeeded, model::PaymentType::RequiresCapture].contains(&pi_status) {
                Ok(pi)
            } else {
                Err(helper_model::VeygoError::CardDeclined)
            }
        }
        Err(e) => {
            match e {
                StripeError::Stripe(api_err, _) => {
                    let api_error = api_err.type_;
                    if api_error == ApiErrorsType::CardError {
                        return Err(helper_model::VeygoError::CardDeclined)
                    }
                    Err(helper_model::VeygoError::InternalServerError)
                }
                _ => {
                    Err(helper_model::VeygoError::InternalServerError)
                }
            }
        }
    }
}

pub async fn drop_auth(intent_id: &str) -> Result<PaymentIntent, helper_model::VeygoError> {
    let client = stripe_client().await;
    let result = CancelPaymentIntent::new(intent_id).send(client).await;

    match result {
        Ok(result) => { Ok(result) }
        Err(e) => {
            match e {
                StripeError::Stripe(api_err, _) => {
                    let api_error = api_err.type_;
                    if api_error == ApiErrorsType::InvalidRequestError {
                        return Err(helper_model::VeygoError::CanNotRefund)
                    }
                    Err(helper_model::VeygoError::InternalServerError)
                }
                _ => {
                    Err(helper_model::VeygoError::InternalServerError)
                }
            }
        }
    }
}

pub async fn capture_payment(intent_id: &str, amount_final: Option<(i64, bool)>) -> Result<PaymentIntent, helper_model::VeygoError> {
    let client = stripe_client().await;
    let result = if let Some(amount_final) = amount_final {
        CapturePaymentIntent::new(intent_id)
            .amount_to_capture(amount_final.0)
            .final_capture(amount_final.1)
            .send(client)
            .await
    } else {
        CapturePaymentIntent::new(intent_id)
            .final_capture(true)
            .send(client)
            .await
    };

    match result {
        Ok(result) => { Ok(result) }
        Err(e) => {
            match e {
                StripeError::Stripe(api_err, _) => {
                    let api_error = api_err.type_;
                    if api_error == ApiErrorsType::InvalidRequestError {
                        return Err(helper_model::VeygoError::CanNotCapture)
                    }
                    Err(helper_model::VeygoError::InternalServerError)
                }
                _ => {
                    Err(helper_model::VeygoError::InternalServerError)
                }
            }
        }
    }
}
