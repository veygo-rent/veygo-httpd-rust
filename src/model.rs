use diesel::prelude::*;
use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};

// Diesel requires us to define a custom mapping between the Rust enum
// and the database type, if we are not using string.
use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, ToSql, Output};
use std::io::Write;
use diesel::{AsExpression, FromSqlRow};
use crate::schema::*;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)] //lets us map the enum to TEXT in PostgresSQL
pub enum AgreementStatus {
    Rental,
    Void,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum EmployeeTier {
    User,
    GeneralEmployee,
    Maintenance,
    Admin,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum PaymentType {
    Cash,
    ACH,
    CC,
    BadDebt,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum PlanTier {
    Free,
    Silver,
    Gold,
    Platinum,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Text)]
pub enum Gender {
    Male,
    Female,
    Other,
    PNTS, // prefer not to say
}

//This is for postgres. For other databases the type might be different.
impl<DB> ToSql<diesel::sql_types::Text, DB> for AgreementStatus
where
    DB: Backend,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match *self {
            AgreementStatus::Rental => out.write_all(b"Rental")?,
            AgreementStatus::Void => out.write_all(b"Void")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl<DB> FromSql<diesel::sql_types::Text, DB> for AgreementStatus
where
    DB: Backend,
    String: FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<diesel::sql_types::Text, DB>>::from_sql(bytes)?;
        match value.as_str() {
            "Rental" => Ok(AgreementStatus::Rental),
            "Void" => Ok(AgreementStatus::Void),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
// The following is the traits implementation for other Enums.
impl<DB> ToSql<diesel::sql_types::Text, DB> for EmployeeTier
where
    DB: Backend,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match *self {
            EmployeeTier::User => out.write_all(b"User")?,
            EmployeeTier::GeneralEmployee => out.write_all(b"GeneralEmployee")?,
            EmployeeTier::Maintenance => out.write_all(b"Maintenance")?,
            EmployeeTier::Admin => out.write_all(b"Admin")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl<DB> FromSql<diesel::sql_types::Text, DB> for EmployeeTier
where
    DB: Backend,
    String: FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<diesel::sql_types::Text, DB>>::from_sql(bytes)?;
        match value.as_str() {
            "User" => Ok(EmployeeTier::User),
            "GeneralEmployee" => Ok(EmployeeTier::GeneralEmployee),
            "Maintenance" => Ok(EmployeeTier::Maintenance),
            "Admin" => Ok(EmployeeTier::Admin),

            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
impl<DB> ToSql<diesel::sql_types::Text, DB> for PaymentType
where
    DB: Backend,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match *self {
            PaymentType::Cash => out.write_all(b"Cash")?,
            PaymentType::ACH => out.write_all(b"ACH")?,
            PaymentType::CC => out.write_all(b"CC")?,
            PaymentType::BadDebt => out.write_all(b"BadDebt")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl<DB> FromSql<diesel::sql_types::Text, DB> for PaymentType
where
    DB: Backend,
    String: FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<diesel::sql_types::Text, DB>>::from_sql(bytes)?;
        match value.as_str() {
            "Cash" => Ok(PaymentType::Cash),
            "ACH" => Ok(PaymentType::ACH),
            "CC" => Ok(PaymentType::CC),
            "BadDebt" => Ok(PaymentType::BadDebt),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
impl<DB> ToSql<diesel::sql_types::Text, DB> for PlanTier
where
    DB: Backend,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match *self {
            PlanTier::Free => out.write_all(b"Free")?,
            PlanTier::Silver => out.write_all(b"Silver")?,
            PlanTier::Gold => out.write_all(b"Gold")?,
            PlanTier::Platinum => out.write_all(b"Platinum")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl<DB> FromSql<diesel::sql_types::Text, DB> for PlanTier
where
    DB: Backend,
    String: FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<diesel::sql_types::Text, DB>>::from_sql(bytes)?;
        match value.as_str() {
            "Free" => Ok(PlanTier::Free),
            "Silver" => Ok(PlanTier::Silver),
            "Gold" => Ok(PlanTier::Gold),
            "Platinum" => Ok(PlanTier::Platinum),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
impl<DB> ToSql<diesel::sql_types::Text, DB> for Gender
where
    DB: Backend,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, DB>) -> serialize::Result {
        match *self {
            Gender::Male => out.write_all(b"Male")?,
            Gender::Female => out.write_all(b"Female")?,
            Gender::Other => out.write_all(b"Other")?,
            Gender::PNTS => out.write_all(b"PNTS")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl<DB> FromSql<diesel::sql_types::Text, DB> for Gender
where
    DB: Backend,
    String: FromSql<diesel::sql_types::Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let value = <String as FromSql<diesel::sql_types::Text, DB>>::from_sql(bytes)?;
        match value.as_str() {
            "Male" => Ok(Gender::Male),
            "Female" => Ok(Gender::Female),
            "Other" => Ok(Gender::Other),
            "PNTS" => Ok(Gender::PNTS),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = renters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Renter {
    pub id: i32,
    pub name: String,
    pub student_email: String,
    pub student_email_expiration: Option<NaiveDate>,
    pub password: String, // Hashed!
    pub phone: String,
    pub phone_is_verified: bool,
    pub date_of_birth: NaiveDate,
    pub profile_picture: Option<String>,
    pub gender: Option<Gender>,
    pub date_of_registration: DateTime<Utc>,
    pub drivers_license_number: Option<String>,
    pub drivers_license_state_region: Option<String>,
    pub drivers_license_image: Option<String>,
    pub drivers_license_image_secondary: Option<String>,
    pub drivers_license_expiration: Option<NaiveDate>,
    pub insurance_id_image: Option<String>,
    pub insurance_id_expiration: Option<NaiveDate>,
    pub lease_agreement_image: Option<String>,
    pub apartment_id: i32,
    pub lease_agreement_expiration: Option<NaiveDate>,
    pub billing_address: Option<String>,
    pub signature_image: Option<String>,
    pub signature_datetime: Option<DateTime<Utc>>,
    pub plan_tier: PlanTier,
    pub plan_renewal_day: String,
    pub plan_expire_month_year: String,
    pub plan_available_duration: f64,
    pub is_plan_annual: bool,
    pub employee_tier: EmployeeTier,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = renters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRenter<'a> {
    pub name: &'a str,
    pub student_email: &'a str,
    pub student_email_expiration: Option<NaiveDate>,
    pub password: &'a str, // Hash this before inserting!
    pub phone: &'a str,
    pub phone_is_verified: bool,
    pub date_of_birth: NaiveDate,
    pub profile_picture: Option<&'a str>,
    pub gender: Option<Gender>,
    pub date_of_registration: DateTime<Utc>,
    pub drivers_license_number: Option<&'a str>,
    pub drivers_license_state_region: Option<&'a str>,
    pub drivers_license_image: Option<&'a str>,
    pub drivers_license_image_secondary: Option<&'a str>,
    pub drivers_license_expiration: Option<NaiveDate>,
    pub insurance_id_image: Option<&'a str>,
    pub insurance_id_expiration: Option<NaiveDate>,
    pub lease_agreement_image: Option<&'a str>,
    pub apartment_id: i32,
    pub lease_agreement_expiration: Option<NaiveDate>,
    pub billing_address: Option<&'a str>,
    pub signature_image: Option<&'a str>,
    pub signature_datetime: Option<DateTime<Utc>>,
    pub plan_tier: PlanTier,
    pub plan_renewal_day: &'a str,
    pub plan_expire_month_year: &'a str,
    pub plan_available_duration: f64,
    pub is_plan_annual: bool,
    pub employee_tier: EmployeeTier,
}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = payment_methods)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PaymentMethod {
    pub id: i32,
    pub cardholder_name: String,
    pub masked_card_number: String,
    pub network: String,
    pub expiration: String,
    pub token: String,
    pub nickname: Option<String>,
    pub is_enabled: bool,
    pub renter_id: i32,
    pub last_used_date_time: Option<DateTime<Utc>>,
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = payment_methods)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct  NewPaymentMethod<'a> {
    pub cardholder_name: &'a str,
    pub masked_card_number: &'a str,
    pub network: &'a str,
    pub expiration: &'a str,
    pub token: &'a str,
    pub nickname: Option<&'a str>,
    pub is_enabled: bool,
    pub renter_id: i32,
    pub last_used_date_time: Option<DateTime<Utc>>,
}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = apartments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Apartment {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub accepted_school_email_domain: String,
    pub free_tier_hours: f64,
    pub free_tier_rate: f64,
    pub silver_tier_hours: f64,
    pub silver_tier_rate: f64,
    pub gold_tier_hours: f64,
    pub gold_tier_rate: f64,
    pub platinum_tier_hours: f64,
    pub platinum_tier_rate: f64,
    pub duration_rate: f64,
    pub liability_protection_rate: f64,
    pub pcdw_protection_rate: f64,
    pub pcdw_ext_protection_rate: f64,
    pub rsa_protection_rate: f64,
    pub pai_protection_rate: f64,
    pub sales_tax_rate: f64,
    pub is_operating: bool,
    pub is_public: bool,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = apartments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewApartment<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub phone: &'a str,
    pub address: &'a str,
    pub accepted_school_email_domain: &'a str,
    pub free_tier_hours: f64,
    pub free_tier_rate: f64,
    pub silver_tier_hours: f64,
    pub silver_tier_rate: f64,
    pub gold_tier_hours: f64,
    pub gold_tier_rate: f64,
    pub platinum_tier_hours: f64,
    pub platinum_tier_rate: f64,
    pub duration_rate: f64,
    pub liability_protection_rate: f64,
    pub pcdw_protection_rate: f64,
    pub pcdw_ext_protection_rate: f64,
    pub rsa_protection_rate: f64,
    pub pai_protection_rate: f64,
    pub sales_tax_rate: f64,
    pub is_operating: bool,
    pub is_public: bool,

}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[diesel(table_name = transponder_companies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TransponderCompany {
    pub id: i32,
    pub name: String,
    pub corresponding_key_for_vehicle_id: String,
    pub corresponding_key_for_transaction_name: String,
    pub custom_prefix_for_transaction_name: String,
    pub corresponding_key_for_transaction_time: String,
    pub corresponding_key_for_transaction_amount: String,
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = transponder_companies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct  NewTransponderCompany<'a> {
    pub name: &'a str,
    pub corresponding_key_for_vehicle_id: &'a str,
    pub corresponding_key_for_transaction_name: &'a str,
    pub custom_prefix_for_transaction_name: &'a str,
    pub corresponding_key_for_transaction_time: &'a str,
    pub corresponding_key_for_transaction_amount: &'a str,
}

#[derive(Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = vehicles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Vehicle {
    pub id: i32,
    pub vin: String,
    pub name: String,
    pub available: bool,
    pub license_number: String,
    pub license_state: String,
    pub year: String,
    pub make: String,
    pub model: String,
    pub msrp_factor: f64,
    pub image_link: Option<String>,
    pub odometer: i32,
    pub tank_size: f64,
    pub tank_level_percentage: i32,
    pub first_transponder_number: Option<String>,
    pub first_transponder_company_id: Option<i32>,
    pub second_transponder_number: Option<String>,
    pub second_transponder_company_id: Option<i32>,
    pub third_transponder_number: Option<String>,
    pub third_transponder_company_id: Option<i32>,
    pub fourth_transponder_number: Option<String>,
    pub fourth_transponder_company_id: Option<i32>,
    pub apartment_id: i32,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = vehicles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewVehicle<'a> {
    pub vin: &'a str,
    pub name: &'a str,
    pub available: bool,
    pub license_number: &'a str,
    pub license_state: &'a str,
    pub year: &'a str,
    pub make: &'a str,
    pub model: &'a str,
    pub msrp_factor: f64,
    pub image_link: Option<&'a str>,
    pub odometer: i32,
    pub tank_size: f64,
    pub tank_level_percentage: i32,
    pub first_transponder_number: Option<&'a str>,
    pub first_transponder_company_id: Option<i32>,
    pub second_transponder_number: Option<&'a str>,
    pub second_transponder_company_id: Option<i32>,
    pub third_transponder_number: Option<&'a str>,
    pub third_transponder_company_id: Option<i32>,
    pub fourth_transponder_number: Option<&'a str>,
    pub fourth_transponder_company_id: Option<i32>,
    pub apartment_id: i32,
}

#[derive(Queryable, Identifiable, Associations,Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = damages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Damage {
    pub id: i32,
    pub note: String,
    pub record_date: DateTime<Utc>,
    pub occur_date: DateTime<Utc>,
    pub standard_coordination_x_percentage: i32,
    pub standard_coordination_y_percentage: i32,
    pub first_image: Option<String>,
    pub second_image: Option<String>,
    pub third_image: Option<String>,
    pub fourth_image: Option<String>,
    pub fixed_date: Option<DateTime<Utc>>,
    pub fixed_amount: Option<f64>,
    pub agreement_id: Option<i32>,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = damages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDamage<'a> {
    pub note: &'a str,
    pub record_date: DateTime<Utc>,
    pub occur_date: DateTime<Utc>,
    pub standard_coordination_x_percentage: i32,
    pub standard_coordination_y_percentage: i32,
    pub first_image: Option<&'a str>,
    pub second_image: Option<&'a str>,
    pub third_image: Option<&'a str>,
    pub fourth_image: Option<&'a str>,
    pub fixed_date: Option<DateTime<Utc>>,
    pub fixed_amount: Option<f64>,
    pub agreement_id: Option<i32>,
}

#[derive(Queryable, Identifiable, Associations,Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Apartment))]
#[diesel(belongs_to(Vehicle))]
#[diesel(belongs_to(Renter))]
#[diesel(belongs_to(PaymentMethod))]
#[diesel(table_name = agreements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Agreement {
    pub id: i32,
    pub confirmation: String,
    pub status: AgreementStatus,
    pub user_name: String,
    pub user_date_of_birth: NaiveDate,
    pub user_email: String,
    pub user_phone: String,
    pub user_billing_address: String,
    pub rsvp_pickup_time: DateTime<Utc>,
    pub rsvp_drop_off_time: DateTime<Utc>,
    pub liability_protection_rate: f64,
    pub pcdw_protection_rate: f64,
    pub pcdw_ext_protection_rate: f64,
    pub rsa_protection_rate: f64,
    pub pai_protection_rate: f64,
    pub actual_pickup_time: Option<DateTime<Utc>>,
    pub pickup_odometer: Option<i32>,
    pub pickup_level: Option<i32>,
    pub actual_drop_off_time: Option<DateTime<Utc>>,
    pub drop_off_odometer: Option<i32>,
    pub drop_off_level: Option<i32>,
    pub tax_rate: f64,
    pub msrp_factor: f64,
    pub plan_duration: f64,
    pub pay_as_you_go_duration: f64,
    pub duration_rate: f64,
    pub apartment_id: i32,
    pub vehicle_id: i32,
    pub renter_id: i32,
    pub payment_method_id: i32,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Apartment))]
#[diesel(belongs_to(Vehicle))]
#[diesel(belongs_to(Renter))]
#[diesel(belongs_to(PaymentMethod))]
#[diesel(table_name = agreements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAgreement<'a> {
    pub confirmation: &'a str,
    pub status: AgreementStatus,
    pub user_name: &'a str,
    pub user_date_of_birth: NaiveDate,
    pub user_email: &'a str,
    pub user_phone: &'a str,
    pub user_billing_address: &'a str,
    pub rsvp_pickup_time: DateTime<Utc>,
    pub rsvp_drop_off_time: DateTime<Utc>,
    pub liability_protection_rate: f64,
    pub pcdw_protection_rate: f64,
    pub pcdw_ext_protection_rate: f64,
    pub rsa_protection_rate: f64,
    pub pai_protection_rate: f64,
    pub actual_pickup_time: Option<DateTime<Utc>>,
    pub pickup_odometer: Option<i32>,
    pub pickup_level: Option<i32>,
    pub actual_drop_off_time: Option<DateTime<Utc>>,
    pub drop_off_odometer: Option<i32>,
    pub drop_off_level: Option<i32>,
    pub tax_rate: f64,
    pub msrp_factor: f64,
    pub plan_duration: f64,
    pub pay_as_you_go_duration: f64,
    pub duration_rate: f64,
    pub apartment_id: i32,
    pub vehicle_id: i32,
    pub renter_id: i32,
    pub payment_method_id: i32,
}

#[derive(Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = charges)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Charge {
    pub id: i32,
    pub name: String,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub agreement_id: i32,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = charges)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct  NewCharge<'a> {
    pub name: &'a str,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<&'a str>,
    pub agreement_id: i32,
}

#[derive(Queryable, Identifiable, Associations,Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Agreement))]
#[diesel(belongs_to(PaymentMethod))]
#[diesel(table_name = payments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Payment {
    pub id: i32,
    pub r#type: PaymentType,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub reference_number: Option<String>,
    pub agreement_id: i32,
    pub payment_method_id: Option<i32>,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Agreement))]
#[diesel(belongs_to(PaymentMethod))]
#[diesel(table_name = payments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct  NewPayment {
    pub r#type: PaymentType,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub reference_number: Option<String>,
    pub agreement_id: i32,
    pub payment_method_id: Option<i32>,
}

#[derive(Queryable, Identifiable,Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = access_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AccessToken {
    pub id: i32,
    pub user_id: i32,
    pub token: String,
    pub exp: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = access_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct  NewAccessToken<'a> {
    pub user_id: i32,
    pub token: &'a str,
    pub exp: DateTime<Utc>,
}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[diesel(table_name = do_not_rent_lists)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DoNotRentList {
    pub id: i32,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub note: String,
    pub exp: Option<NaiveDate>,
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = do_not_rent_lists)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDoNotRentList<'a> {
    pub name: Option<&'a str>,
    pub phone: Option<&'a str>,
    pub email: Option<&'a str>,
    pub note: &'a str,
    pub exp: Option<NaiveDate>,
}