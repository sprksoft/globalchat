-- Add up migration script here

CREATE TABLE promote_keys(
  key VARCHAR PRIMARY KEY,
  new_role INTEGER NOT NULL,
  used_by INTEGER NULL,

  FOREIGN KEY (used_by) REFERENCES users(id) ON DELETE CASCADE
);

CREATE OR REPLACE FUNCTION claim_key(
  ses_id UUID,
  target_key VARCHAR
) RETURNS TEXT AS $$
DECLARE
  user_role INTEGER;
  user_id INTEGER;
  v_used_by INTEGER;
  v_new_role INTEGER;
BEGIN
    SELECT users.role, users.id INTO user_role, user_id FROM sessions INNER JOIN users ON sessions.user_id = users.id WHERE sessions.id = ses_id;
    SELECT new_role, used_by INTO v_new_role, v_used_by FROM promote_keys WHERE key = target_key;

    IF user_id IS NULL THEN
      RETURN 'notloggedin';
    END IF;

    IF v_new_role IS NULL THEN
      RETURN 'invalidkey';
    END IF;

    IF v_used_by IS NOT NULL THEN
      RETURN 'keyused';
    END IF;

    IF user_role >= v_new_role THEN
      RETURN 'higherrole';
    END IF;

    UPDATE promote_keys SET used_by=user_id WHERE key = target_key;
    UPDATE users SET role=v_new_role WHERE id=user_id;
    RETURN 'ok';
END;
$$ LANGUAGE plpgsql;
