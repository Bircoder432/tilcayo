ALTER TABLE users
ADD COLUMN username TEXT;

UPDATE users
SET username = 'user-' || id
WHERE username IS NULL;

ALTER TABLE users
ALTER COLUMN username SET NOT NULL;

CREATE UNIQUE INDEX users_username_idx
ON users (username);
