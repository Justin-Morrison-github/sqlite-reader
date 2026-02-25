PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS posts;

--------------------------------------------------
-- POSTS TABLE (foreign keys + long text)
--------------------------------------------------
CREATE TABLE posts (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    title TEXT,
    body TEXT,
    published INTEGER,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

INSERT INTO posts (user_id, title, body, published) VALUES
(1, "Hello World", "My first post", 1),
(1, "Long Post", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", 0),
(2, "SQLite", "Testing parser edge cases", 1),
(3, "Null Author", NULL, 0);
