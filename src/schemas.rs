use chrono::{DateTime, Utc, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgreementStatus { // Added pub
    Rental,
    Void,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmployeeTier { // Added pub
    User,
    GeneralEmployee,
    Maintenance,
    Admin,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentType { // Added pub
    Cash,
    ACH,
    CC,
    BadDebt,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanTier { // Added pub
    Free,
    Silver,
    Gold,
    Platinum,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender { // Added pub
    Male,
    Female,
    Other,
    PNTS, // prefer not to say
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Renter { // Added pub
    pub id: i32, // not public
    pub name: String,
    pub student_email: String,
    pub student_email_expiration: Option<NaiveDate>,
    pub password: String, // not public
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct PaymentMethod {
    pub id: i32, // not public
    pub cardholder_name: String,
    pub masked_card_number: String,
    pub network: String,
    pub expiration: String,
    pub token: String,
    pub nickname: Option<String>,
    pub is_enabled: bool,
    pub renter_id: i32, // not public
    pub last_used_date_time: Option<DateTime<Utc>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Apartment {
    pub id: i32, // not public
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TransponderCompany {
    pub id: i32, // not public
    pub name: String,
    pub corresponding_key_for_vehicle_id: String,
    pub corresponding_key_for_transaction_name: String,
    pub custom_prefix_for_transaction_name: String,
    pub corresponding_key_for_transaction_time: String,
    pub corresponding_key_for_transaction_amount: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Vehicle {
    pub id: i32, // not public
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
    pub forth_transponder_number: Option<String>,
    pub forth_transponder_company_id: Option<i32>,
    pub apartment_id: i32, // not public
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Damage {
    pub id: i32, // not public
    pub note: String,
    pub record_date: DateTime<Utc>,
    pub occur_date: DateTime<Utc>,
    pub standard_coordination_x_percentage: i32,
    pub standard_coordination_y_percentage: i32,
    pub first_image: Option<String>,
    pub second_image: Option<String>,
    pub third_image: Option<String>,
    pub forth_image: Option<String>,
    pub fixed_date: Option<DateTime<Utc>>,
    pub fixed_amount: Option<f64>,
    pub agreement_id: Option<i32>, // not public
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Agreement {
    pub id: i32, // not public
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
    pub msrp_factor: f64,
    pub plan_duration: f64,
    pub pay_as_you_go_duration: f64,
    pub duration_rate: f64, // rate to extend hourly
    pub apartment_id: i32, // not public
    pub vehicle_id: i32, // not public
    pub renter_id: i32, // not public
    pub payment_method_id: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Charge {
    pub id: i32, // not public
    pub name: String,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub agreement_id: i32, // not public
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Payment {
    pub id: i32, // not public
    pub r#type: PaymentType,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub note: Option<String>,
    pub reference_number: Option<String>,
    pub agreement_id: i32, // not public
    pub payment_method_id: Option<i32>,
}
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct AccessToken {
    pub id: i32, // not public
    pub user_id: i32, // not public
    pub token: String,
    pub exp: DateTime<Utc>, // not public
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DoNotRentList {
    pub id: i32, // not public
    pub name: Option<String>, // not public
    pub phone: Option<String>, // not public
    pub email: Option<String>, // not public
    pub note: String, // not public
    pub exp: Option<NaiveDate>, // not public
}