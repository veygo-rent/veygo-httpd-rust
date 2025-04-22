use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use warp::http::header::{HeaderMap, HeaderValue};

// Diesel requires us to define a custom mapping between the Rust enum
// and the database type, if we are not using string.
use crate::schema::*;
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, Output, ToSql};
use diesel::{AsExpression, FromSqlRow};
use std::io::Write;

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::VerificationTypeEnum)]
pub enum VerificationType {
    Email,
    Phone,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::AgreementStatusEnum)]
pub enum AgreementStatus {
    Rental,
    Void,
    Canceled,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::EmployeeTierEnum)]
pub enum EmployeeTier {
    User,
    GeneralEmployee,
    Maintenance,
    Admin,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::PaymentTypeEnum)]
pub enum PaymentType {
    Canceled,
    Processing,
    RequiresAction,
    RequiresCapture,
    RequiresConfirmation,
    RequiresPaymentMethod,
    Succeeded,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::PlanTierEnum)]
pub enum PlanTier {
    Free,
    Silver,
    Gold,
    Platinum,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::GenderEnum)]
pub enum Gender {
    Male,
    Female,
    Other,
    PNTS,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::TransactionTypeEnum)]
pub enum TransactionType {
    Credit,
    Cash,
}

//This is for postgres. For other databases the type might be different.
impl ToSql<sql_types::AgreementStatusEnum, Pg> for AgreementStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            AgreementStatus::Rental => out.write_all(b"Rental")?,
            AgreementStatus::Void => out.write_all(b"Void")?,
            AgreementStatus::Canceled => out.write_all(b"Canceled")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::AgreementStatusEnum, Pg> for AgreementStatus {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"Rental" => Ok(AgreementStatus::Rental),
            b"Void" => Ok(AgreementStatus::Void),
            b"Canceled" => Ok(AgreementStatus::Canceled),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
impl ToSql<sql_types::TransactionTypeEnum, Pg> for TransactionType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            TransactionType::Credit => out.write_all(b"Credit")?,
            TransactionType::Cash => out.write_all(b"Cash")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::TransactionTypeEnum, Pg> for TransactionType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"Credit" => Ok(TransactionType::Credit),
            b"Cash" => Ok(TransactionType::Cash),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
impl ToSql<sql_types::VerificationTypeEnum, Pg> for VerificationType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            VerificationType::Phone => out.write_all(b"phone")?,
            VerificationType::Email => out.write_all(b"email")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::VerificationTypeEnum, Pg> for VerificationType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"phone" => Ok(VerificationType::Phone),
            b"email" => Ok(VerificationType::Email),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
// The following is the traits implementation for other Enums.
impl ToSql<sql_types::EmployeeTierEnum, Pg> for EmployeeTier {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            EmployeeTier::User => out.write_all(b"User")?,
            EmployeeTier::GeneralEmployee => out.write_all(b"GeneralEmployee")?,
            EmployeeTier::Maintenance => out.write_all(b"Maintenance")?,
            EmployeeTier::Admin => out.write_all(b"Admin")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::EmployeeTierEnum, Pg> for EmployeeTier {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"User" => Ok(EmployeeTier::User),
            b"GeneralEmployee" => Ok(EmployeeTier::GeneralEmployee),
            b"Maintenance" => Ok(EmployeeTier::Maintenance),
            b"Admin" => Ok(EmployeeTier::Admin),

            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
impl ToSql<sql_types::PaymentTypeEnum, Pg> for PaymentType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            PaymentType::Canceled => out.write_all(b"canceled")?,
            PaymentType::Processing => out.write_all(b"processing")?,
            PaymentType::RequiresAction => out.write_all(b"requires_action")?,
            PaymentType::RequiresCapture => out.write_all(b"requires_capture")?,
            PaymentType::RequiresConfirmation => out.write_all(b"requires_confirmation")?,
            PaymentType::RequiresPaymentMethod => out.write_all(b"requires_payment_method")?,
            PaymentType::Succeeded => out.write_all(b"succeeded")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::PaymentTypeEnum, Pg> for PaymentType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"canceled" => Ok(PaymentType::Canceled),
            b"processing" => Ok(PaymentType::Processing),
            b"requires_action" => Ok(PaymentType::RequiresAction),
            b"requires_capture" => Ok(PaymentType::RequiresCapture),
            b"requires_confirmation" => Ok(PaymentType::RequiresConfirmation),
            b"requires_payment_method" => Ok(PaymentType::RequiresPaymentMethod),
            b"succeeded" => Ok(PaymentType::Succeeded),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
impl ToSql<sql_types::PlanTierEnum, Pg> for PlanTier {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            PlanTier::Free => out.write_all(b"Free")?,
            PlanTier::Silver => out.write_all(b"Silver")?,
            PlanTier::Gold => out.write_all(b"Gold")?,
            PlanTier::Platinum => out.write_all(b"Platinum")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::PlanTierEnum, Pg> for PlanTier {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"Free" => Ok(PlanTier::Free),
            b"Silver" => Ok(PlanTier::Silver),
            b"Gold" => Ok(PlanTier::Gold),
            b"Platinum" => Ok(PlanTier::Platinum),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}
impl ToSql<sql_types::GenderEnum, Pg> for Gender {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            Gender::Male => out.write_all(b"Male")?,
            Gender::Female => out.write_all(b"Female")?,
            Gender::Other => out.write_all(b"Other")?,
            Gender::PNTS => out.write_all(b"PNTS")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::GenderEnum, Pg> for Gender {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"Male" => Ok(Gender::Male),
            b"Female" => Ok(Gender::Female),
            b"Other" => Ok(Gender::Other),
            b"PNTS" => Ok(Gender::PNTS),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = rental_transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RentalTransaction {
    pub id: i32,
    pub agreement_id: i32,
    pub transaction_type: TransactionType,
    pub duration: f64,
    pub transaction_time: DateTime<Utc>,
}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize, AsChangeset)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = renters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Renter {
    pub id: i32,
    pub name: String,
    pub stripe_id: Option<String>,
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
    pub insurance_liability_expiration: Option<NaiveDate>,
    pub insurance_collision_expiration: Option<NaiveDate>,
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
    pub subscription_payment_method_id: Option<i32>,
}

impl Renter {
    pub fn to_publish_renter(&self) -> PublishRenter {
        PublishRenter {
            id: self.id,
            name: self.name.clone(),
            student_email: self.student_email.clone(),
            student_email_expiration: self.student_email_expiration,
            phone: self.phone.clone(),
            phone_is_verified: self.phone_is_verified,
            date_of_birth: self.date_of_birth.clone(),
            profile_picture: self.profile_picture.clone(),
            gender: self.gender.clone(),
            date_of_registration: self.date_of_registration,
            drivers_license_number: self.drivers_license_number.clone(),
            drivers_license_state_region: self.drivers_license_state_region.clone(),
            drivers_license_image: self.drivers_license_image.clone(),
            drivers_license_image_secondary: self.drivers_license_image_secondary.clone(),
            drivers_license_expiration: self.drivers_license_expiration.clone(),
            insurance_id_image: self.insurance_id_image.clone(),
            insurance_liability_expiration: self.insurance_liability_expiration.clone(),
            insurance_collision_expiration: self.insurance_collision_expiration.clone(),
            lease_agreement_image: self.lease_agreement_image.clone(),
            apartment_id: self.apartment_id,
            lease_agreement_expiration: self.lease_agreement_expiration,
            billing_address: self.billing_address.clone(),
            signature_image: self.signature_image.clone(),
            signature_datetime: self.signature_datetime.clone(),
            plan_tier: self.plan_tier.clone(),
            plan_renewal_day: self.plan_renewal_day.clone(),
            plan_expire_month_year: self.plan_expire_month_year.clone(),
            plan_available_duration: self.plan_available_duration,
            is_plan_annual: self.is_plan_annual,
            employee_tier: self.employee_tier.clone(),
            subscription_payment_method_id: self.subscription_payment_method_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublishRenter {
    pub id: i32,
    pub name: String,
    pub student_email: String,
    pub student_email_expiration: Option<NaiveDate>,
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
    pub insurance_liability_expiration: Option<NaiveDate>,
    pub insurance_collision_expiration: Option<NaiveDate>,
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
    pub subscription_payment_method_id: Option<i32>,
}

#[derive(Insertable, Debug, Clone, Deserialize, Serialize)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = renters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRenter {
    pub name: String,
    pub student_email: String,
    pub password: String, // Hash this before inserting!
    pub phone: String,
    pub date_of_birth: NaiveDate,
    pub apartment_id: i32,
    pub plan_renewal_day: String,
    pub plan_expire_month_year: String,
    pub plan_available_duration: f64,
    pub employee_tier: EmployeeTier,
}

#[derive(
    Queryable, Identifiable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsChangeset,
)]
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
    pub md5: String,
    pub nickname: Option<String>,
    pub is_enabled: bool,
    pub renter_id: i32,
    pub last_used_date_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishPaymentMethod {
    pub id: i32,
    pub cardholder_name: String,
    pub masked_card_number: String,
    pub network: String,
    pub expiration: String,
    pub nickname: Option<String>,
    pub is_enabled: bool,
    pub renter_id: i32,
    pub last_used_date_time: Option<DateTime<Utc>>,
}

impl PaymentMethod {
    pub fn to_public_payment_method(&self) -> PublishPaymentMethod {
        PublishPaymentMethod {
            id: self.id,
            cardholder_name: self.cardholder_name.clone(),
            masked_card_number: self.masked_card_number.clone(),
            network: self.network.clone(),
            expiration: self.expiration.clone(),
            nickname: self.nickname.clone(),
            is_enabled: self.is_enabled,
            renter_id: self.renter_id,
            last_used_date_time: self.last_used_date_time.clone(),
        }
    }
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = payment_methods)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewPaymentMethod {
    pub cardholder_name: String,
    pub masked_card_number: String,
    pub network: String,
    pub expiration: String,
    pub token: String,
    pub md5: String,
    pub nickname: Option<String>,
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

impl Apartment {
    pub fn to_publish_apartment(&self) -> PublishApartment {
        PublishApartment {
            id: self.id,
            name: self.name.clone(),
            email: self.email.clone(),
            phone: self.phone.clone(),
            address: self.address.clone(),
            free_tier_hours: self.free_tier_hours,
            free_tier_rate: self.free_tier_rate,
            silver_tier_hours: self.silver_tier_hours,
            silver_tier_rate: self.silver_tier_rate,
            gold_tier_hours: self.gold_tier_hours,
            gold_tier_rate: self.gold_tier_rate,
            platinum_tier_hours: self.platinum_tier_hours,
            platinum_tier_rate: self.platinum_tier_rate,
            duration_rate: self.duration_rate,
            liability_protection_rate: self.liability_protection_rate,
            pcdw_protection_rate: self.pcdw_protection_rate,
            pcdw_ext_protection_rate: self.pcdw_ext_protection_rate,
            rsa_protection_rate: self.rsa_protection_rate,
            pai_protection_rate: self.pai_protection_rate,
            sales_tax_rate: self.sales_tax_rate,
            is_public: self.is_public,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublishApartment {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
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
    pub is_public: bool,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = apartments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewApartment {
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
    pub timestamp_format: String,
    pub timezone: Option<String>,
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[diesel(table_name = transponder_companies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTransponderCompany {
    pub name: String,
    pub corresponding_key_for_vehicle_id: String,
    pub corresponding_key_for_transaction_name: String,
    pub custom_prefix_for_transaction_name: String,
    pub corresponding_key_for_transaction_time: String,
    pub corresponding_key_for_transaction_amount: String,
    pub timestamp_format: String,
    pub timezone: Option<String>,
}

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublishVehicle {
    pub id: i32,
    pub vin: String,
    pub name: String,
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
    pub apartment_id: i32,
}

impl Vehicle {
    pub fn to_publish_vehicle(&self) -> PublishVehicle {
        PublishVehicle {
            id: self.id,
            vin: self.vin.clone(),
            name: self.name.clone(),
            license_number: self.license_number.clone(),
            license_state: self.license_state.clone(),
            year: self.year.clone(),
            make: self.make.clone(),
            model: self.model.clone(),
            msrp_factor: self.msrp_factor,
            image_link: self.image_link.clone(),
            odometer: self.odometer,
            tank_size: self.tank_size,
            tank_level_percentage: self.tank_level_percentage,
            apartment_id: self.apartment_id,
        }
    }
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = vehicles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewVehicle {
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

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = damage_submissions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DamageSubmission {
    pub id: i32,
    pub reported_by: i32,
    pub first_image: String,
    pub second_image: String,
    pub third_image: Option<String>,
    pub fourth_image: Option<String>,
    pub description: String,
    pub processed: bool,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(table_name = damage_submissions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDamageSubmission {
    pub reported_by: i32,
    pub first_image: String,
    pub second_image: String,
    pub third_image: Option<String>,
    pub fourth_image: Option<String>,
    pub description: String,
}

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
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
pub struct NewDamage {
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

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
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
    pub pickup_front_image: Option<String>,
    pub pickup_back_image: Option<String>,
    pub pickup_left_image: Option<String>,
    pub pickup_right_image: Option<String>,
    pub actual_drop_off_time: Option<DateTime<Utc>>,
    pub drop_off_odometer: Option<i32>,
    pub drop_off_level: Option<i32>,
    pub drop_off_front_image: Option<String>,
    pub drop_off_back_image: Option<String>,
    pub drop_off_left_image: Option<String>,
    pub drop_off_right_image: Option<String>,
    pub tax_rate: f64,
    pub msrp_factor: f64,
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
pub struct NewAgreement {
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
    pub tax_rate: f64,
    pub msrp_factor: f64,
    pub duration_rate: f64,
    pub apartment_id: i32,
    pub vehicle_id: i32,
    pub renter_id: i32,
    pub payment_method_id: i32,
}

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = charges)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Charge {
    pub id: i32,
    pub name: String,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub agreement_id: Option<i32>,
    pub vehicle_id: i32,
    pub checksum: String,
    pub transponder_company_id: Option<i32>,
    pub vehicle_identifier: Option<String>,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = charges)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewCharge {
    pub name: String,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub agreement_id: Option<i32>,
    pub vehicle_id: i32,
    pub checksum: String,
    pub transponder_company_id: Option<i32>,
    pub vehicle_identifier: Option<String>,
}

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(Agreement))]
#[diesel(belongs_to(PaymentMethod))]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = payments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Payment {
    pub id: i32,
    pub r#type: PaymentType,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub reference_number: Option<String>,
    pub agreement_id: Option<i32>,
    pub renter_id: i32,
    pub payment_method_id: i32,
}

#[derive(Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Agreement))]
#[diesel(belongs_to(PaymentMethod))]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = payments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewPayment {
    pub payment_type: PaymentType,
    pub amount: f64,
    pub note: Option<String>,
    pub reference_number: Option<String>,
    pub agreement_id: Option<i32>,
    pub renter_id: i32,
    pub payment_method_id: i32,
}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = access_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AccessToken {
    pub id: i32,
    pub user_id: i32,
    pub token: Vec<u8>,
    pub exp: DateTime<Utc>,
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = access_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAccessToken {
    pub user_id: i32,
    pub token: Vec<u8>,
    pub exp: DateTime<Utc>,
}

impl AccessToken {
    pub fn to_publish_access_token(&self) -> PublishAccessToken {
        let token_string = hex::encode(self.token.clone());
        PublishAccessToken {
            token: token_string,
            exp: self.exp,
        }
    }
    pub fn to_header_map(&self) -> HeaderMap {
        let mut header_map = HeaderMap::new();
        let token_string = hex::encode(self.token.clone());
        let exp_string = self.exp.to_string();
        header_map.insert(
            "token",
            HeaderValue::from_str(token_string.as_str()).unwrap(),
        );
        header_map.insert("exp", HeaderValue::from_str(exp_string.as_str()).unwrap());
        header_map
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishAccessToken {
    pub token: String,
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
pub struct NewDoNotRentList {
    pub name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub note: String,
    pub exp: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct RequestToken {
    pub user_id: i32,
    pub token: String,
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq, AsChangeset)]
#[diesel(table_name = verifications)]
#[diesel(belongs_to(Renter))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Verification {
    pub id: i32,
    pub verification_method: VerificationType,
    pub renter_id: i32,
    pub expires_at: DateTime<Utc>,
    pub code: String,
}

#[derive(Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = verifications)]
#[diesel(belongs_to(Renter))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewVerification {
    pub verification_method: VerificationType,
    pub renter_id: i32,
    pub code: String,
}
