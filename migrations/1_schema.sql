CREATE EXTENSION pgcrypto;

CREATE TABLE members (
    id SERIAL PRIMARY KEY,
    webconnex_id INT DEFAULT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    reason_removed TEXT DEFAULT NULL,
    created_on DATE DEFAULT NOW() NOT NULL
);

CREATE TABLE payments (
    id SERIAL PRIMARY KEY,
    member_id INT REFERENCES members (id) NOT NULL,
    created_on DATE DEFAULT NOW(),
    duration_months INT DEFAULT 1,
    amount_paid DECIMAL(6, 2),
    method TEXT NULL,
    platform TEXT,
    subscription_id INT NULL DEFAULT NULL,
    transaction_id INT,
    notes TEXT
);

CREATE TABLE accounts (
    id SERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE
);
