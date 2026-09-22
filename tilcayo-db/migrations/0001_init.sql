CREATE TABLE roles (
    id BIGSERIAL PRIMARY KEY
);

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    role_id BIGINT NOT NULL REFERENCES roles(id)
);

CREATE TABLE schemas (
    id BIGSERIAL PRIMARY KEY,
    schema JSONB NOT NULL
);

CREATE TABLE resources (
    id BIGSERIAL PRIMARY KEY,
    schema_id BIGINT NOT NULL REFERENCES schemas(id),
    data JSONB NOT NULL
);

CREATE TABLE permissions (
    role_id BIGINT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    schema_id BIGINT NOT NULL REFERENCES schemas(id),
    read BOOLEAN NOT NULL DEFAULT FALSE,
    write BOOLEAN NOT NULL DEFAULT FALSE,

    PRIMARY KEY (role_id, schema_id)
);
