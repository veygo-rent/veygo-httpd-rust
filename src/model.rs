use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use warp::http::header::{HeaderMap, HeaderValue};

// Diesel requires us to define a custom mapping between the Rust enum
// and the database type if we are not using string.
use crate::schema::*;
use diesel::deserialize::{self, FromSql};
use diesel::pg::{Pg, PgValue};
use diesel::serialize::{self, Output, ToSql, WriteTuple};
use diesel::{AsExpression, FromSqlRow};
use std::io::Write;

use diesel::sql_types::{VarChar, Nullable, Record};

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::UsAddress)]
pub struct UsAddress {
    pub street_address: String,
    pub extended_address: Option<String>,
    pub city: String,
    pub state: String,
    pub zipcode: String,
}

impl FromSql<sql_types::UsAddress, Pg> for UsAddress {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let (street_address, extended_address, city, state, zipcode) = FromSql::<Record<(VarChar, Nullable<VarChar>, VarChar, VarChar, VarChar)>, Pg>::from_sql(bytes)?;
        Ok(
            UsAddress { street_address, extended_address, city, state, zipcode }
        )
    }
}

impl ToSql<sql_types::UsAddress, Pg> for UsAddress {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let street_address = self.street_address.clone();
        let extended_address = self.extended_address.clone();
        let city = self.city.clone();
        let state = self.state.clone();
        let zipcode = self.zipcode.clone();
        WriteTuple::<(VarChar, Nullable<VarChar>, VarChar, VarChar, VarChar)>::write_tuple(
            &(street_address, extended_address, city, state, zipcode),
            out,
        )
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::TaxTypeEnum)]
pub enum TaxType {
    Percent,
    Daily,
    Fixed,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::PolicyEnum)]
pub enum PolicyType {
    Rental,
    Privacy,
    Membership,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::AuditActionEnum)]
pub enum AuditActionType {
    Create,
    Update,
    Read,
    Delete,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::VerificationTypeEnum)]
pub enum VerificationType {
    Email,
    Phone,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = sql_types::RemoteMgmtEnum)]
pub enum RemoteMgmtType {
    Revers,
    Tesla,
    Geotab,
    None,
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
    RequiresCapture,
    RequiresPaymentMethod,
    Succeeded,
    VeygoBadDebt,
    VeygoInsurance,
}

impl From<stripe::PaymentIntentStatus> for PaymentType {
    fn from(status: stripe::PaymentIntentStatus) -> Self {
        match status {
            stripe::PaymentIntentStatus::Canceled => PaymentType::Canceled,
            stripe::PaymentIntentStatus::RequiresCapture => PaymentType::RequiresCapture,
            stripe::PaymentIntentStatus::RequiresPaymentMethod => PaymentType::RequiresPaymentMethod,
            stripe::PaymentIntentStatus::Succeeded => PaymentType::Succeeded,
            _ => PaymentType::Canceled,
        }
    }
}

#[derive(
    Deserialize,
    Serialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    AsExpression,
    FromSqlRow,
    Ord,
    PartialOrd,
)]
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

//This is for postgres. For other databases the type might be different.
impl ToSql<sql_types::PolicyEnum, Pg> for PolicyType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            PolicyType::Rental => out.write_all(b"rental")?,
            PolicyType::Privacy => out.write_all(b"privacy")?,
            PolicyType::Membership => out.write_all(b"membership")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::PolicyEnum, Pg> for PolicyType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"rental" => Ok(PolicyType::Rental),
            b"privacy" => Ok(PolicyType::Privacy),
            b"membership" => Ok(PolicyType::Membership),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<sql_types::TaxTypeEnum, Pg> for TaxType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            TaxType::Percent => out.write_all(b"percent")?,
            TaxType::Daily => out.write_all(b"daily")?,
            TaxType::Fixed => out.write_all(b"fixed")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::TaxTypeEnum, Pg> for TaxType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"percent" => Ok(TaxType::Percent),
            b"daily" => Ok(TaxType::Daily),
            b"fixed" => Ok(TaxType::Fixed),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<sql_types::AuditActionEnum, Pg> for AuditActionType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            AuditActionType::Create => out.write_all(b"create")?,
            AuditActionType::Update => out.write_all(b"update")?,
            AuditActionType::Read => out.write_all(b"read")?,
            AuditActionType::Delete => out.write_all(b"delete")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::AuditActionEnum, Pg> for AuditActionType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"create" => Ok(AuditActionType::Create),
            b"update" => Ok(AuditActionType::Update),
            b"read" => Ok(AuditActionType::Read),
            b"delete" => Ok(AuditActionType::Delete),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

impl ToSql<sql_types::RemoteMgmtEnum, Pg> for RemoteMgmtType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        match *self {
            RemoteMgmtType::Revers => out.write_all(b"reverse")?,
            RemoteMgmtType::Tesla => out.write_all(b"tesla")?,
            RemoteMgmtType::Geotab => out.write_all(b"geotab")?,
            RemoteMgmtType::None => out.write_all(b"none")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::RemoteMgmtEnum, Pg> for RemoteMgmtType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"reverse" => Ok(RemoteMgmtType::Revers),
            b"tesla" => Ok(RemoteMgmtType::Tesla),
            b"geotab" => Ok(RemoteMgmtType::Geotab),
            b"none" => Ok(RemoteMgmtType::None),
            _ => Err("Unrecognized enum variant".into()),
        }
    }
}

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
            PaymentType::RequiresCapture => out.write_all(b"requires_capture")?,
            PaymentType::RequiresPaymentMethod => out.write_all(b"requires_payment_method")?,
            PaymentType::Succeeded => out.write_all(b"succeeded")?,
            PaymentType::VeygoBadDebt => out.write_all(b"veygo.bad_debt")?,
            PaymentType::VeygoInsurance => out.write_all(b"veygo.insurance")?,
        }
        Ok(serialize::IsNull::No)
    }
}

impl FromSql<sql_types::PaymentTypeEnum, Pg> for PaymentType {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        match bytes.as_bytes() {
            b"canceled" => Ok(PaymentType::Canceled),
            b"requires_capture" => Ok(PaymentType::RequiresCapture),
            b"requires_payment_method" => Ok(PaymentType::RequiresPaymentMethod),
            b"succeeded" => Ok(PaymentType::Succeeded),
            b"veygo.bad_debt" => Ok(PaymentType::VeygoBadDebt),
            b"veygo.insurance" => Ok(PaymentType::VeygoInsurance),
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

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize, AsChangeset,
)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = reward_transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RewardTransaction {
    pub id: i32,
    pub agreement_id: Option<i32>,
    pub duration: f64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub transaction_time: DateTime<Utc>,
    pub renter_id: i32,
}

#[derive(
    Insertable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = reward_transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRewardTransaction {
    pub agreement_id: Option<i32>,
    pub duration: f64,
    pub renter_id: i32,
}

#[derive(
    Queryable,
    Identifiable,
    Associations,
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = renters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Renter {
    // Absolutely No Frontend Access
    pub id: i32,
    pub name: String,
    pub stripe_id: Option<String>,
    pub student_email: String,
    pub student_email_expiration: Option<NaiveDate>,
    pub password: String,
    pub phone: String,
    pub phone_is_verified: bool,
    pub date_of_birth: NaiveDate,
    pub profile_picture: Option<String>,
    pub gender: Option<Gender>,
    #[serde(with = "chrono::serde::ts_seconds")]
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
    pub billing_address: Option<UsAddress>,
    pub signature_image: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub signature_datetime: Option<DateTime<Utc>>,
    pub plan_tier: PlanTier,
    pub plan_renewal_day: String,
    pub plan_expire_month_year: String,
    pub plan_available_duration: f64,
    pub is_plan_annual: bool,
    pub employee_tier: EmployeeTier,
    pub subscription_payment_method_id: Option<i32>,
    pub apple_apns: Option<String>,
    pub admin_apple_apns: Option<String>,
    pub requires_secondary_driver_lic: bool
}

impl From<Renter> for PublishRenter {
    fn from(renter: Renter) -> Self {
        PublishRenter {
            id: renter.id,
            name: renter.name,
            student_email: renter.student_email,
            student_email_expiration: renter.student_email_expiration,
            phone: renter.phone,
            phone_is_verified: renter.phone_is_verified,
            date_of_birth: renter.date_of_birth,
            profile_picture: renter.profile_picture,
            gender: renter.gender,
            date_of_registration: renter.date_of_registration,
            drivers_license_number: renter.drivers_license_number,
            drivers_license_state_region: renter.drivers_license_state_region,
            drivers_license_image: renter.drivers_license_image,
            drivers_license_image_secondary: renter.drivers_license_image_secondary,
            drivers_license_expiration: renter.drivers_license_expiration,
            insurance_id_image: renter.insurance_id_image,
            insurance_liability_expiration: renter.insurance_liability_expiration,
            insurance_collision_expiration: renter.insurance_collision_expiration,
            lease_agreement_image: renter.lease_agreement_image,
            apartment_id: renter.apartment_id,
            lease_agreement_expiration: renter.lease_agreement_expiration,
            billing_address: renter.billing_address,
            signature_image: renter.signature_image,
            signature_datetime: renter.signature_datetime,
            plan_tier: renter.plan_tier,
            plan_renewal_day: renter.plan_renewal_day,
            plan_expire_month_year: renter.plan_expire_month_year,
            plan_available_duration: renter.plan_available_duration,
            is_plan_annual: renter.is_plan_annual,
            employee_tier: renter.employee_tier,
            subscription_payment_method_id: renter.subscription_payment_method_id,
            requires_secondary_driver_lic: renter.requires_secondary_driver_lic,
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
    #[serde(with = "chrono::serde::ts_seconds")]
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
    pub billing_address: Option<UsAddress>,
    pub signature_image: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub signature_datetime: Option<DateTime<Utc>>,
    pub plan_tier: PlanTier,
    pub plan_renewal_day: String,
    pub plan_expire_month_year: String,
    pub plan_available_duration: f64,
    pub is_plan_annual: bool,
    pub employee_tier: EmployeeTier,
    pub subscription_payment_method_id: Option<i32>,
    pub requires_secondary_driver_lic: bool
}

#[derive(Insertable, Debug, Clone, Deserialize, Serialize)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = renters)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRenter {
    pub name: String,
    pub student_email: String,
    pub password: String,
    pub phone: String,
    pub date_of_birth: NaiveDate,
    pub apartment_id: i32,
    pub plan_renewal_day: String,
    pub plan_expire_month_year: String,
    pub plan_available_duration: f64,
    pub employee_tier: EmployeeTier,
}

#[derive(
    Queryable,
    Identifiable,
    Associations,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    AsChangeset,
)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = payment_methods)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PaymentMethod {
    // Absolutely No Frontend Access
    pub id: i32,
    pub cardholder_name: String,
    pub masked_card_number: String,
    pub network: String,
    pub expiration: String,
    pub token: String,
    pub fingerprint: String,
    pub nickname: Option<String>,
    pub is_enabled: bool,
    pub renter_id: i32,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_used_date_time: Option<DateTime<Utc>>,
    pub cdw_enabled: bool,
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
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_used_date_time: Option<DateTime<Utc>>,
    pub cdw_enabled: bool,
}

impl From<PaymentMethod> for PublishPaymentMethod {
    fn from(pm: PaymentMethod) -> Self {
        PublishPaymentMethod {
            id: pm.id,
            cardholder_name: pm.cardholder_name,
            masked_card_number: pm.masked_card_number,
            network: pm.network,
            expiration: pm.expiration,
            nickname: pm.nickname,
            is_enabled: pm.is_enabled,
            renter_id: pm.renter_id,
            last_used_date_time: pm.last_used_date_time,
            cdw_enabled: pm.cdw_enabled,
        }
    }
}

#[derive(Deserialize, Serialize, Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = payment_methods)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewPaymentMethod {
    pub cardholder_name: String,
    pub masked_card_number: String,
    pub network: String,
    pub expiration: String,
    pub token: String,
    pub fingerprint: String,
    pub nickname: Option<String>,
    pub is_enabled: bool,
    pub renter_id: i32,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_used_date_time: Option<DateTime<Utc>>,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = apartments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Apartment {
    pub id: i32,
    pub name: String,
    pub timezone: String,
    pub email: String,
    pub phone: String,
    pub address: UsAddress,
    pub accepted_school_email_domain: String,
    pub free_tier_hours: f64,
    pub silver_tier_hours: Option<f64>,
    pub silver_tier_rate: Option<f64>,
    pub gold_tier_hours: Option<f64>,
    pub gold_tier_rate: Option<f64>,
    pub platinum_tier_hours: Option<f64>,
    pub platinum_tier_rate: Option<f64>,
    pub duration_rate: f64,
    pub liability_protection_rate: Option<f64>,
    pub pcdw_protection_rate: Option<f64>,
    pub pcdw_ext_protection_rate: Option<f64>,
    pub rsa_protection_rate: Option<f64>,
    pub pai_protection_rate: Option<f64>,
    pub is_operating: bool,
    pub is_public: bool,
    pub uni_id: i32,
    pub mileage_rate_overwrite: Option<f64>,
    pub mileage_package_overwrite: Option<f64>,
    pub mileage_conversion: f64,
}

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = apartments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewApartment {
    pub name: String,
    pub timezone: String,
    pub email: String,
    pub phone: String,
    pub address: UsAddress,
    pub accepted_school_email_domain: String,
    pub free_tier_hours: f64,
    pub silver_tier_hours: Option<f64>,
    pub silver_tier_rate: Option<f64>,
    pub gold_tier_hours: Option<f64>,
    pub gold_tier_rate: Option<f64>,
    pub platinum_tier_hours: Option<f64>,
    pub platinum_tier_rate: Option<f64>,
    pub duration_rate: f64,
    pub liability_protection_rate: Option<f64>,
    pub pcdw_protection_rate: Option<f64>,
    pub pcdw_ext_protection_rate: Option<f64>,
    pub rsa_protection_rate: Option<f64>,
    pub pai_protection_rate: Option<f64>,
    pub is_operating: bool,
    pub is_public: bool,
    pub uni_id: i32,
    pub mileage_rate_overwrite: Option<f64>,
    pub mileage_package_overwrite: Option<f64>,
    pub mileage_conversion: f64,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = locations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Location {
    pub id: i32,
    pub apartment_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub is_operational: bool,
}

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = locations)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewLocation {
    pub apartment_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
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
    Queryable, Identifiable, Associations, AsChangeset, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(Location))]
#[diesel(table_name = vehicles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Vehicle {
    pub id: i32,
    pub vin: String,
    pub name: String,
    pub capacity: i32,
    pub doors: i32,
    pub small_bags: i32,
    pub large_bags: i32,
    pub carplay: bool,
    pub lane_keep: bool,
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
    pub location_id: i32,
    pub remote_mgmt: RemoteMgmtType,
    pub remote_mgmt_id: String,
    pub requires_own_insurance: bool,
    pub admin_pin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublishRenterVehicle {
    pub id: i32,
    pub vin: String,
    pub name: String,
    pub capacity: i32,
    pub doors: i32,
    pub small_bags: i32,
    pub large_bags: i32,
    pub carplay: bool,
    pub lane_keep: bool,
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
    pub location_id: i32,
    pub remote_mgmt: RemoteMgmtType,
    pub requires_own_insurance: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublishAdminVehicle {
    pub id: i32,
    pub name: String,
    pub vin: String,
    pub capacity: i32,
    pub doors: i32,
    pub small_bags: i32,
    pub large_bags: i32,
    pub carplay: bool,
    pub lane_keep: bool,
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
    pub location_id: i32,
    pub remote_mgmt: RemoteMgmtType,
    pub remote_mgmt_id: String,
    pub requires_own_insurance: bool,
    pub admin_pin: Option<String>,
}

impl From<Vehicle> for PublishRenterVehicle {
    fn from(v: Vehicle) -> Self {
        PublishRenterVehicle {
            id: v.id,
            vin: v.vin,
            name: v.name,
            capacity: v.capacity,
            doors: v.doors,
            small_bags: v.small_bags,
            large_bags: v.large_bags,
            carplay: v.carplay,
            lane_keep: v.lane_keep,
            license_number: v.license_number,
            license_state: v.license_state,
            year: v.year,
            make: v.make,
            model: v.model,
            msrp_factor: v.msrp_factor,
            image_link: v.image_link,
            odometer: v.odometer,
            tank_size: v.tank_size,
            tank_level_percentage: v.tank_level_percentage,
            location_id: v.location_id,
            remote_mgmt: v.remote_mgmt,
            requires_own_insurance: v.requires_own_insurance,
        }
    }
}

impl From<Vehicle> for PublishAdminVehicle {
    fn from(v: Vehicle) -> Self {
        PublishAdminVehicle {
            id: v.id,
            vin: v.vin,
            capacity: v.capacity,
            doors: v.doors,
            small_bags: v.small_bags,
            large_bags: v.large_bags,
            carplay: v.carplay,
            lane_keep: v.lane_keep,
            name: v.name,
            available: v.available,
            license_number: v.license_number,
            license_state: v.license_state,
            year: v.year,
            make: v.make,
            model: v.model,
            msrp_factor: v.msrp_factor,
            image_link: v.image_link,
            odometer: v.odometer,
            tank_size: v.tank_size,
            tank_level_percentage: v.tank_level_percentage,
            first_transponder_number: v.first_transponder_number,
            first_transponder_company_id: v.first_transponder_company_id,
            second_transponder_number: v.second_transponder_number,
            second_transponder_company_id: v.second_transponder_company_id,
            third_transponder_number: v.third_transponder_number,
            third_transponder_company_id: v.third_transponder_company_id,
            fourth_transponder_number: v.fourth_transponder_number,
            fourth_transponder_company_id: v.fourth_transponder_company_id,
            location_id: v.location_id,
            remote_mgmt: v.remote_mgmt,
            remote_mgmt_id: v.remote_mgmt_id,
            requires_own_insurance: v.requires_own_insurance,
            admin_pin: v.admin_pin,
        }
    }
}

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Apartment))]
#[diesel(table_name = vehicles)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewVehicle {
    pub vin: String,
    pub name: String,
    pub capacity: i32,
    pub doors: i32,
    pub small_bags: i32,
    pub large_bags: i32,
    pub carplay: bool,
    pub lane_keep: bool,
    pub available: bool,
    pub license_number: String,
    pub license_state: String,
    pub year: String,
    pub make: String,
    pub model: String,
    pub msrp_factor: f64,
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
    pub location_id: i32,
    pub remote_mgmt: RemoteMgmtType,
    pub remote_mgmt_id: String,
    pub requires_own_insurance: bool,
}

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
#[diesel(table_name = damage_submissions)]
#[diesel(belongs_to(Renter, foreign_key = reported_by))]
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

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[diesel(table_name = claims)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Claim {
    pub id: i32,
    pub note: Option<String>,
    pub time: DateTime<Utc>,
    pub agreement_id: i32,
    pub admin_fee: Option<f64>,
    pub tow_charge: Option<f64>,
}

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
#[diesel(belongs_to(Claim))]
#[diesel(belongs_to(Vehicle))]
#[diesel(table_name = damages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Damage {
    pub id: i32,
    pub note: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub record_date: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub occur_date: DateTime<Utc>,
    pub standard_coordination_x_percentage: i32,
    pub standard_coordination_y_percentage: i32,
    pub first_image: String,
    pub second_image: String,
    pub third_image: Option<String>,
    pub fourth_image: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub fixed_date: Option<DateTime<Utc>>,
    pub fixed_amount: Option<f64>,
    pub depreciation: Option<f64>,
    pub lost_of_use: Option<f64>,
    pub claim_id: i32,
    pub vehicle_id: i32,
}

#[derive(Deserialize, Serialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Claim))]
#[diesel(belongs_to(Vehicle))]
#[diesel(table_name = damages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDamage {
    pub note: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub record_date: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub occur_date: DateTime<Utc>,
    pub standard_coordination_x_percentage: i32,
    pub standard_coordination_y_percentage: i32,
    pub first_image: String,
    pub second_image: String,
    pub third_image: Option<String>,
    pub fourth_image: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub fixed_date: Option<DateTime<Utc>>,
    pub fixed_amount: Option<f64>,
    pub depreciation: Option<f64>,
    pub lost_of_use: Option<f64>,
    pub claim_id: i32,
    pub vehicle_id: i32,
}

#[derive(Queryable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Vehicle))]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = vehicle_snapshots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VehicleSnapshot {
    pub id: i32,
    pub left_image: String,
    pub right_image: String,
    pub front_image: String,
    pub back_image: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
    pub odometer: i32,
    pub level: i32,
    pub vehicle_id: i32,
    pub rear_right: String,
    pub rear_left: String,
    pub front_right: String,
    pub front_left: String,
    pub dashboard: Option<String>,
    pub renter_id: i32,
}

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(belongs_to(Vehicle))]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = vehicle_snapshots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewVehicleSnapshot {
    pub left_image: String,
    pub right_image: String,
    pub front_image: String,
    pub back_image: String,
    pub odometer: i32,
    pub level: i32,
    pub vehicle_id: i32,
    pub rear_right: String,
    pub rear_left: String,
    pub front_right: String,
    pub front_left: String,
    pub dashboard: Option<String>,
    pub renter_id: i32,
}

#[derive(Queryable, Selectable, Identifiable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = promos)]
#[diesel(primary_key(code))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Promo {
    pub code: String,
    pub name: String,
    pub amount: f64,
    pub is_enabled: bool,
    pub is_one_time: bool,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
    pub user_id: Option<i32>,
    pub apt_id: Option<i32>,
    pub uni_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublishPromo {
    pub code: String,
    pub name: String,
    pub amount: f64,
    pub is_enabled: bool,
    pub is_one_time: bool,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
}

impl From<Promo> for PublishPromo {
    fn from(p: Promo) -> Self {
        PublishPromo {
            code: p.code,
            name: p.name,
            amount: p.amount,
            is_enabled: p.is_enabled,
            is_one_time: p.is_one_time,
            exp: p.exp,
        }
    }
}

#[derive(
    Queryable,
    Selectable,
    AsChangeset,
    Identifiable,
    Associations,
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
)]
#[diesel(belongs_to(Vehicle))]
#[diesel(belongs_to(Location))]
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
    pub user_billing_address: UsAddress,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub rsvp_pickup_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub rsvp_drop_off_time: DateTime<Utc>,
    pub liability_protection_rate: Option<f64>,
    pub pcdw_protection_rate: Option<f64>,
    pub pcdw_ext_protection_rate: Option<f64>,
    pub rsa_protection_rate: Option<f64>,
    pub pai_protection_rate: Option<f64>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub actual_pickup_time: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub actual_drop_off_time: Option<DateTime<Utc>>,
    pub msrp_factor: f64,
    pub duration_rate: f64,
    pub vehicle_id: i32,
    pub renter_id: i32,
    pub payment_method_id: i32,
    pub vehicle_snapshot_before: Option<i32>,
    pub vehicle_snapshot_after: Option<i32>,
    pub promo_id: Option<String>,
    pub manual_discount: Option<f64>,
    pub location_id: i32,
    pub mileage_package_id: Option<i32>,
    pub mileage_conversion: f64,
    pub mileage_rate_overwrite: Option<f64>,
    pub mileage_package_overwrite: Option<f64>,
    pub utilization_factor: f64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub date_of_creation: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Location))]
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
    pub user_billing_address: UsAddress,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub rsvp_pickup_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub rsvp_drop_off_time: DateTime<Utc>,
    pub liability_protection_rate: Option<f64>,
    pub pcdw_protection_rate: Option<f64>,
    pub pcdw_ext_protection_rate: Option<f64>,
    pub rsa_protection_rate: Option<f64>,
    pub pai_protection_rate: Option<f64>,
    pub msrp_factor: f64,
    pub duration_rate: f64,
    pub vehicle_id: i32,
    pub renter_id: i32,
    pub payment_method_id: i32,
    pub promo_id: Option<String>,
    pub manual_discount: Option<f64>,
    pub location_id: i32,
    pub mileage_package_id: Option<i32>,
    pub mileage_conversion: f64,
    pub mileage_rate_overwrite: Option<f64>,
    pub mileage_package_overwrite: Option<f64>,
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
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub agreement_id: Option<i32>,
    pub vehicle_id: i32,
    pub transponder_company_id: Option<i32>,
    pub vehicle_identifier: Option<String>,
}

#[derive(Deserialize, Serialize, Insertable, Debug, Clone, PartialEq)]
#[diesel(belongs_to(Agreement))]
#[diesel(table_name = charges)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewCharge {
    pub name: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub agreement_id: Option<i32>,
    pub vehicle_id: i32,
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
    pub payment_type: PaymentType,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub reference_number: Option<String>,
    pub agreement_id: i32,
    pub renter_id: i32,
    pub payment_method_id: Option<i32>,
    pub amount_authorized: f64,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub capture_before: Option<DateTime<Utc>>,
    pub is_deposit: bool,
}

#[derive(Deserialize, Serialize, Insertable, Debug, Clone, PartialEq)]
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
    pub agreement_id: i32,
    pub renter_id: i32,
    pub payment_method_id: Option<i32>,
    pub amount_authorized: f64,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub capture_before: Option<DateTime<Utc>>,
    pub is_deposit: bool,
}

#[derive(
    Queryable, Identifiable, Associations, Debug, Clone, PartialEq, Eq, Serialize, Deserialize,
)]
#[diesel(belongs_to(Renter, foreign_key = user_id))]
#[diesel(table_name = access_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AccessToken {
    pub id: i32,
    pub user_id: i32,
    pub token: Vec<u8>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(belongs_to(Renter))]
#[diesel(table_name = access_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAccessToken {
    pub user_id: i32,
    pub token: Vec<u8>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
}

impl AccessToken {
    pub fn to_header_map(&self) -> HeaderMap {
        let mut header_map = HeaderMap::new();
        let token_string = hex::encode(self.token.clone());
        let exp_string = self.exp.to_string();
        (&mut header_map).insert(
            "token",
            HeaderValue::from_str(token_string.as_str()).unwrap(),
        );
        (&mut header_map).insert("exp", HeaderValue::from_str(exp_string.as_str()).unwrap());
        header_map
    }
}

impl From<AccessToken> for PublishAccessToken {
    fn from(at: AccessToken) -> Self {
        let token_string = hex::encode(at.token.clone());
        PublishAccessToken {
            token: token_string,
            exp: at.exp,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishAccessToken {
    pub token: String,
    #[serde(with = "chrono::serde::ts_seconds")]
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

#[derive(Insertable, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, AsChangeset)]
#[diesel(table_name = verifications)]
#[diesel(belongs_to(Renter))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Verification {
    pub id: i32,
    pub verification_method: VerificationType,
    pub renter_id: i32,
    #[serde(with = "chrono::serde::ts_seconds")]
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

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = taxes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTax {
    pub name: String,
    pub multiplier: f64,
    pub is_effective: bool,
    pub is_sales_tax: bool,
    pub tax_type: TaxType,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, AsChangeset, Queryable)]
#[diesel(table_name = taxes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Tax {
    pub id: i32,
    pub name: String,
    pub multiplier: f64,
    pub is_effective: bool,
    pub is_sales_tax: bool,
    pub tax_type: TaxType,
}

#[derive(Insertable, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[diesel(table_name = mileage_packages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMileagePackage {
    pub miles: i32,
    pub discounted_rate: i32,
    pub is_active: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, AsChangeset, Queryable)]
#[diesel(table_name = mileage_packages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MileagePackage {
    pub id: i32,
    pub miles: i32,
    pub discounted_rate: i32,
    pub is_active: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, AsChangeset, Queryable, Insertable)]
#[diesel(table_name = agreements_damages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AgreementDamage {
    pub agreement_id: i32,
    pub damage_id: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, AsChangeset, Queryable, Insertable)]
#[diesel(table_name = agreements_taxes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AgreementTax {
    pub agreement_id: i32,
    pub tax_id: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, AsChangeset, Queryable, Insertable)]
#[diesel(table_name = apartments_taxes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ApartmentTax {
    pub apartment_id: i32,
    pub tax_id: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, AsChangeset, Queryable)]
#[diesel(table_name = audits)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Audit {
    pub id: i32,
    pub renter_id: Option<i32>,
    pub action: AuditActionType,
    pub path: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Insertable)]
#[diesel(table_name = audits)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAudit {
    pub renter_id: Option<i32>,
    pub action: AuditActionType,
    pub path: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(table_name = policies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Policy {
    pub id: i32,
    pub policy_type: PolicyType,
    pub policy_effective_date: NaiveDate,
    pub content: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Insertable)]
#[diesel(table_name = policies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewPolicy {
    pub policy_type: PolicyType,
    pub policy_effective_date: NaiveDate,
    pub content: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Queryable, Identifiable)]
#[diesel(table_name = rate_offers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RateOffer {
    pub id: i32,
    pub renter_id: i32,
    pub apartment_id: i32,
    pub multiplier: f64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub exp: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Insertable)]
#[diesel(table_name = rate_offers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewRateOffer {
    pub renter_id: i32,
    pub apartment_id: i32,
    pub multiplier: f64,
}
