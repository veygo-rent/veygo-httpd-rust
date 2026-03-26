use rust_decimal::Decimal;

#[allow(dead_code)]
pub static RSVP_BUFFER: i64 = 15;
#[allow(dead_code)]
pub static DEPOSIT_AMOUNT: i64 = 200;

#[allow(dead_code)]
pub const PRICE_PER_CENT_ON_GAS: Decimal = Decimal::from_parts(150, 0, 0, false, 2);

#[allow(dead_code)]
pub static MIN_IOS_VERSION: &str = "1.0.1";
#[allow(dead_code)]
pub static MIN_ANDROID_VERSION: &str = "1.0.1";