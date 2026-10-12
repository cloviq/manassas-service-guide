-- Additive: application accounts and server-side sessions (token stored as SHA-256 only).
CREATE TABLE users (
    id uuid PRIMARY KEY,
    username text NOT NULL UNIQUE CHECK (username ~ '^[a-z0-9._-]{3,64}$'),
    password_hash text NOT NULL,
    role text NOT NULL CHECK (role IN ('user','admin')),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE sessions (
    token_hash bytea PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at timestamptz NOT NULL
);
CREATE INDEX sessions_user ON sessions(user_id);
