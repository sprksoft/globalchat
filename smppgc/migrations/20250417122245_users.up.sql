-- Add up migration script here

CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  smid VARCHAR NOT NULL UNIQUE,
  irl_name VARCHAR NOT NULL,
  role INTEGER NOT NULL DEFAULT 0,
  ban_count INTEGER NOT NULL DEFAULT 0,
);

CREATE TABLE names (
  name VARCHAR NOT NULL PRIMARY KEY,
  user_id INTEGER NOT NULL,
  last_used INTEGER NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE OR REPLACE FUNCTION claim_name(
    id INTEGER,
    claim_name VARCHAR,
    max_names INTEGER,
    retention_seconds INTEGER
) RETURNS TEXT AS $$
DECLARE
  claimed_names_count INTEGER;
  oldest_name VARCHAR;
  name_exists INTEGER;
BEGIN
    DELETE FROM names WHERE EXTRACT(epoch from now())-last_used > retention_seconds;

    SELECT COUNT(*) INTO claimed_names_count FROM names WHERE user_id = id;
    SELECT name INTO oldest_name FROM names WHERE user_id = id ORDER BY last_used LIMIT 1;
    IF claimed_names_count >= max_names THEN
      DELETE FROM names WHERE name = oldest_name AND user_id = id;
    END IF;

    SELECT COUNT(*) INTO name_exists FROM names WHERE name = claim_name AND user_id = id LIMIT 1;

    IF name_exists = 1 THEN
      RETURN 'ok';
    END IF;

    -- Create username if doesn't exist (using ON CONFLICT DO NOTHING)
    INSERT INTO names (name,user_id,last_used)
    VALUES (claim_name, id, EXTRACT(epoch from now()))
    ON CONFLICT (name) DO NOTHING;

    IF NOT FOUND THEN
      RETURN NULL;
    END IF;

    RETURN 'ok';
END;
$$ LANGUAGE plpgsql;
