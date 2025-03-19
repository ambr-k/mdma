DROP TABLE IF EXISTS email_templates;

CREATE TABLE email_templates (
    id TEXT PRIMARY KEY,
    template TEXT NOT NULL
);

DROP TABLE IF EXISTS email_addresses;
CREATE TABLE email_addresses (
    id TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
