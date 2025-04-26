-- Add up migration script here

CREATE TABLE users (
  id SERIAL PRIMARY KEY,
  smid VARCHAR NOT NULL UNIQUE,
  irl_name VARCHAR NOT NULL,
  role INTEGER NOT NULL DEFAULT 0,
  ban_count INTEGER NOT NULL DEFAULT 0,
  ban_release_timestamp INTEGER NOT NULL DEFAULT 0 -- NULL means not banned
);

CREATE TABLE names (
  name VARCHAR NOT NULL PRIMARY KEY,
  user_id INTEGER NOT NULL,
  last_used TIMESTAMP NOT NULL,

  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE OR REPLACE FUNCTION claim_username(
    id INTEGER,
    name VARCHAR,
    retention_seconds INTEGER,
    max_names INTEGER,
) RETURNS TEXT AS $$
DECLARE
  claimed_names_count INTEGER,
  oldest_name: VARCHAR,
BEGIN
    DELETE FROM names WHERE EXTRACT(epoch from now())-last_used < retention_seconds;
    -- Start transaction
    BEGIN
      SELECT COUNT(*),oldest INTO claimed_names_count,oldest FROM names WHERE names.id = id ORDER BY last_used;
      IF claimed_names_count < max_names THEN
        DELETE FROM names WHERE name == oldest_name;
      END IF;

        -- Create username if doesn't exist (using ON CONFLICT DO NOTHING)
        INSERT INTO name (name,user_id,last_used)
        VALUES (name, id, extract(epoch from now()))
        ON CONFLICT (name) DO NOTHING
        RETURNING username_id INTO v_username_id;

        RETURN 'Success: Username claimed successfully';
    EXCEPTION
        WHEN others THEN
            RETURN 'Error: ' || SQLERRM;
    END;

    RETURN 'Success: Username claimed successfully';
END;
$$ LANGUAGE plpgsql;
