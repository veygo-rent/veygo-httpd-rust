use std::option::Option;
use crate::POOL;
use crate::model::Renter;
use chrono::Utc;
use diesel::prelude::*;

pub async fn get_user_by_id(user_id: &i32) -> QueryResult<Renter> {
    let mut pool = POOL.get().unwrap();
    use crate::schema::renters::dsl::*;
    renters
        .filter(id.eq(&user_id))
        .get_result::<Renter>(&mut pool)
}

pub fn get_dnr_record_for(renter: &Renter) -> Option<Vec<crate::model::DoNotRentList>> {
    let mut pool = POOL.get().unwrap();

    let today = Utc::now().date_naive(); // e.g. 2025-03-01

    // Run the database check on a blocking thread
    use crate::schema::do_not_rent_lists::dsl::*;

    let result = do_not_rent_lists
        .filter(
            name.eq(Some(&renter.name))
                .or(phone.eq(Some(&renter.phone)))
                .or(email.eq(Some(&renter.student_email)))
        )
        .filter(exp.is_null().or(exp.lt(today)))
        .get_results::<crate::model::DoNotRentList>(&mut pool);

    match result {
        Ok(list) => Some(list),
        Err(_) => None
    }
}

#[allow(dead_code)]
pub fn get_university_apartment_by_renter(renter: &Renter) -> (crate::model::Apartment, Option<crate::model::Apartment>) {
    let mut pool = POOL.get().unwrap();
    use crate::schema::apartments::dsl::*;
    let immediate_record = apartments.filter(id.eq(&renter.apartment_id)).get_result::<crate::model::Apartment>(&mut pool).unwrap();
    if immediate_record.uni_id == 1 {
        (immediate_record, None)
    } else {
        let univ = apartments.filter(id.eq(&immediate_record.uni_id)).get_result::<crate::model::Apartment>(&mut pool).unwrap();
        (univ, Some(immediate_record))
    }
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
