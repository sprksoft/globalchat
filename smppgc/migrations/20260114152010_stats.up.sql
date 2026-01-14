-- Add up migration script here


-- every entry represents a week time frame
CREATE TABLE user_stats (
  user_id INTEGER PRIMARY KEY,
  time INTEGER NOT NULL,

  ban_count INTEGER NOT NULL DEFAULT 0,
  message_count INTEGER NOT NULL DEFAULT 0,
  sticker_count INTEGER NOT NULL DEFAULT 0,
  word_count INTEGER NOT NULL DEFAULT 0,
  bad_word_count INTEGER NOT NULL DEFAULT 0,
  online_seconds INTEGER NOT NULL DEFAULT 0,

  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE word_stats (
  word VARCHAR NOT NULL,
  tag INTEGER NOT NULL,
  time INTEGER NOT NULL,
  user_id INTEGER NULL,

  FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
