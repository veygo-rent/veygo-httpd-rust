// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "agreement_status_enum"))]
    pub struct AgreementStatusEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "audit_action_enum"))]
    pub struct AuditActionEnum;

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
    #[diesel(postgres_type(name = "policy_enum"))]
    pub struct PolicyEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "remote_mgmt_enum"))]
    pub struct RemoteMgmtEnum;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "tax_type_enum"))]
    pub struct TaxTypeEnum;

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
        #[max_length = 8]
        confirmation -> Varchar,
        status -> AgreementStatusEnum,
        #[max_length = 26]
        user_name -> Varchar,
        user_date_of_birth -> Date,
        #[max_length = 36]
        user_email -> Varchar,
        #[max_length = 10]
        user_phone -> Varchar,
        #[max_length = 128]
        user_billing_address -> Varchar,
        rsvp_pickup_time -> Timestamptz,
        rsvp_drop_off_time -> Timestamptz,
        liability_protection_rate -> Nullable<Float8>,
        pcdw_protection_rate -> Nullable<Float8>,
        pcdw_ext_protection_rate -> Nullable<Float8>,
        rsa_protection_rate -> Nullable<Float8>,
        pai_protection_rate -> Nullable<Float8>,
        actual_pickup_time -> Nullable<Timestamptz>,
        actual_drop_off_time -> Nullable<Timestamptz>,
        msrp_factor -> Float8,
        duration_rate -> Float8,
        vehicle_id -> Int4,
        renter_id -> Int4,
        payment_method_id -> Int4,
        vehicle_snapshot_before -> Nullable<Int4>,
        vehicle_snapshot_after -> Nullable<Int4>,
        #[max_length = 16]
        promo_id -> Nullable<Varchar>,
        manual_discount -> Nullable<Float8>,
        location_id -> Int4,
        mileage_package_id -> Nullable<Int4>,
        mileage_conversion -> Float8,
        mileage_rate_overwrite -> Nullable<Float8>,
        mileage_package_overwrite -> Nullable<Float8>,
        utilization_factor -> Float8,
    }
}

diesel::table! {
    agreements_damages (agreement_id, damage_id) {
        agreement_id -> Int4,
        damage_id -> Int4,
    }
}

diesel::table! {
    agreements_taxes (agreement_id, tax_id) {
        agreement_id -> Int4,
        tax_id -> Int4,
    }
}

diesel::table! {
    apartments (id) {
        id -> Int4,
        #[max_length = 26]
        name -> Varchar,
        #[max_length = 36]
        timezone -> Varchar,
        #[max_length = 36]
        email -> Varchar,
        #[max_length = 10]
        phone -> Varchar,
        #[max_length = 128]
        address -> Varchar,
        #[max_length = 16]
        accepted_school_email_domain -> Varchar,
        free_tier_hours -> Float8,
        silver_tier_hours -> Nullable<Float8>,
        silver_tier_rate -> Nullable<Float8>,
        gold_tier_hours -> Nullable<Float8>,
        gold_tier_rate -> Nullable<Float8>,
        platinum_tier_hours -> Nullable<Float8>,
        platinum_tier_rate -> Nullable<Float8>,
        duration_rate -> Float8,
        liability_protection_rate -> Nullable<Float8>,
        pcdw_protection_rate -> Nullable<Float8>,
        pcdw_ext_protection_rate -> Nullable<Float8>,
        rsa_protection_rate -> Nullable<Float8>,
        pai_protection_rate -> Nullable<Float8>,
        is_operating -> Bool,
        is_public -> Bool,
        uni_id -> Nullable<Int4>,
        mileage_rate_overwrite -> Nullable<Float8>,
        mileage_package_overwrite -> Nullable<Float8>,
        mileage_conversion -> Float8,
    }
}

diesel::table! {
    apartments_taxes (apartment_id, tax_id) {
        apartment_id -> Int4,
        tax_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::AuditActionEnum;

    audits (id) {
        id -> Int4,
        renter_id -> Nullable<Int4>,
        action -> AuditActionEnum,
        #[max_length = 64]
        path -> Varchar,
        time -> Timestamptz,
    }
}

diesel::table! {
    charges (id) {
        id -> Int4,
        #[max_length = 64]
        name -> Varchar,
        time -> Timestamptz,
        amount -> Float8,
        note -> Nullable<Text>,
        agreement_id -> Nullable<Int4>,
        vehicle_id -> Int4,
        #[max_length = 32]
        checksum -> Varchar,
        transponder_company_id -> Nullable<Int4>,
        #[max_length = 26]
        vehicle_identifier -> Nullable<Varchar>,
    }
}

diesel::table! {
    claims (id) {
        id -> Int4,
        note -> Nullable<Text>,
        time -> Timestamptz,
        agreement_id -> Int4,
        admin_fee -> Nullable<Float8>,
        tow_charge -> Nullable<Float8>,
    }
}

diesel::table! {
    damage_submissions (id) {
        id -> Int4,
        reported_by -> Int4,
        #[max_length = 255]
        first_image -> Varchar,
        #[max_length = 255]
        second_image -> Varchar,
        #[max_length = 255]
        third_image -> Nullable<Varchar>,
        #[max_length = 255]
        fourth_image -> Nullable<Varchar>,
        description -> Text,
        processed_by -> Nullable<Int4>,
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
        #[max_length = 255]
        first_image -> Nullable<Varchar>,
        #[max_length = 255]
        second_image -> Nullable<Varchar>,
        #[max_length = 255]
        third_image -> Nullable<Varchar>,
        #[max_length = 255]
        fourth_image -> Nullable<Varchar>,
        fixed_date -> Nullable<Timestamptz>,
        fixed_amount -> Nullable<Float8>,
        depreciation -> Nullable<Float8>,
        lost_of_use -> Nullable<Float8>,
        claim_id -> Int4,
        vehicle_id -> Int4,
    }
}

diesel::table! {
    do_not_rent_lists (id) {
        id -> Int4,
        #[max_length = 26]
        name -> Nullable<Varchar>,
        #[max_length = 10]
        phone -> Nullable<Varchar>,
        #[max_length = 36]
        email -> Nullable<Varchar>,
        note -> Text,
        exp -> Nullable<Date>,
    }
}

diesel::table! {
    locations (id) {
        id -> Int4,
        apartment_id -> Int4,
        #[max_length = 64]
        name -> Varchar,
        description -> Nullable<Text>,
        latitude -> Float8,
        longitude -> Float8,
        is_operational -> Bool,
    }
}

diesel::table! {
    mileage_packages (id) {
        id -> Int4,
        miles -> Int4,
        discounted_rate -> Int4,
        is_active -> Bool,
    }
}

diesel::table! {
    payment_methods (id) {
        id -> Int4,
        #[max_length = 26]
        cardholder_name -> Varchar,
        #[max_length = 20]
        masked_card_number -> Varchar,
        #[max_length = 10]
        network -> Varchar,
        #[max_length = 10]
        expiration -> Varchar,
        #[max_length = 32]
        token -> Varchar,
        #[max_length = 32]
        fingerprint -> Varchar,
        #[max_length = 32]
        nickname -> Nullable<Varchar>,
        is_enabled -> Bool,
        renter_id -> Int4,
        last_used_date_time -> Nullable<Timestamptz>,
        cdw_enabled -> Bool,
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
        note -> Nullable<Text>,
        #[max_length = 18]
        reference_number -> Nullable<Varchar>,
        agreement_id -> Nullable<Int4>,
        renter_id -> Int4,
        payment_method_id -> Int4,
        amount_authorized -> Nullable<Float8>,
        capture_before -> Nullable<Timestamptz>,
        is_deposit -> Bool,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PolicyEnum;

    policies (id) {
        id -> Int4,
        policy_type -> PolicyEnum,
        policy_effective_date -> Date,
        content -> Text,
    }
}

diesel::table! {
    promos (code) {
        #[max_length = 16]
        code -> Varchar,
        #[max_length = 16]
        name -> Varchar,
        amount -> Float8,
        is_enabled -> Bool,
        is_one_time -> Bool,
        exp -> Timestamptz,
        user_id -> Nullable<Int4>,
        apt_id -> Nullable<Int4>,
        uni_id -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::GenderEnum;
    use super::sql_types::PlanTierEnum;
    use super::sql_types::EmployeeTierEnum;

    renters (id) {
        id -> Int4,
        #[max_length = 26]
        name -> Varchar,
        #[max_length = 22]
        stripe_id -> Nullable<Varchar>,
        #[max_length = 36]
        student_email -> Varchar,
        student_email_expiration -> Nullable<Date>,
        #[max_length = 72]
        password -> Varchar,
        #[max_length = 10]
        phone -> Varchar,
        phone_is_verified -> Bool,
        date_of_birth -> Date,
        #[max_length = 255]
        profile_picture -> Nullable<Varchar>,
        gender -> Nullable<GenderEnum>,
        date_of_registration -> Timestamptz,
        #[max_length = 26]
        drivers_license_number -> Nullable<Varchar>,
        #[max_length = 6]
        drivers_license_state_region -> Nullable<Varchar>,
        #[max_length = 255]
        drivers_license_image -> Nullable<Varchar>,
        #[max_length = 255]
        drivers_license_image_secondary -> Nullable<Varchar>,
        drivers_license_expiration -> Nullable<Date>,
        #[max_length = 255]
        insurance_id_image -> Nullable<Varchar>,
        insurance_liability_expiration -> Nullable<Date>,
        insurance_collision_expiration -> Nullable<Date>,
        #[max_length = 255]
        lease_agreement_image -> Nullable<Varchar>,
        apartment_id -> Int4,
        lease_agreement_expiration -> Nullable<Date>,
        #[max_length = 128]
        billing_address -> Nullable<Varchar>,
        #[max_length = 255]
        signature_image -> Nullable<Varchar>,
        signature_datetime -> Nullable<Timestamptz>,
        plan_tier -> PlanTierEnum,
        #[max_length = 2]
        plan_renewal_day -> Varchar,
        #[max_length = 6]
        plan_expire_month_year -> Varchar,
        plan_available_duration -> Float8,
        is_plan_annual -> Bool,
        employee_tier -> EmployeeTierEnum,
        subscription_payment_method_id -> Nullable<Int4>,
        #[max_length = 32]
        apple_apns -> Nullable<Varchar>,
        #[max_length = 32]
        admin_apple_apns -> Nullable<Varchar>,
    }
}

diesel::table! {
    reward_transactions (id) {
        id -> Int4,
        agreement_id -> Int4,
        duration -> Float8,
        transaction_time -> Timestamptz,
    }
}

diesel::table! {
    services (id) {
        id -> Int4,
        interval -> Int4,
        note -> Text,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::TaxTypeEnum;

    taxes (id) {
        id -> Int4,
        #[max_length = 32]
        name -> Varchar,
        multiplier -> Float8,
        is_effective -> Bool,
        is_sales_tax -> Bool,
        tax_type -> TaxTypeEnum,
    }
}

diesel::table! {
    transponder_companies (id) {
        id -> Int4,
        #[max_length = 18]
        name -> Varchar,
        #[max_length = 36]
        corresponding_key_for_vehicle_id -> Varchar,
        #[max_length = 36]
        corresponding_key_for_transaction_name -> Varchar,
        #[max_length = 18]
        custom_prefix_for_transaction_name -> Varchar,
        #[max_length = 36]
        corresponding_key_for_transaction_time -> Varchar,
        #[max_length = 36]
        corresponding_key_for_transaction_amount -> Varchar,
        #[max_length = 26]
        timestamp_format -> Varchar,
        #[max_length = 26]
        timezone -> Nullable<Varchar>,
    }
}

diesel::table! {
    vehicle_snapshots (id) {
        id -> Int4,
        #[max_length = 255]
        left_image -> Varchar,
        #[max_length = 255]
        right_image -> Varchar,
        #[max_length = 255]
        front_image -> Varchar,
        #[max_length = 255]
        back_image -> Varchar,
        time -> Timestamptz,
        odometer -> Int4,
        level -> Int4,
        vehicle_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::RemoteMgmtEnum;

    vehicles (id) {
        id -> Int4,
        #[max_length = 20]
        vin -> Varchar,
        #[max_length = 18]
        name -> Varchar,
        capacity -> Int4,
        doors -> Int4,
        small_bags -> Int4,
        large_bags -> Int4,
        carplay -> Bool,
        lane_keep -> Bool,
        available -> Bool,
        #[max_length = 10]
        license_number -> Varchar,
        #[max_length = 3]
        license_state -> Varchar,
        #[max_length = 4]
        year -> Varchar,
        #[max_length = 12]
        make -> Varchar,
        #[max_length = 12]
        model -> Varchar,
        msrp_factor -> Float8,
        #[max_length = 255]
        image_link -> Nullable<Varchar>,
        odometer -> Int4,
        tank_size -> Float8,
        tank_level_percentage -> Int4,
        #[max_length = 26]
        first_transponder_number -> Nullable<Varchar>,
        first_transponder_company_id -> Nullable<Int4>,
        #[max_length = 26]
        second_transponder_number -> Nullable<Varchar>,
        second_transponder_company_id -> Nullable<Int4>,
        #[max_length = 26]
        third_transponder_number -> Nullable<Varchar>,
        third_transponder_company_id -> Nullable<Int4>,
        #[max_length = 26]
        fourth_transponder_number -> Nullable<Varchar>,
        fourth_transponder_company_id -> Nullable<Int4>,
        location_id -> Int4,
        remote_mgmt -> RemoteMgmtEnum,
        #[max_length = 32]
        remote_mgmt_id -> Varchar,
        requires_own_insurance -> Bool,
        #[max_length = 4]
        admin_pin -> Nullable<Varchar>,
    }
}

diesel::table! {
    vehicles_services (vehicle_id, service_id) {
        vehicle_id -> Int4,
        service_id -> Int4,
        odometer -> Int4,
        #[max_length = 255]
        document -> Varchar,
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
        #[max_length = 8]
        code -> Varchar,
    }
}

diesel::joinable!(access_tokens -> renters (user_id));
diesel::joinable!(agreements -> locations (location_id));
diesel::joinable!(agreements -> mileage_packages (mileage_package_id));
diesel::joinable!(agreements -> payment_methods (payment_method_id));
diesel::joinable!(agreements -> promos (promo_id));
diesel::joinable!(agreements -> renters (renter_id));
diesel::joinable!(agreements -> vehicles (vehicle_id));
diesel::joinable!(agreements_damages -> agreements (agreement_id));
diesel::joinable!(agreements_damages -> damages (damage_id));
diesel::joinable!(agreements_taxes -> agreements (agreement_id));
diesel::joinable!(agreements_taxes -> taxes (tax_id));
diesel::joinable!(apartments_taxes -> apartments (apartment_id));
diesel::joinable!(apartments_taxes -> taxes (tax_id));
diesel::joinable!(audits -> renters (renter_id));
diesel::joinable!(charges -> agreements (agreement_id));
diesel::joinable!(charges -> transponder_companies (transponder_company_id));
diesel::joinable!(charges -> vehicles (vehicle_id));
diesel::joinable!(claims -> agreements (agreement_id));
diesel::joinable!(damage_submissions -> agreements (reported_by));
diesel::joinable!(damage_submissions -> renters (processed_by));
diesel::joinable!(damages -> claims (claim_id));
diesel::joinable!(damages -> vehicles (vehicle_id));
diesel::joinable!(locations -> apartments (apartment_id));
diesel::joinable!(payment_methods -> renters (renter_id));
diesel::joinable!(payments -> agreements (agreement_id));
diesel::joinable!(payments -> payment_methods (payment_method_id));
diesel::joinable!(payments -> renters (renter_id));
diesel::joinable!(promos -> renters (user_id));
diesel::joinable!(renters -> apartments (apartment_id));
diesel::joinable!(reward_transactions -> agreements (agreement_id));
diesel::joinable!(vehicle_snapshots -> vehicles (vehicle_id));
diesel::joinable!(vehicles -> locations (location_id));
diesel::joinable!(vehicles_services -> services (service_id));
diesel::joinable!(vehicles_services -> vehicles (vehicle_id));
diesel::joinable!(verifications -> renters (renter_id));

diesel::allow_tables_to_appear_in_same_query!(
    access_tokens,
    agreements,
    agreements_damages,
    agreements_taxes,
    apartments,
    apartments_taxes,
    audits,
    charges,
    claims,
    damage_submissions,
    damages,
    do_not_rent_lists,
    locations,
    mileage_packages,
    payment_methods,
    payments,
    policies,
    promos,
    renters,
    reward_transactions,
    services,
    taxes,
    transponder_companies,
    vehicle_snapshots,
    vehicles,
    vehicles_services,
    verifications,
);
