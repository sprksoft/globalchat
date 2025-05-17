-- Add down migration script here
DROP TABLE names;
DROP TABLE users;
DROP TABLE sessions;

DROP FUNCTION claim_name(
  id INTEGER,
  name VARCHAR,
  retention_seconds INTEGER,
  max_names INTEGER
);
