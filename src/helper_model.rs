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