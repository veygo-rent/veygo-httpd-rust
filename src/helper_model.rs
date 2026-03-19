use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use crate::model;
use rust_decimal::prelude::*;

#[derive(Serialize, Debug, Clone)]
pub struct ErrorResponse {
    pub title: String,
    pub message: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct TripDetailedInfo {
    pub agreement: model::Agreement,
    pub vehicle: model::PublishRenterVehicle,
    pub apartment: model::Apartment,
    pub location: model::Location,
    pub vehicle_snapshot_before: Option<model::VehicleSnapshot>,
    pub payment_method: model::PublishPaymentMethod,
    pub promo: Option<model::PublishPromo>,
    pub mileage_package: Option<model::MileagePackage>,
    pub taxes: Vec<model::Tax>,
    pub vehicle_snapshot_after: Option<model::VehicleSnapshot>,
}

#[derive(Serialize, Debug, Clone)]
pub struct TripInfo {
    pub agreement: model::Agreement,
    pub apartment_timezone: String,
    pub location_name: String,
    pub vehicle_name: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct FilePath {
    pub file_path: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct FileLink {
    pub file_link: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GenerateSnapshotRequest {
    pub vehicle_vin: String,
    pub left_image_path: String,
    pub right_image_path: String,
    pub front_image_path: String,
    pub back_image_path: String,
    pub front_right_image_path: String,
    pub front_left_image_path: String,
    pub back_right_image_path: String,
    pub back_left_image_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CheckOutRequest {
    pub agreement_id: i32,
    pub vehicle_snapshot_id: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CheckInRequest {
    pub agreement_id: i32,
    pub vehicle_snapshot_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NewAgreementRequest {
    pub vehicle_id: i32,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub start_time: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub end_time: DateTime<Utc>,
    pub payment_id: i32,
    pub liability: bool,
    pub pcdw: bool,
    pub pcdw_ext: bool,
    pub rsa: bool,
    pub pai: bool,
    pub rate_offer_id: Option<i32>,
    pub mileage_package_id: Option<i32>,
    pub promo_code: Option<String>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub hours_using_reward: Option<Decimal>,
}

#[derive(Serialize, Debug, Clone)]
pub struct RewardHoursSummaryResponse {
    #[serde(with = "rust_decimal::serde::str")]
    pub total: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub used: Decimal,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VeygoError {
    InternalServerError,
    RecordNotFound,
    TokenFormatError,
    InvalidToken,
    CardNotSupported,
    CardDeclined,
    CanNotCapture,
    CanNotRefund,
    InputDataError
}

impl std::fmt::Display for VeygoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VeygoError::InternalServerError => write!(f, "Internal Server Error"),
            VeygoError::RecordNotFound => write!(f, "Record Not Found"),
            VeygoError::TokenFormatError => write!(f, "Token Format Error"),
            VeygoError::InvalidToken => write!(f, "Invalid Token"),
            VeygoError::CardNotSupported => write!(f, "Card Not Supported"),
            VeygoError::CardDeclined => write!(f, "Card Declined"),
            VeygoError::CanNotRefund => write!(f, "Cannot Refund"),
            VeygoError::CanNotCapture => write!(f, "Cannot Capture"),
            VeygoError::InputDataError => write!(f, "Input Data Error"),
        }
    }
}

impl std::error::Error for VeygoError {}
