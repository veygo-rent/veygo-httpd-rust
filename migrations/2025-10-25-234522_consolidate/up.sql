create type gender_enum as enum ('Male', 'Female', 'Other', 'PNTS');
create type plan_tier_enum as enum ('Free', 'Silver', 'Gold', 'Platinum');
create type employee_tier_enum as enum ('User', 'GeneralEmployee', 'Maintenance', 'Admin');
create type verification_type_enum as enum ('email', 'phone');
create type remote_mgmt_enum as enum ('revers',  'geotab', 'smartcar', 'tesla', 'none');
create type agreement_status_enum as enum ('Rental', 'Void', 'Canceled');
create type payment_type_enum as enum ('canceled', 'processing', 'requires_action', 'requires_capture', 'requires_confirmation', 'requires_payment_method', 'succeeded');

create table do_not_rent_lists
(
    id    serial,
    name  varchar(26),
    phone varchar(10),
    email varchar(36),
    note  text not null,
    exp   date,
    constraint do_not_rent_lists_pk primary key (id)
);

create index do_not_rent_lists_name_idx
    on do_not_rent_lists (name);
create index do_not_rent_lists_phone_idx
    on do_not_rent_lists (phone);
create index do_not_rent_lists_email_idx
    on do_not_rent_lists (email);

create table transponder_companies
(
    id                                       serial,
    name                                     varchar(18) not null,
    corresponding_key_for_vehicle_id         varchar(36) not null,
    corresponding_key_for_transaction_name   varchar(36) not null,
    custom_prefix_for_transaction_name       varchar(18) not null,
    corresponding_key_for_transaction_time   varchar(36) not null,
    corresponding_key_for_transaction_amount varchar(36) not null,
    timestamp_format                         varchar(26) not null,
    timezone                                 varchar(26),
    constraint transponder_companies_pk primary key (id),
    constraint transponder_companies_name_uk unique (name)
);

create table apartments
(
    id                           serial,
    name                         varchar(26)      not null,
    email                        varchar(36)      not null,
    phone                        varchar(10)      not null,
    address                      varchar(128)      not null,
    accepted_school_email_domain varchar(16)      not null,
    free_tier_hours              double precision not null,
    silver_tier_hours            double precision not null,
    silver_tier_rate             double precision not null,
    gold_tier_hours              double precision not null,
    gold_tier_rate               double precision not null,
    platinum_tier_hours          double precision not null,
    platinum_tier_rate           double precision not null,
    duration_rate                double precision not null,
    liability_protection_rate    double precision,
    pcdw_protection_rate         double precision,
    pcdw_ext_protection_rate     double precision,
    rsa_protection_rate          double precision,
    pai_protection_rate          double precision,
    is_operating                 boolean          not null,
    is_public                    boolean          not null,
    uni_id                       integer,
    constraint apartments_pk primary key (id),
    constraint apartments_name_uk unique (name),
    constraint apartments_uni_id_fk foreign key (uni_id) references apartments(id)
);

create index apartments_name_idx
    on apartments (name);
create index apartments_email_idx
    on apartments (email);
create index apartments_phone_idx
    on apartments (phone);

create table renters
(
    id                              serial,
    name                            varchar(26)                                                 not null,
    stripe_id                       varchar(22),
    student_email                   varchar(36)                                                 not null,
    student_email_expiration        date,
    password                        varchar(72)                                                 not null,
    phone                           varchar(10)                                                 not null,
    phone_is_verified               boolean                  default false                      not null,
    date_of_birth                   date                                                        not null,
    profile_picture                 varchar(255),
    gender                          gender_enum,
    date_of_registration            timestamp with time zone default CURRENT_TIMESTAMP          not null,
    drivers_license_number          varchar(26),
    drivers_license_state_region    varchar(6),
    drivers_license_image           varchar(255),
    drivers_license_image_secondary varchar(255),
    drivers_license_expiration      date,
    insurance_id_image              varchar(255),
    insurance_liability_expiration  date,
    insurance_collision_expiration  date,
    lease_agreement_image           varchar(255),
    apartment_id                    integer                                                     not null,
    lease_agreement_expiration      date,
    billing_address                 varchar(128),
    signature_image                 varchar(255),
    signature_datetime              timestamp with time zone,
    plan_tier                       plan_tier_enum           default 'Free'::plan_tier_enum     not null,
    plan_renewal_day                varchar(2)                                                 not null,
    plan_expire_month_year          varchar(6)                                                  not null,
    plan_available_duration         double precision                                            not null,
    is_plan_annual                  boolean                  default false                      not null,
    employee_tier                   employee_tier_enum       default 'User'::employee_tier_enum not null,
    subscription_payment_method_id  integer,
    apple_apns                      varchar(32),
    admin_apple_apns                varchar(32),
    constraint renters_pk primary key (id),
    constraint renters_student_email_uk unique (student_email),
    constraint renters_phone_uk unique (phone),
    constraint renters_apartment_id_fk foreign key (apartment_id) references apartments(id)
);

create index renters_name_idx
    on renters (name);

create table access_tokens
(
    id      serial,
    user_id integer                  not null,
    token   bytea                    not null,
    exp     timestamp with time zone not null,
    constraint access_tokens_pk primary key (id),
    constraint access_tokens_user_id_fk foreign key (user_id) references renters(id)
);

create table payment_methods
(
    id                  serial,
    cardholder_name     varchar(26)           not null,
    masked_card_number  varchar(20)           not null,
    network             varchar(10)           not null,
    expiration          varchar(10)           not null,
    token               varchar(32)           not null,
    fingerprint         varchar(32)           not null,
    nickname            varchar(32),
    is_enabled          boolean               not null,
    renter_id           integer               not null,
    last_used_date_time timestamp with time zone,
    cdw_enabled         boolean default false not null,
    constraint payment_methods_pk primary key (id),
    constraint payment_methods_fingerprint_uk unique (fingerprint),
    constraint payment_methods_renter_id_fk foreign key (renter_id) references renters(id)
);

create table locations
(
    id             serial,
    apartment_id   integer              not null,
    name           varchar(64)          not null,
    description    text,
    latitude       double precision     not null,
    longitude      double precision     not null,
    is_operational boolean default true not null,
    constraint locations_pk primary key (id),
    constraint locations_apartment_id_fk foreign key (apartment_id) references apartments(id)
);

create table vehicles
(
    id                            serial,
    vin                           varchar(20)                                       not null,
    name                          varchar(18)                                       not null,
    available                     boolean                                           not null,
    license_number                varchar(10)                                       not null,
    license_state                 varchar(3)                                        not null,
    year                          varchar(4)                                        not null,
    make                          varchar(12)                                       not null,
    model                         varchar(12)                                       not null,
    msrp_factor                   double precision                                  not null,
    image_link                    varchar(255),
    odometer                      integer                                           not null,
    tank_size                     double precision                                  not null,
    tank_level_percentage         integer                                           not null,
    first_transponder_number      varchar(26),
    first_transponder_company_id  integer,
    second_transponder_number     varchar(26),
    second_transponder_company_id integer,
    third_transponder_number      varchar(26),
    third_transponder_company_id  integer,
    fourth_transponder_number     varchar(26),
    fourth_transponder_company_id integer,
    location_id                   integer                                           not null,
    remote_mgmt                   remote_mgmt_enum default 'none'::remote_mgmt_enum not null,
    remote_mgmt_id                varchar(32)     default ''::character varying    not null,
    requires_own_insurance        boolean                                           not null,
    constraint vehicles_pk primary key (id),
    constraint vehicles_vin_uk unique (vin),
    constraint vehicles_location_id_fk foreign key (location_id) references locations(id)
);

create index vehicles_license_idx
    on vehicles (license_number, license_state);

create table promos
(
    code        varchar(16)              not null,
    name        varchar(16)              not null,
    amount      double precision         not null,
    is_enabled  boolean default true     not null,
    is_one_time boolean default false    not null,
    exp         timestamp with time zone not null,
    user_id     integer default 0        not null,
    apt_id      integer default 0        not null,
    uni_id      integer default 0        not null,
    constraint promos_pk primary key (code)
);

create table agreements
(
    id                        serial,
    confirmation              varchar(8)                            not null,
    status                    agreement_status_enum                 not null,
    user_name                 varchar(26)                           not null,
    user_date_of_birth        date                                  not null,
    user_email                varchar(36)                           not null,
    user_phone                varchar(10)                           not null,
    user_billing_address      varchar(128)                          not null,
    rsvp_pickup_time          timestamp with time zone              not null,
    rsvp_drop_off_time        timestamp with time zone              not null,
    liability_protection_rate double precision,
    pcdw_protection_rate      double precision,
    pcdw_ext_protection_rate  double precision,
    rsa_protection_rate       double precision,
    pai_protection_rate       double precision,
    actual_pickup_time        timestamp with time zone,
    actual_drop_off_time      timestamp with time zone,
    msrp_factor               double precision                      not null,
    duration_rate             double precision                      not null,
    vehicle_id                integer                               not null,
    renter_id                 integer                               not null,
    payment_method_id         integer                               not null,
    vehicle_snapshot_before   integer,
    vehicle_snapshot_after    integer,
    promo_id                  varchar(16),
    manual_discount           double precision,
    location_id               integer                               not null,
    constraint agreements_pk primary key (id),
    constraint agreements_confirmation_uk unique (confirmation),
    constraint agreements_vehicle_id_fk foreign key (vehicle_id) references vehicles(id),
    constraint agreements_renter_id_fk foreign key (renter_id) references renters(id),
    constraint agreements_payment_method_id_fk foreign key (payment_method_id) references payment_methods(id),
    constraint agreements_location_id_fk foreign key (location_id) references locations(id),
    constraint agreements_promo_id_fk foreign key (promo_id) references promos(code)
);

create table damage_submissions
(
    id           serial,
    reported_by  integer               not null,
    first_image  varchar(255)          not null,
    second_image varchar(255)          not null,
    third_image  varchar(255),
    fourth_image varchar(255),
    description  text                  not null,
    processed_by integer,
    constraint damage_submissions_pk primary key (id),
    constraint damage_submissions_reported_by_fk foreign key (reported_by) references agreements (id),
    constraint damage_submissions_processed_by_fk foreign key (processed_by) references renters (id)
);

create table verifications
(
    id                  serial,
    verification_method verification_type_enum                                                      not null,
    renter_id           integer                                                                     not null,
    expires_at          timestamp with time zone default (CURRENT_TIMESTAMP + '00:10:00'::interval) not null,
    code                varchar(8)                                                                  not null,
    constraint verifications_pk primary key (id),
    constraint verifications_renter_id_fk foreign key (renter_id) references renters(id)
);

create index verifications_code_idx
    on verifications (code);

create table taxes
(
    id           serial,
    name         varchar(32)      not null,
    multiplier   double precision not null,
    is_effective boolean          not null,
    constraint taxes_pk primary key (id)
);

create index taxes_name_idx
    on taxes (name);

create table charges
(
    id                     serial,
    name                   varchar(64)              not null,
    time                   timestamp with time zone not null,
    amount                 double precision         not null,
    note                   text,
    agreement_id           integer,
    vehicle_id             integer                  not null,
    checksum               varchar(32)              not null,
    transponder_company_id integer,
    vehicle_identifier     varchar(26),
    constraint charges_pk primary key (id),
    constraint charges_checksum_uk unique (checksum),
    constraint charges_transponder_company_id_fk foreign key (transponder_company_id) references transponder_companies(id),
    constraint charges_vehicle_id_fk foreign key (vehicle_id) references vehicles(id),
    constraint charges_agreement_id_fk foreign key (agreement_id) references agreements(id)
);

create table damages
(
    id                                 serial,
    note                               text                     not null,
    record_date                        timestamp with time zone not null,
    occur_date                         timestamp with time zone not null,
    standard_coordination_x_percentage integer                  not null,
    standard_coordination_y_percentage integer                  not null,
    first_image                        varchar(255),
    second_image                       varchar(255),
    third_image                        varchar(255),
    fourth_image                       varchar(255),
    fixed_date                         timestamp with time zone,
    fixed_amount                       double precision,
    agreement_id                       integer,
    constraint damages_pk primary key (id),
    constraint damages_agreement_id_fk foreign key (agreement_id) references agreements(id)
);

create table payments
(
    id                serial,
    payment_type      payment_type_enum                                  not null,
    time              timestamp with time zone default CURRENT_TIMESTAMP not null,
    amount            double precision                                   not null,
    note              text,
    reference_number  varchar(18),
    agreement_id      integer,
    renter_id         integer                                            not null,
    payment_method_id integer                                            not null,
    amount_authorized double precision,
    capture_before    timestamp with time zone,
    is_deposit        boolean                                            not null,
    constraint payments_pk primary key (id),
    constraint payments_renter_id_fk foreign key (renter_id) references renters(id),
    constraint payments_payment_method_id_fk foreign key (payment_method_id) references payment_methods(id),
    constraint payments_agreement_id_fk foreign key (agreement_id) references agreements(id)
);

create table reward_transactions
(
    id               serial,
    agreement_id     integer                                            not null,
    duration         double precision                                   not null,
    transaction_time timestamp with time zone default CURRENT_TIMESTAMP not null,
    constraint reward_transactions_pk primary key (id),
    constraint reward_transactions_agreement_id_fk foreign key (agreement_id) references agreements(id)
);

create table vehicle_snapshots
(
    id          serial,
    left_image  varchar(255)                           not null,
    right_image varchar(255)                           not null,
    front_image varchar(255)                           not null,
    back_image  varchar(255)                           not null,
    time        timestamp with time zone default now() not null,
    odometer    integer                                not null,
    level       integer                                not null,
    vehicle_id  integer                                not null,
    constraint vehicle_snapshots_pk primary key (id),
    constraint vehicle_snapshots_vehicle_id_fk foreign key (vehicle_id) references vehicles(id)
);

create table agreements_taxes
(
    agreement_id integer not null,
    tax_id       integer not null,
    constraint agreements_taxes_pk primary key (agreement_id, tax_id),
    constraint agreements_taxes_agreement_id_fk foreign key (agreement_id) references agreements(id),
    constraint agreements_taxes_tax_id_fk foreign key (tax_id) references taxes(id)
);

create table apartments_taxes
(
    apartment_id integer not null,
    tax_id       integer not null,
    constraint apartments_taxes_pk primary key (apartment_id, tax_id),
    constraint apartments_taxes_apartment_id_fk foreign key (apartment_id) references apartments(id),
    constraint apartments_taxes_tax_id_fk foreign key (tax_id) references taxes(id)
);

create table charges_taxes
(
    charge_id integer not null,
    tax_id    integer not null,
    constraint charges_taxes_pk primary key (charge_id, tax_id),
    constraint charges_taxes_charge_id_fk foreign key (charge_id) references charges(id),
    constraint charges_taxes_tax_id_fk foreign key (tax_id) references taxes(id)
);

create table agreements_damages
(
    agreement_id integer not null,
    damage_id    integer not null,
    constraint agreements_damages_pk primary key (agreement_id, damage_id),
    constraint agreements_damages_agreement_id_fk foreign key (agreement_id) references agreements(id),
    constraint agreements_damages_damage_id_fk foreign key (damage_id) references damages(id)
);

insert into taxes (name, multiplier, is_effective)
values ('IN Sales Tax', 0.07, true),
       ('IN Car Rental Excise Tax', 0.04, true);

insert into apartments (name,
                        email,
                        phone,
                        address,
                        accepted_school_email_domain,
                        free_tier_hours,
                        silver_tier_hours, silver_tier_rate,
                        gold_tier_hours, gold_tier_rate,
                        platinum_tier_hours, platinum_tier_rate,
                        duration_rate,
                        liability_protection_rate,
                        pcdw_protection_rate,
                        pcdw_ext_protection_rate,
                        rsa_protection_rate,
                        pai_protection_rate,
                        is_operating,
                        is_public,
                        uni_id)
values ('Veygo HQ',
        'admin@veygo.rent',
        '8334683946',
        '101 Foundry Dr, Ste 1200, West Lafayette, IN 47906',
        'veygo.rent',
        0.0,
        0.0, 0.0,
        0.0, 0.0,
        0.0, 0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        TRUE,
        TRUE,
        NULL),
       ('Purdue University',
        'newres@purdue.edu',
        '7654944600',
        '610 Purdue Mall, West Lafayette, IN 47907',
        'purdue.edu',
        0.5,
        2.0, 71.99,
        5.0, 192.88,
        10.0, 305.49,
        10.5,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        TRUE,
        TRUE,
        1);

insert into apartments_taxes (apartment_id, tax_id)
values (1, 1),
       (1, 2),
       (2, 1),
       (2, 2);
