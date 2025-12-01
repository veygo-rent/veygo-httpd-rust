drop index if exists policies_policy_type_idx;
drop index if exists policies_policy_effective_date_idx;
drop table if exists audits;

drop table if exists policies;

drop table if exists agreements_taxes;
drop table if exists apartments_taxes;
drop table if exists agreements_damages;
drop table if exists vehicles_services;

drop table if exists reward_transactions;

drop table if exists payments;

drop table if exists damages;

drop table if exists claims;

drop table if exists charges;

drop index if exists taxes_name_idx;
drop table if exists taxes;

drop index if exists verifications_code_idx;
drop table if exists verifications;

drop table if exists damage_submissions;

drop index if exists agreements_status_idx;
drop index if exists agreements_rsvp_pickup_time_idx;
drop index if exists agreements_actual_drop_off_time_idx;
drop table if exists agreements;

drop table if exists vehicle_snapshots;

drop table if exists mileage_packages;

drop table if exists promos;

drop index if exists vehicles_license_idx;
drop table if exists vehicles;

drop table if exists services;

drop table if exists locations;

drop index if exists payment_methods_fingerprint_enabled_uk;
drop table if exists payment_methods;

drop index if exists access_tokens_token_idx;
drop table if exists access_tokens;

drop index if exists renters_name_idx;
drop table if exists renters;

drop index if exists apartments_name_idx;
drop index if exists apartments_email_idx;
drop index if exists apartments_phone_idx;
drop table if exists apartments;

drop index if exists transponder_companies_name_idx;
drop table if exists transponder_companies;

drop index if exists do_not_rent_lists_name_idx;
drop index if exists do_not_rent_lists_phone_idx;
drop index if exists do_not_rent_lists_email_idx;
drop table if exists do_not_rent_lists;

drop type if exists gender_enum;
drop type if exists plan_tier_enum;
drop type if exists employee_tier_enum;
drop type if exists verification_type_enum;
drop type if exists remote_mgmt_enum;
drop type if exists agreement_status_enum;
drop type if exists payment_type_enum;
drop type if exists audit_action_enum;
drop type if exists policy_enum;
drop type if exists tax_type_enum;