// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "agreement_status_enum"))]
    pub struct AgreementStatusEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "employee_tier_enum"))]
    pub struct EmployeeTierEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "gender_enum"))]
    pub struct GenderEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "payment_type_enum"))]
    pub struct PaymentTypeEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "plan_tier_enum"))]
    pub struct PlanTierEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "remote_mgmt_enum"))]
    pub struct RemoteMgmtEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "transaction_type_enum"))]
    pub struct TransactionTypeEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "verification_type_enum"))]
    pub struct VerificationTypeEnum;
}

diesel::table! {
    access_tokens (id) {
        id -> Int4,
        user_id -> Int4,
        token -> Bytea,
        exp -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AgreementStatusEnum;

    agreements (id) {
        id -> Int4,
        confirmation -> Varchar,
        status -> AgreementStatusEnum,
        user_name -> Varchar,
        user_date_of_birth -> Date,
        user_email -> Varchar,
        user_phone -> Varchar,
        user_billing_address -> Varchar,
        rsvp_pickup_time -> Timestamptz,
        rsvp_drop_off_time -> Timestamptz,
        liability_protection_rate -> Float8,
        pcdw_protection_rate -> Float8,
        pcdw_ext_protection_rate -> Float8,
        rsa_protection_rate -> Float8,
        pai_protection_rate -> Float8,
        actual_pickup_time -> Nullable<Timestamptz>,
        pickup_odometer -> Nullable<Int4>,
        pickup_level -> Nullable<Int4>,
        actual_drop_off_time -> Nullable<Timestamptz>,
        drop_off_odometer -> Nullable<Int4>,
        drop_off_level -> Nullable<Int4>,
        msrp_factor -> Float8,
        duration_rate -> Float8,
        apartment_id -> Int4,
        vehicle_id -> Int4,
        renter_id -> Int4,
        payment_method_id -> Int4,
        damage_ids -> Array<Nullable<Int4>>,
        vehicle_snapshot_before -> Nullable<Int4>,
        vehicle_snapshot_after -> Nullable<Int4>,
        promo_id -> Nullable<Int4>,
        taxes -> Array<Nullable<Int4>>,
        location_id -> Int4,
    }
}

diesel::table! {
    apartments (id) {
        id -> Int4,
        name -> Varchar,
        email -> Varchar,
        phone -> Varchar,
        address -> Varchar,
        accepted_school_email_domain -> Varchar,
        free_tier_hours -> Float8,
        free_tier_rate -> Float8,
        silver_tier_hours -> Float8,
        silver_tier_rate -> Float8,
        gold_tier_hours -> Float8,
        gold_tier_rate -> Float8,
        platinum_tier_hours -> Float8,
        platinum_tier_rate -> Float8,
        duration_rate -> Float8,
        liability_protection_rate -> Float8,
        pcdw_protection_rate -> Float8,
        pcdw_ext_protection_rate -> Float8,
        rsa_protection_rate -> Float8,
        pai_protection_rate -> Float8,
        is_operating -> Bool,
        is_public -> Bool,
        uni_id -> Int4,
        taxes -> Array<Nullable<Int4>>,
    }
}

diesel::table! {
    charges (id) {
        id -> Int4,
        name -> Varchar,
        time -> Timestamptz,
        amount -> Float8,
        note -> Nullable<Varchar>,
        agreement_id -> Nullable<Int4>,
        vehicle_id -> Int4,
        checksum -> Varchar,
        transponder_company_id -> Nullable<Int4>,
        vehicle_identifier -> Nullable<Varchar>,
        taxes -> Array<Nullable<Int4>>,
    }
}

diesel::table! {
    damage_submissions (id) {
        id -> Int4,
        reported_by -> Int4,
        first_image -> Varchar,
        second_image -> Varchar,
        third_image -> Nullable<Varchar>,
        fourth_image -> Nullable<Varchar>,
        description -> Varchar,
        processed -> Bool,
    }
}

diesel::table! {
    damages (id) {
        id -> Int4,
        note -> Text,
        record_date -> Timestamptz,
        occur_date -> Timestamptz,
        standard_coordination_x_percentage -> Int4,
        standard_coordination_y_percentage -> Int4,
        first_image -> Nullable<Varchar>,
        second_image -> Nullable<Varchar>,
        third_image -> Nullable<Varchar>,
        fourth_image -> Nullable<Varchar>,
        fixed_date -> Nullable<Timestamptz>,
        fixed_amount -> Nullable<Float8>,
        agreement_id -> Nullable<Int4>,
    }
}

diesel::table! {
    do_not_rent_lists (id) {
        id -> Int4,
        name -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        note -> Text,
        exp -> Nullable<Date>,
    }
}

diesel::table! {
    locations (id) {
        id -> Int4,
        apartment_id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        latitude -> Float8,
        longitude -> Float8,
        enabled -> Bool,
    }
}

diesel::table! {
    payment_methods (id) {
        id -> Int4,
        cardholder_name -> Varchar,
        masked_card_number -> Varchar,
        network -> Varchar,
        expiration -> Varchar,
        token -> Varchar,
        md5 -> Varchar,
        nickname -> Nullable<Varchar>,
        is_enabled -> Bool,
        renter_id -> Int4,
        last_used_date_time -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PaymentTypeEnum;

    payments (id) {
        id -> Int4,
        payment_type -> PaymentTypeEnum,
        time -> Timestamptz,
        amount -> Float8,
        note -> Nullable<Varchar>,
        reference_number -> Nullable<Varchar>,
        agreement_id -> Nullable<Int4>,
        renter_id -> Int4,
        payment_method_id -> Int4,
        amount_authorized -> Nullable<Float8>,
        capture_before -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    promos (code) {
        code -> Varchar,
        name -> Varchar,
        amount -> Float8,
        is_enabled -> Bool,
        is_one_time -> Bool,
        exp -> Timestamptz,
        user_id -> Int4,
        apt_id -> Int4,
        uni_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TransactionTypeEnum;

    rental_transactions (id) {
        id -> Int4,
        agreement_id -> Int4,
        transaction_type -> TransactionTypeEnum,
        duration -> Float8,
        transaction_time -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::GenderEnum;
    use super::sql_types::PlanTierEnum;
    use super::sql_types::EmployeeTierEnum;

    renters (id) {
        id -> Int4,
        name -> Varchar,
        stripe_id -> Nullable<Varchar>,
        student_email -> Varchar,
        student_email_expiration -> Nullable<Date>,
        password -> Varchar,
        phone -> Varchar,
        phone_is_verified -> Bool,
        date_of_birth -> Date,
        profile_picture -> Nullable<Varchar>,
        gender -> Nullable<GenderEnum>,
        date_of_registration -> Timestamptz,
        drivers_license_number -> Nullable<Varchar>,
        drivers_license_state_region -> Nullable<Varchar>,
        drivers_license_image -> Nullable<Varchar>,
        drivers_license_image_secondary -> Nullable<Varchar>,
        drivers_license_expiration -> Nullable<Date>,
        insurance_id_image -> Nullable<Varchar>,
        insurance_liability_expiration -> Nullable<Date>,
        insurance_collision_expiration -> Nullable<Date>,
        lease_agreement_image -> Nullable<Varchar>,
        apartment_id -> Int4,
        lease_agreement_expiration -> Nullable<Date>,
        billing_address -> Nullable<Varchar>,
        signature_image -> Nullable<Varchar>,
        signature_datetime -> Nullable<Timestamptz>,
        plan_tier -> PlanTierEnum,
        plan_renewal_day -> Varchar,
        plan_expire_month_year -> Varchar,
        plan_available_duration -> Float8,
        is_plan_annual -> Bool,
        employee_tier -> EmployeeTierEnum,
        subscription_payment_method_id -> Nullable<Int4>,
        apple_apns -> Nullable<Varchar>,
        admin_apple_apns -> Nullable<Varchar>,
    }
}

diesel::table! {
    taxes (id) {
        id -> Int4,
        name -> Varchar,
        multiplier -> Float8,
        is_effective -> Bool,
    }
}

diesel::table! {
    transponder_companies (id) {
        id -> Int4,
        name -> Varchar,
        corresponding_key_for_vehicle_id -> Varchar,
        corresponding_key_for_transaction_name -> Varchar,
        custom_prefix_for_transaction_name -> Varchar,
        corresponding_key_for_transaction_time -> Varchar,
        corresponding_key_for_transaction_amount -> Varchar,
        timestamp_format -> Varchar,
        timezone -> Nullable<Varchar>,
    }
}

diesel::table! {
    vehicle_snapshots (id) {
        id -> Int4,
        left_image -> Varchar,
        right_image -> Varchar,
        front_image -> Varchar,
        back_image -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::RemoteMgmtEnum;

    vehicles (id) {
        id -> Int4,
        vin -> Varchar,
        name -> Varchar,
        available -> Bool,
        license_number -> Varchar,
        license_state -> Varchar,
        year -> Varchar,
        make -> Varchar,
        model -> Varchar,
        msrp_factor -> Float8,
        image_link -> Nullable<Varchar>,
        odometer -> Int4,
        tank_size -> Float8,
        tank_level_percentage -> Int4,
        first_transponder_number -> Nullable<Varchar>,
        first_transponder_company_id -> Nullable<Int4>,
        second_transponder_number -> Nullable<Varchar>,
        second_transponder_company_id -> Nullable<Int4>,
        third_transponder_number -> Nullable<Varchar>,
        third_transponder_company_id -> Nullable<Int4>,
        fourth_transponder_number -> Nullable<Varchar>,
        fourth_transponder_company_id -> Nullable<Int4>,
        location_id -> Int4,
        remote_mgmt -> RemoteMgmtEnum,
        #[max_length = 255]
        remote_mgmt_id -> Varchar,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::VerificationTypeEnum;

    verifications (id) {
        id -> Int4,
        verification_method -> VerificationTypeEnum,
        renter_id -> Int4,
        expires_at -> Timestamptz,
        code -> Varchar,
    }
}

diesel::joinable!(access_tokens -> renters (user_id));
diesel::joinable!(agreements -> apartments (apartment_id));
diesel::joinable!(agreements -> locations (location_id));
diesel::joinable!(agreements -> payment_methods (payment_method_id));
diesel::joinable!(agreements -> renters (renter_id));
diesel::joinable!(agreements -> vehicles (vehicle_id));
diesel::joinable!(charges -> agreements (agreement_id));
diesel::joinable!(charges -> vehicles (vehicle_id));
diesel::joinable!(damage_submissions -> renters (reported_by));
diesel::joinable!(damages -> agreements (agreement_id));
diesel::joinable!(locations -> apartments (apartment_id));
diesel::joinable!(payment_methods -> renters (renter_id));
diesel::joinable!(payments -> agreements (agreement_id));
diesel::joinable!(payments -> payment_methods (payment_method_id));
diesel::joinable!(payments -> renters (renter_id));
diesel::joinable!(rental_transactions -> agreements (agreement_id));
diesel::joinable!(renters -> apartments (apartment_id));
diesel::joinable!(vehicles -> locations (location_id));
diesel::joinable!(verifications -> renters (renter_id));

diesel::allow_tables_to_appear_in_same_query!(
    access_tokens,
    agreements,
    apartments,
    charges,
    damage_submissions,
    damages,
    do_not_rent_lists,
    locations,
    payment_methods,
    payments,
    promos,
    rental_transactions,
    renters,
    taxes,
    transponder_companies,
    vehicle_snapshots,
    vehicles,
    verifications,
);
