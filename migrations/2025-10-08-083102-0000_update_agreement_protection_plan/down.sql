ALTER TABLE agreements
    ALTER COLUMN liability_protection_rate SET NOT NULL,
ALTER COLUMN pcdw_protection_rate SET NOT NULL,
    ALTER COLUMN pcdw_ext_protection_rate SET NOT NULL,
    ALTER COLUMN rsa_protection_rate SET NOT NULL,
    ALTER COLUMN pai_protection_rate SET NOT NULL;