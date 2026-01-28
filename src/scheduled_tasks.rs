use crate::{POOL, integration, model, helper_model::VeygoError};
use chrono::{Datelike, NaiveTime, Utc};
use diesel::prelude::*;
use std::time::Duration;
use rust_decimal::prelude::*;
use stripe_core::{PaymentIntentCaptureMethod};

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

        println!("====== Running Daily Tasks ======");

        use diesel::dsl::sql;
        use crate::schema::renters::dsl as rt_q;

        let mut pool = POOL.get().unwrap();

        let user_needs_to_renew = rt_q::renters.filter(sql::<diesel::sql_types::Bool>("(
    (plan_renewal_day::int = EXTRACT(DAY FROM CURRENT_DATE)
    OR (
        EXTRACT(DAY FROM CURRENT_DATE) = EXTRACT(DAY FROM (date_trunc('MONTH', CURRENT_DATE + INTERVAL '1 MONTH') - INTERVAL '1 day'))
        AND plan_renewal_day::int > EXTRACT(DAY FROM (date_trunc('MONTH', CURRENT_DATE + INTERVAL '1 MONTH') - INTERVAL '1 day'))
    ))
    AND TO_CHAR(CURRENT_DATE, 'MMYYYY') = plan_expire_month_year
)")).load::<model::Renter>(&mut pool);

        let Ok(user_needs_to_renew) = user_needs_to_renew else {
            continue
        };

        if user_needs_to_renew.is_empty() {
            continue
        };

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
            use crate::schema::apartments::dsl as apt_q;
            let apartment = apt_q::apartments
                .find(&renter.apartment_id)
                .get_result::<model::Apartment>(&mut pool);

            let Ok(apartment) = apartment else {
                continue
            };

            if !apartment.is_operating {
                continue
            }

            let description;
            let mut rent = match renter.plan_tier {
                model::PlanTier::Platinum => {
                    if let Some(rent) = apartment.platinum_tier_rate {
                        description = String::from("PLAT TIER SUBS");
                        rent
                    } else if let Some(rent) = apartment.gold_tier_rate {
                        description = String::from("GOLD TIER SUBS");
                        renter.plan_tier = model::PlanTier::Gold;
                        rent
                    } else if let Some(rent) = apartment.silver_tier_hours {
                        description = String::from("SILVER TIER SUBS");
                        renter.plan_tier = model::PlanTier::Silver;
                        rent
                    } else {
                        renter.plan_tier = model::PlanTier::Free;
                        description = String::from("FREE TIER SUBS");
                        Decimal::zero()
                    }
                }
                model::PlanTier::Gold => {
                    if let Some(rent) = apartment.gold_tier_rate {
                        description = String::from("GOLD TIER SUBS");
                        rent
                    } else if let Some(rent) = apartment.silver_tier_hours {
                        description = String::from("SILVER TIER SUBS");
                        renter.plan_tier = model::PlanTier::Silver;
                        rent
                    } else {
                        renter.plan_tier = model::PlanTier::Free;
                        description = String::from("FREE TIER SUBS");
                        Decimal::zero()
                    }
                }
                model::PlanTier::Silver => {
                    if let Some(rent) = apartment.silver_tier_hours {
                        description = String::from("SILVER TIER SUBS");
                        rent
                    } else {
                        renter.plan_tier = model::PlanTier::Free;
                        description = String::from("FREE TIER SUBS");
                        Decimal::zero()
                    }
                }
                model::PlanTier::Free => {
                    description = String::from("FREE TIER SUBS");
                    Decimal::zero()
                }
            };
            if renter.is_plan_annual {
                rent = rent * Decimal::new(100, 1);
            }

            let renter_email = integration::sendgrid_veygo::make_email_obj(&renter.student_email, &renter.name);
            if rent != Decimal::zero() {
                if let Some(renew_id) = renter.subscription_payment_method_id &&
                    let Some(addr) = renter.billing_address.clone() {

                    // Get Payment Method
                    use crate::schema::payment_methods::dsl as pm_q;
                    let plan_pm = pm_q::payment_methods.find(renew_id).get_result::<model::PaymentMethod>(&mut pool);
                    let Ok(plan_pm) = plan_pm else {
                        continue
                    };

                    use crate::schema::apartments_taxes::dsl as at_q;
                    use crate::schema::taxes::dsl as t_q;

                    let vec_taxes = at_q::apartments_taxes
                        .inner_join(t_q::taxes)
                        .filter(at_q::apartment_id.eq(&renter.apartment_id))
                        .filter(t_q::tax_type.eq(model::TaxType::Percent))
                        .filter(t_q::is_sales_tax.eq(true))
                        .select(t_q::multiplier)
                        .get_results::<Decimal>(&mut pool);

                    let Ok(vec_taxes) = vec_taxes else {
                        continue
                    };

                    let mut sales_tax_rate = Decimal::zero();
                    for vec_tax in vec_taxes {
                        sales_tax_rate += vec_tax;
                    }

                    let taxed_rent = rent * (Decimal::one() + sales_tax_rate);
                    let taxed_rent_in_int = taxed_rent.round_dp(2).mantissa() as i64;

                    let payment_result = integration::stripe_veygo::create_payment_intent(
                        &renter.stripe_id, &plan_pm.token, taxed_rent_in_int, PaymentIntentCaptureMethod::Automatic, &description
                    ).await;

                    match payment_result {
                        Ok(_pi) => {
                            let new_plan_payment = model::NewSubscriptionPayment{
                                renter_id: renter.id,
                                payment_method_id: renew_id,
                                apartment_id: renter.apartment_id,
                                renter_name: renter.name.clone(),
                                renter_email: renter.student_email.clone(),
                                renter_phone: renter.phone.clone(),
                                renter_billing_address: addr,
                                time: Default::default(),
                                is_annual: renter.is_plan_annual,
                                amount: rent,
                                plan_tier: renter.plan_tier,
                                plan_renewal_day: Default::default(),
                            };

                            use crate::schema::subscription_payments::dsl as sp_q;
                            let result = diesel::insert_into(sp_q::subscription_payments)
                                .values(&new_plan_payment)
                                .get_result::<model::SubscriptionPayment>(&mut pool);

                            let Ok(_sp) = result else {
                                continue
                            };
                        }
                        Err(err) => {
                            match err {
                                VeygoError::CardDeclined => {
                                    // Downgrade plan
                                    renter.plan_tier = model::PlanTier::Free;

                                    // Downgrade email
                                    integration::sendgrid_veygo::send_email(None, renter_email, "You have been downgraded", "You have been downgraded to free plan due to payment method being declined. \nHowever, you are still welcome to upgrade to other plans anytime. ", None, None).await.unwrap();
                                }
                                _ => {
                                    continue
                                }
                            }
                        }
                    }
                } else {
                    // Downgrade plan
                    renter.plan_tier = model::PlanTier::Free;

                    // Downgrade email
                    integration::sendgrid_veygo::send_email(None, renter_email, "You have been downgraded", "You have been downgraded to free plan due to payment method being declined. \nHowever, you are still welcome to upgrade to other plans anytime. ", None, None).await.unwrap();
                }
            }

            // Update renter exp
            if renter.is_plan_annual {
                renter.plan_expire_month_year = renew_for_one_year.clone();
            } else {
                renter.plan_expire_month_year = renew_for_one_month.clone();
            }
            diesel::update(rt_q::renters.find(renter.id))
                .set(&renter).execute(&mut pool).unwrap();
        }

        let now = Utc::now();
        // Delete expired tokens
        use crate::schema::access_tokens::dsl as at_q;
        diesel::delete(
            at_q::access_tokens.filter(at_q::exp.lt(now))
        ).execute(&mut pool).unwrap();
        // Delete expired verifications
        use crate::schema::verifications::dsl as v_q;
        diesel::delete(
            v_q::verifications.filter(v_q::expires_at.lt(now))
        ).execute(&mut pool).unwrap();
        println!("===== Daily Tasks Completed =====");
    }
}
