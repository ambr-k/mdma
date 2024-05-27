CREATE TABLE generations (
    id SERIAL PRIMARY KEY, title TEXT UNIQUE NOT NULL, start_date DATE NOT NULL
);

CREATE VIEW member_generations AS
SELECT id AS member_id, (
        SELECT id AS generation_id
        FROM generations
        WHERE
            start_date <= members.consecutive_since_cached
        ORDER BY start_date DESC
        LIMIT 1
    )
FROM members