-- Add down migration script here

DROP TABLE promote_keys;

DROP FUNCTION claim_key(
  ses_id UUID,
  key VARCHAR
);
