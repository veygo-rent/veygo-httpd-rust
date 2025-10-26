drop table if exists agreements_taxes;
drop table if exists apartments_taxes;
drop table if exists charges_taxes;
drop table if exists agreements_damages;

drop table if exists vehicle_snapshots;

drop table if exists reward_transactions;

drop table if exists payments;

drop table if exists damages;

drop table if exists charges;

drop index if exists taxes_name_idx;
drop table if exists taxes;

drop index if exists verifications_code_idx;
drop table if exists verifications;

drop table if exists damage_submissions;

drop table if exists agreements;

drop table if exists promos;

drop index if exists vehicles_license_idx;
drop table if exists vehicles;

drop table if exists locations;

drop table if exists payment_methods;

drop table if exists access_tokens;

drop index if exists renters_name_idx;
drop table if exists renters;

drop index if exists apartments_name_idx;
drop index if exists apartments_email_idx;
drop index if exists apartments_phone_idx;
drop table if exists apartments;

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