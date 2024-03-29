CREATE TABLE members (
    id SERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    reason_removed TEXT DEFAULT NULL,
    created_on DATE DEFAULT NOW() NOT NULL,
    consecutive_since_cached DATE NULL,
    consecutive_until_cached DATE NULL
);

CREATE TABLE payments (
    id SERIAL PRIMARY KEY,
    member_id INT REFERENCES members (id) NOT NULL,
    effective_on DATE NOT NULL DEFAULT NOW(),
    created_on DATE NOT NULL DEFAULT NOW(),
    duration_months INT NOT NULL DEFAULT 1,
    amount_paid DECIMAL(6, 2) NOT NULL,
    payment_method TEXT NULL,
    platform TEXT,
    transaction_id INT,
    notes TEXT
);

CREATE TABLE accounts (
    id SERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE VIEW latest_streaks AS
WITH payment_ranges AS (
    SELECT
        member_id,
        effective_on AS start_date,
        effective_on + (INTERVAL '1 month' * duration_months) + (INTERVAL '7 days') AS end_date
    FROM payments
), payments_has_previous AS (
    SELECT
        member_id,
        start_date,
        COALESCE(start_date <= LAG(end_date) OVER (PARTITION BY member_id ORDER BY end_date ASC), FALSE) AS has_previous
    FROM payment_ranges
), latest_streak_starts AS (
    SELECT member_id, MAX(start_date) AS start_date
    FROM payments_has_previous
    WHERE NOT has_previous
    GROUP BY member_id
), latest_payment_ends AS (
    SELECT member_id, MAX(end_date) AS end_date
    FROM payment_ranges
    GROUP BY member_id
)
SELECT
    latest_streak_starts.member_id AS member_id,
    latest_streak_starts.start_date AS start_date,
    latest_payment_ends.end_date AS end_date
FROM latest_streak_starts INNER JOIN latest_payment_ends ON latest_streak_starts.member_id = latest_payment_ends.member_id;

CREATE FUNCTION update_consecutive_cache () RETURNS TRIGGER AS $$
DECLARE
    cached_end_date CONSTANT DATE := (SELECT consecutive_until_cached FROM members WHERE id = NEW.member_id LIMIT 1);
    payment_ends CONSTANT DATE := NEW.effective_on + (INTERVAL '1 month' * NEW.duration_months) + (INTERVAL '7 days');
BEGIN
    IF cached_end_date BETWEEN NEW.effective_on AND payment_ends THEN
        UPDATE members
        SET consecutive_until_cached = payment_ends
        WHERE id = NEW.member_id;
    ELSIF (cached_end_date IS NULL) OR (cached_end_date < NEW.effective_on) THEN
        UPDATE members
        SET consecutive_since_cached = NEW.effective_on, consecutive_until_cached = payment_ends
        WHERE id = NEW.member_id;
    ELSE
        WITH latest_streak AS (SELECT * FROM latest_streaks WHERE member_id = NEW.member_id)
        UPDATE members
        SET consecutive_since_cached = latest_streak.start_date, consecutive_until_cached = latest_streak.end_date
        WHERE id = NEW.member_id;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_consecutive_cache_on_insert
    AFTER INSERT ON payments
    FOR EACH ROW
    EXECUTE FUNCTION update_consecutive_cache ();