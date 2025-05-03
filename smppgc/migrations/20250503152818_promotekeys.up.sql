-- Add up migration script here

CREATE TABLE promote_keys(
  key VARCHAR PRIMARY KEY,
  new_role INTEGER NOT NULL,
  used_by INTEGER NULL,

  FOREIGN KEY (used_by) REFERENCES users(id) ON DELETE CASCADE
);
