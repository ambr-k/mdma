UPDATE payments
SET
    payment_method = 'webconnex'
WHERE
    platform = 'webconnex';

ALTER TABLE payments DROP COLUMN platform;