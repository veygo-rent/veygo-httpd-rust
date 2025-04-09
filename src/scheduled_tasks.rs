use crate::{integration, model, POOL};
use chrono::{Datelike, NaiveTime, Utc};
use diesel::prelude::*;
use std::time::Duration;
use stripe::{ErrorCode, StripeError};

pub async fn nightly_task() {
    loop {
        let now = Utc::now();
        let midnight = now
            .date_naive()
            .succ_opt()
            .unwrap()
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let duration_until_midnight = (midnight - now.naive_utc())
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(1));

        tokio::time::sleep(duration_until_midnight).await;

        // 👇 Catch panics inside the loop
        if let Err(e) = tokio::spawn(async move {
            println!("====== Running Daily Tasks ======");
            use diesel::dsl::sql;
            use crate::schema::renters::dsl::*;
            let user_needs_to_renew: Vec<model::Renter> = renters.filter(sql::<diesel::sql_types::Bool>("(
    (plan_renewal_day::int = EXTRACT(DAY FROM CURRENT_DATE)
    OR (
        EXTRACT(DAY FROM CURRENT_DATE) = EXTRACT(DAY FROM (date_trunc('MONTH', CURRENT_DATE + INTERVAL '1 MONTH') - INTERVAL '1 day'))
        AND plan_renewal_day::int > EXTRACT(DAY FROM (date_trunc('MONTH', CURRENT_DATE + INTERVAL '1 MONTH') - INTERVAL '1 day'))
    ))
    AND TO_CHAR(CURRENT_DATE, 'MMYYYY') = plan_expire_month_year
)")).load::<model::Renter>(&mut POOL.clone().get().unwrap()).unwrap();
            let today = Utc::now();
            let mut year = today.year();
            let mut month = today.month();

            let renew_for_one_year = format!("{:02}{}", month, year + 1);

            if month == 12 {
                month = 1;
                year += 1;
            } else {
                month += 1;
            }
            let renew_for_one_month = format!("{:02}{}", month, year);

            for mut renter in user_needs_to_renew {
                use crate::schema::apartments::dsl::*;
                let apartment: model::Apartment = apartments.filter(id.eq(renter.apartment_id)).get_result::<model::Apartment>(&mut POOL.clone().get().unwrap()).unwrap();
                if !apartment.is_operating {
                    break;
                }
                let mut description = String::from("Veygo ");
                let mut rent = match renter.plan_tier {
                    model::PlanTier::Free => { 0.0 }
                    model::PlanTier::Silver => {
                        description = description + "Silver Tier Subscription";
                        apartment.silver_tier_rate
                    }
                    model::PlanTier::Gold => {
                        description = description + "Gold Tier Subscription";
                        apartment.gold_tier_rate
                    }
                    model::PlanTier::Platinum => {
                        description = description + "Platinum Tier Subscription";
                        apartment.platinum_tier_rate
                    }
                };
                if renter.is_plan_annual {
                    rent = rent * 10.0;
                }
                let sender = integration::sendgrid_veygo::make_email_obj("no-reply@veygo.rent", Option::from("Veygo No-Reply"));
                let renter_email = integration::sendgrid_veygo::make_email_obj(&renter.student_email, Option::from(renter.name.as_str()));
                if rent != 0.0 {
                    // Get Payment Method
                    use crate::schema::payment_methods::dsl::*;
                    let plan_pm: model::PaymentMethod = payment_methods.filter(id.eq(renter.subscription_payment_method_id.unwrap())).get_result::<model::PaymentMethod>(&mut POOL.clone().get().unwrap()).unwrap();
                    // Charge Renter. If fails, switch to Free Tier
                    let taxed_rent = rent * (1.00 + apartment.sales_tax_rate);
                    let taxed_rent_in_int = (taxed_rent * 100.0).round() as i64;
                    use stripe::PaymentIntentCaptureMethod;
                    let payment_result = integration::stripe_veygo::create_payment_intent(description.clone(), renter.clone().stripe_id.unwrap(), plan_pm.token, taxed_rent_in_int, PaymentIntentCaptureMethod::Automatic).await;
                    match payment_result {
                        Err(error) => {
                            match error {
                                StripeError::Stripe(request_error) => {
                                    if request_error.code == Some(ErrorCode::CardDeclined) {
                                        // Downgrade plan
                                        renter.plan_tier = model::PlanTier::Free;

                                        // Downgrade email
                                        integration::sendgrid_veygo::send_email(sender, renter_email, "You have been downgraded", "You have been downgraded to free plan due to payment method being declined. \nHowever, you are still welcome to upgrade to other plans anytime. ", None, None).await.unwrap();
                                    }
                                }
                                StripeError::QueryStringSerialize(ser_err) => {
                                    eprintln!("Query string serialization error: {:?}", ser_err);
                                }
                                StripeError::JSONSerialize(json_err) => {
                                    eprintln!("JSON serialization error: {}", json_err.to_string());
                                }
                                StripeError::UnsupportedVersion => {
                                    eprintln!("Unsupported Stripe API version");
                                }
                                StripeError::ClientError(msg) => {
                                    eprintln!("Client error: {}", msg);
                                }
                                StripeError::Timeout => {
                                    eprintln!("Stripe request timed out");
                                }
                            }
                        }
                        Ok(pmi) => {
                            // Approved
                            // Save Payment
                            let new_payment = model::NewPayment {
                                payment_type: model::PaymentType::Succeeded,
                                amount: taxed_rent,
                                note: Some(description),
                                reference_number: Some(pmi.id.to_string()),
                                agreement_id: None,
                                renter_id: renter.id,
                                payment_method_id: plan_pm.id,
                            };
                            use crate::schema::payments::dsl::*;
                            diesel::insert_into(payments).values(&new_payment).get_result::<model::Payment>(&mut POOL.clone().get().unwrap()).unwrap();
                            // Paid Tier renewal email
                            integration::sendgrid_veygo::send_email(sender, renter_email, "Your plan has been renewed", "Your payment has been processed and your plan has been renewed. ", None, None).await.unwrap();
                        }
                    }
                } else {
                    // Free Tier renewal email
                    integration::sendgrid_veygo::send_email(sender, renter_email, "Your plan has been renewed", "Your plan has been renewed. \nEnjoy your free plan! ", None, None).await.unwrap();
                }
                // Update renter exp
                if renter.is_plan_annual {
                    renter.plan_expire_month_year = renew_for_one_year.clone();
                } else {
                    renter.plan_expire_month_year = renew_for_one_month.clone();
                }
                diesel::update(renters.find(renter.id))
                    .set(&renter).execute(&mut POOL.clone().get().unwrap()).unwrap();
            }
            println!("===== Daily Tasks Completed =====");
        })
            .await
            .map_err(|e| format!("Task panicked: {e}"))
        {
            integration::sendgrid_veygo::send_email(
                integration::sendgrid_veygo::make_email_obj("no-reply@veygo.rent", Option::from("Veygo Server")),
                integration::sendgrid_veygo::make_email_obj("dev@veygo.rent", Option::from("Veygo Dev Team")),
                "Midnight daily task has failed",
                e.as_str(),
                None,
                None,
            )
                .await
                .unwrap();
        }
    }
}
