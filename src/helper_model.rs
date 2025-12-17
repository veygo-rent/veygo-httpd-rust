use serde_derive::{Deserialize, Serialize};
use crate::model;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ErrorResponse {
    pub title: String,
    pub message: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CurrentTrip {
    pub agreement: model::Agreement,
    pub vehicle: model::PublishRenterVehicle,
    pub apartment: model::Apartment,
    pub location: model::Location,
    pub vehicle_snapshot_before: Option<model::VehicleSnapshot>,
    pub payment_method: model::PublishPaymentMethod,
    pub promo: Option<model::PublishPromo>,
    pub mileage_package: Option<model::MileagePackage>,
    pub damages: Vec<model::Damage>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FilePath {
    pub file_path: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FileLink {
    pub file_link: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CheckOutRequest {
    pub agreement_id: i32,
    pub vehicle_snapshot_id: i32,
    pub hours_using_reward: i32,
}