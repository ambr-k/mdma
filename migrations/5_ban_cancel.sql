ALTER TABLE members
ADD COLUMN IF NOT EXISTS notes TEXT NOT NULL DEFAULT '',
ADD COLUMN IF NOT EXISTS cancelled BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS banned BOOLEAN NOT NULL DEFAULT FALSE;

CREATE OR REPLACE FUNCTION is_active(member_id_arg INTEGER) RETURNS BOOLEAN
LANGUAGE SQL
STABLE STRICT
AS $$
    SELECT
        (NOT banned)
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

ALTER TABLE members DROP COLUMN IF EXISTS reason_removed;

CREATE OR REPLACE FUNCTION uncancel_on_payment () RETURNS TRIGGER
LANGUAGE PLPGSQL AS $$
BEGIN
    UPDATE members
    SET
        cancelled = FALSE,
        notes = TRIM(E'\n' FROM notes || E'\n\n=== ' || CURRENT_DATE || E' ===\nAutomatically uncancelled from Payment ID ' || NEW.id)
    WHERE id = NEW.member_id AND cancelled;
    RETURN NULL;
END; $$;

DROP TRIGGER IF EXISTS uncancel_on_payment_on_insert ON payments;

CREATE TRIGGER uncancel_on_payment_on_insert
AFTER INSERT ON payments FOR EACH ROW
EXECUTE FUNCTION uncancel_on_payment ();