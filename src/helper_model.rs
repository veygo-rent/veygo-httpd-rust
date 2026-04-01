use chrono::{DateTime, NaiveDate, Utc};
use serde_derive::{Deserialize, Serialize};
use crate::{model};
use rust_decimal::prelude::*;

#[derive(Debug, Deserialize)]
pub struct TeslaChargingSessionsResponse {
    pub data: Vec<TeslaChargingSessionMin>,
    pub status_code: i32,
}

#[derive(Debug, Deserialize)]
pub struct TeslaChargingSessionMin {
    pub start_date_time: DateTime<Utc>,
    pub location: TeslaChargingLocationMin,
    pub total_cost: TeslaTotalCostMin,
}

#[derive(Debug, Deserialize)]
pub struct TeslaChargingLocationMin {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct TeslaTotalCostMin {
    pub excl_vat: f64,
    pub incl_vat: f64,
}


#[derive(Debug, Deserialize)]
pub struct TeslaVehicleDataEnvelope {
    pub response: TeslaVehicleData,
}

#[derive(Debug, Deserialize)]
pub struct TeslaVehicleData {
    pub charge_state: TeslaChargeState,
    pub vehicle_state: TeslaVehicleState,
    pub drive_state: TeslaDriveState,
}

#[derive(Debug, Deserialize)]
pub struct TeslaChargeState {
    pub battery_level: i32,
}

#[derive(Debug, Deserialize)]
pub struct TeslaVehicleState {
    pub odometer: f64,
}

#[derive(Debug, Deserialize)]
pub struct TeslaDriveState {
    pub latitude: f64,
    pub longitude: f64,
}

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
#[serde(tag = "type")]
pub enum CheckInOutRequest {
    #[serde(rename = "with_snapshot_id")]
    WithSnapshotId {
        agreement_id: i32,
        vehicle_snapshot_id: i32,
    },
    #[serde(rename = "with_image_path")]
    WithImagePath {
        agreement_id: i32,
        left_image_path: String,
        right_image_path: String,
        front_image_path: String,
        back_image_path: String,
        front_right_image_path: String,
        front_left_image_path: String,
        back_right_image_path: String,
        back_left_image_path: String,
    },
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum VerifyDriversLicenseRequest {
    #[serde(rename = "decline_primary")]
    DeclinePrimary {
        renter_id: i32,
        reason: String,
    },
    #[serde(rename = "decline_secondary")]
    DeclineSecondary {
        renter_id: i32,
        reason: String,
    },
    #[serde(rename = "require_secondary")]
    RequireSecondary {
        renter_id: i32,
        reason: String,
        drivers_license_number: Option<String>,
        drivers_license_state_region: Option<String>,
    },
    #[serde(rename = "approved")]
    Approved {
        renter_id: i32,
        drivers_license_number: Option<String>,
        drivers_license_state_region: Option<String>,
        drivers_license_expiration: NaiveDate,
        renter_address: Option<model::UsAddress>,
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum VerifyLeaseRequest {
    #[serde(rename = "declined")]
    Declined {
        renter_id: i32,
        reason: String,
    },
    #[serde(rename = "approved")]
    Approved {
        renter_id: i32,
        lease_expiration: NaiveDate,
        renter_address: model::UsAddress,
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum VerifyInsuranceRequest {
    #[serde(rename = "declined")]
    Declined {
        renter_id: i32,
        reason: String,
    },
    #[serde(rename = "approved")]
    Approved {
        renter_id: i32,
        insurance_liability_expiration: NaiveDate,
        insurance_collision_valid: bool,
    }
}

#[derive(Serialize)]
pub struct RenterNeedVerify {
    pub renter: model::PublishRenter,
    pub file_link: FileLink,
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
    pub rate_offer_id: i32,
    pub mileage_package_id: Option<i32>,
    pub promo_code: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub hours_using_reward: Decimal,
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
