use crate::integration::sendgrid_veygo::*;
use crate::{model, POOL};
use chrono::{NaiveTime, Utc};
use diesel::prelude::*;
use std::time::Duration;

pub async fn run_every_midnight() {
    loop {
        let now = Utc::now();
        let midnight = now
            .date_naive()
            .succ_opt()
            .unwrap()
            .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        let duration_until_midnight = (midnight - now.naive_local())
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
            println!("===== Daily Tasks Completed =====");
        })
        .await
        .map_err(|e| format!("Task panicked: {e}"))
        {
            send_email(
                make_email_obj("no-reply@veygo.rent", Option::from("Veygo Server")),
                make_email_obj("dev@veygo.rent", Option::from("Veygo Dev Team")),
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
