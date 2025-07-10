use crate::POOL;
use crate::model::Renter;
use chrono::Utc;
use diesel::prelude::*;

pub async fn get_user_by_id(user_id: i32) -> QueryResult<Renter> {
    let mut pool = POOL.clone().get().unwrap();
    use crate::schema::renters::dsl::*;
    renters
        .filter(id.eq(&user_id))
        .get_result::<Renter>(&mut pool)
}

pub async fn check_if_on_do_not_rent(renter: &Renter) -> bool {
    let mut pool = POOL.clone().get().unwrap();
    // Extract values from Renter
    let user_name = renter.name.clone(); // String
    let user_phone = renter.phone.clone(); // String
    let user_email = renter.student_email.clone(); // String
    let today = Utc::now().date_naive(); // e.g. 2025-03-01

    // Run the database check on a blocking thread
    use crate::schema::do_not_rent_lists::dsl::*;
    diesel::select(diesel::dsl::exists(
        do_not_rent_lists
            .filter(
                name.eq(Some(user_name))
                    .or(phone.eq(Some(user_phone)))
                    .or(email.eq(Some(user_email))),
            )
            .filter(exp.is_null().or(exp.ge(today))),
    ))
    .get_result::<bool>(&mut pool)
    .unwrap()
}

pub fn user_is_admin(user: &Renter) -> bool {
    user.apartment_id == 1 && user.employee_tier == crate::model::EmployeeTier::Admin
}

pub fn user_is_operational_admin(user: &Renter) -> bool {
    if !user_is_admin(&user) {
        return false;
    }
    if let Some(email_expiration) = user.student_email_expiration {
        let today = Utc::now().date_naive();
        email_expiration > today
    } else {
        false
    }
}

pub fn user_is_manager(user: &Renter) -> bool {
    user_is_admin(&user) || user.employee_tier == crate::model::EmployeeTier::GeneralEmployee
}

pub fn user_is_operational_manager(user: &Renter) -> bool {
    if !user_is_manager(&user) {
        return false;
    }
    if let Some(email_expiration) = user.student_email_expiration {
        let today = Utc::now().date_naive();
        email_expiration > today
    } else {
        false
    }
}
