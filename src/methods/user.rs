use std::option::Option;
use crate::POOL;
use crate::model;
use crate::helper_model::VeygoError;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use diesel::prelude::*;
use diesel::result::Error;

impl model::Renter {
    pub fn plan_renewal_date (&self) -> Result<NaiveDate, VeygoError> {
        user_plan_renewal_date(self)
    }
    
    pub fn get_university_apartment (&self) -> Result<(model::Apartment, Option<model::Apartment>), VeygoError> {
        get_university_apartment_by_renter(self)
    }
    
    pub fn get_dnr_records (&self) -> Result<Vec<model::DoNotRentList>, VeygoError> {
        get_dnr_records_for(self)
    }
    
    pub fn is_admin (&self) -> bool {
        user_is_admin(self)
    }
    
    pub fn is_operational_admin (&self) -> bool {
        user_is_operational_admin(self)
    }
    
    pub fn is_manager (&self) -> bool {
        user_is_manager(self)
    }
    
    pub fn is_operational_manager (&self) -> bool {
        user_is_operational_manager(self)
    }

    pub fn get_dnr_count (&self) -> Result<i64, VeygoError> {
        let mut pool = POOL.get().unwrap();

        let today = Utc::now().date_naive();

        // Run the database check on a blocking thread
        use crate::schema::do_not_rent_lists::dsl::*;

        let result = do_not_rent_lists
            .filter(
                name.eq(Some(&self.name))
                    .or(phone.eq(Some(&self.phone)))
                    .or(email.eq(Some(&self.student_email)))
            )
            .filter(exp.is_null().or(exp.lt(today)))
            .count()
            .first::<i64>(&mut pool);

        match result {
            Ok(count) => {
                Ok(count)
            }
            Err(_) => {
                Err(VeygoError::InternalServerError)
            }
        }
    }
}

pub async fn get_user_by_id(user_id: &i32) -> Result<model::Renter, VeygoError> {
    // Will VeygoError::RecordNotFound or VeygoError::InternalServerError
    
    let mut pool = POOL.get().unwrap();
    use crate::schema::renters::dsl::*;
    let result = renters
        .find(&user_id)
        .get_result::<model::Renter>(&mut pool);
    match result {
        Ok(user) => Ok(user),
        Err(e) => { 
            match e {
                Error::NotFound => {
                    Err(VeygoError::RecordNotFound)
                }
                _ => { 
                    Err(VeygoError::InternalServerError)
                }
            }
        }
    }
}

fn get_dnr_records_for(renter: &model::Renter) -> Result<Vec<model::DoNotRentList>, VeygoError> {
    let mut pool = POOL.get().unwrap();

    let today = Utc::now().date_naive();

    // Run the database check on a blocking thread
    use crate::schema::do_not_rent_lists::dsl::*;

    let result = do_not_rent_lists
        .filter(
            name.eq(Some(&renter.name))
                .or(phone.eq(Some(&renter.phone)))
                .or(email.eq(Some(&renter.student_email)))
        )
        .filter(exp.is_null().or(exp.lt(today)))
        .order(id.asc())
        .get_results::<model::DoNotRentList>(&mut pool);

    match result {
        Ok(list) => Ok(list),
        Err(e) => { 
            match e {
                _ => { 
                    Err(VeygoError::InternalServerError)
                }
            }
        }
    }
}

fn get_university_apartment_by_renter(renter: &model::Renter) -> Result<(model::Apartment, Option<model::Apartment>), VeygoError> {
    let mut pool = POOL.get().unwrap();
    use crate::schema::apartments::dsl::*;
    let immediate_record = apartments.find(&renter.apartment_id).get_result::<model::Apartment>(&mut pool);
    if immediate_record.is_err() {
        let e = immediate_record.err().unwrap();
        return match e {
            Error::NotFound => {
                Err(VeygoError::RecordNotFound)
            }
            _ => {
                Err(VeygoError::InternalServerError)
            }
        }
    }
    let immediate_record = immediate_record.unwrap();
    if immediate_record.uni_id == 1 {
        Ok((immediate_record, None))
    } else {
        let univ = apartments.find(&immediate_record.uni_id).get_result::<model::Apartment>(&mut pool);
        if univ.is_err() {
            let e = univ.err().unwrap();
            return match e {
                Error::NotFound => {
                    Err(VeygoError::RecordNotFound)
                }
                _ => {
                    Err(VeygoError::InternalServerError)
                }
            }
        }
        let univ = univ.unwrap();
        Ok((univ, Some(immediate_record)))
    }
}

fn user_is_admin(user: &model::Renter) -> bool {
    user.apartment_id == 1 && user.employee_tier == model::EmployeeTier::Admin
}

fn user_is_operational_admin(user: &model::Renter) -> bool {
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

fn user_is_manager(user: &model::Renter) -> bool {
    user_is_admin(&user) || user.employee_tier == model::EmployeeTier::GeneralEmployee
}

fn user_is_operational_manager(user: &model::Renter) -> bool {
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

fn user_plan_renewal_date(
    user: &model::Renter,
) -> Result<NaiveDate, VeygoError> {
    // Expect MMYYYY
    if user.plan_expire_month_year.len() != 6 {
        return Err(VeygoError::InternalServerError);
    }

    // Expect DD
    if user.plan_renewal_day.len() != 2 {
        return Err(VeygoError::InternalServerError);
    }

    let month: u32 = user.plan_expire_month_year[0..2]
        .parse()
        .map_err(|_| VeygoError::InternalServerError)?;
    if !(1..=12).contains(&month) {
        return Err(VeygoError::InternalServerError);
    }

    let year: i32 = user.plan_expire_month_year[2..6]
        .parse()
        .map_err(|_| VeygoError::InternalServerError)?;

    let requested_day: u32 = user
        .plan_renewal_day
        .parse()
        .map_err(|_| VeygoError::InternalServerError)?;
    if !(1..=31).contains(&requested_day) {
        return Err(VeygoError::InternalServerError);
    }

    // Compute the last day of the month by taking the first day of the next month and subtracting 1 day.
    let first_of_next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or(VeygoError::InternalServerError)?
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).ok_or(VeygoError::InternalServerError)?
    };
    let last_day_of_month = (first_of_next_month - Duration::days(1)).day();

    let day = requested_day.min(last_day_of_month);

    NaiveDate::from_ymd_opt(year, month, day).ok_or(VeygoError::InternalServerError)
}
