ALTER TABLE roles
ADD COLUMN name TEXT;

UPDATE roles
SET name = 'role-' || id
WHERE name IS NULL;

ALTER TABLE roles
ALTER COLUMN name SET NOT NULL;

CREATE UNIQUE INDEX roles_name_idx
ON roles (name);


ALTER TABLE schemas
ADD COLUMN name TEXT;

UPDATE schemas
SET name = 'schema-' || id
WHERE name IS NULL;

ALTER TABLE schemas
ALTER COLUMN name SET NOT NULL;

CREATE UNIQUE INDEX schemas_name_idx
ON schemas (name);
