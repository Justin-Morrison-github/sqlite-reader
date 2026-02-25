PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS users;
--------------------------------------------------
-- USERS TABLE (basic types + constraints)
--------------------------------------------------
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT,
    age INTEGER,
    balance REAL DEFAULT 0.0,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO users (username, email, age, balance) VALUES
("alice", "alice@example.com", 25, 100.50),
("bob", "bob@example.com", 30, -42.75),
("charlie", NULL, NULL, 0.0),
("delta", "delta@example.com", 999999999, 1.79e308);

-- CREATE INDEX idx_users_username ON users(username);
