CREATE OR REPLACE FUNCTION is_active(member_id_arg INTEGER) RETURNS BOOLEAN
LANGUAGE SQL
STABLE STRICT
AS $$
    SELECT
        (reason_removed IS DISTINCT FROM '')
        AND EXISTS (
            SELECT id
            FROM payments
            WHERE
                member_id = member_id_arg
                AND effective_on + (
                    INTERVAL '1 month' * duration_months
                ) + (INTERVAL '7 days') >= NOW()
        )
    FROM members
    WHERE
        id = member_id_arg;
$$;

CREATE OR REPLACE FUNCTION consecutive_since(member_id_arg INTEGER) RETURNS DATE
LANGUAGE PLPGSQL
STABLE STRICT
AS $$
    DECLARE
        earliest DATE := NULL;
        candidate DATE := NULL;
    BEGIN
        SELECT effective_on
            FROM payments
            WHERE member_id = member_id_arg
            ORDER BY (effective_on + INTERVAL '1 month' * duration_months) DESC
            LIMIT 1
            INTO candidate;
        
        WHILE candidate IS NOT NULL LOOP
            earliest := candidate;
            SELECT effective_on
                FROM payments
                WHERE
                    member_id = member_id_arg
                    AND effective_on < earliest
                    AND effective_on + INTERVAL '1 month' * duration_months + INTERVAL '7 days' >= earliest
                ORDER BY effective_on ASC
                LIMIT 1
                INTO candidate;
        END LOOP;
        RETURN earliest;
    END;
$$;

CREATE OR REPLACE FUNCTION has_payment_gap(member_id_arg INTEGER) RETURNS BOOLEAN
LANGUAGE SQL
STABLE STRICT
AS $$
    SELECT effective_on <> consecutive_since(member_id_arg)
    FROM payments
    WHERE member_id = member_id_arg
    ORDER BY effective_on ASC
    LIMIT 1
$$;

CREATE OR REPLACE FUNCTION consecutive_until(member_id_arg INTEGER) RETURNS DATE
LANGUAGE SQL
STABLE STRICT
AS $$
    SELECT (effective_on + INTERVAL '1 month' * duration_months)::DATE
        FROM payments
        WHERE member_id = member_id_arg
        ORDER BY (effective_on + INTERVAL '1 month' * duration_months) DESC
        LIMIT 1
$$;

DROP TABLE IF EXISTS consecutive_since_cached;

CREATE TABLE consecutive_since_cached (
    member_id INT REFERENCES members (id) PRIMARY KEY,
    cached_value DATE
);

CREATE OR REPLACE FUNCTION update_consecutive_since_cached () RETURNS TRIGGER
LANGUAGE PLPGSQL AS $$
BEGIN
    INSERT INTO consecutive_since_cached (member_id, cached_value)
        VALUES (NEW.member_id, consecutive_since(NEW.member_id))
        ON CONFLICT (member_id) DO UPDATE SET cached_value = consecutive_since(NEW.member_id);
    RETURN NULL;
END; $$;

CREATE TRIGGER update_consecutive_since_cache_on_insert
AFTER INSERT ON payments FOR EACH ROW
EXECUTE FUNCTION update_consecutive_since_cached ();

CREATE OR REPLACE PROCEDURE reload_consecutive_since_cached ()
LANGUAGE SQL
AS $$
    TRUNCATE TABLE consecutive_since_cached;
    INSERT INTO consecutive_since_cached (member_id, cached_value)
        SELECT id, consecutive_since(id) FROM members;
$$;

CREATE OR REPLACE VIEW member_generations AS
SELECT id AS member_id, (
        SELECT id AS generation_id
        FROM
            generations
            LEFT JOIN consecutive_since_cached ON members.id = member_id
        WHERE
            start_date <= consecutive_since_cached.cached_value
        ORDER BY start_date DESC
        LIMIT 1
    )
FROM members;

CREATE OR REPLACE VIEW member_details AS
SELECT
    members.id AS id,
    generations.id AS "generation_id",
    generations.title AS "generation_name",
    consecutive_since_cached.cached_value AS "consecutive_since",
    consecutive_until (members.id) AS "consecutive_until",
    is_active (members.id) AS "is_active",
    (
        SELECT effective_on
        FROM payments
        WHERE
            member_id = members.id
        ORDER BY effective_on ASC
        LIMIT 1
    ) AS "first_payment"
FROM
    members
    LEFT JOIN member_generations ON members.id = member_generations.member_id
    LEFT JOIN generations ON generations.id = generation_id
    LEFT JOIN consecutive_since_cached ON members.id = consecutive_since_cached.member_id;

CALL reload_consecutive_since_cached ();

DROP TRIGGER IF EXISTS update_consecutive_cache_on_insert ON payments;

DROP VIEW IF EXISTS latest_streaks;

DROP FUNCTION IF EXISTS update_consecutive_cache;

ALTER TABLE members
DROP COLUMN IF EXISTS consecutive_since_cached,
DROP COLUMN IF EXISTS consecutive_until_cached;