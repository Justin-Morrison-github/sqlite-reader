PRAGMA foreign_keys = ON;

-- Drop existing tables if rerunning
DROP TABLE IF EXISTS files;

--------------------------------------------------
-- FILES TABLE (blob storage)
--------------------------------------------------
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    name TEXT,
    data BLOB
);

INSERT INTO files (name, data) VALUES
("empty", X''),
("binary", X'00010203040506070809'),
("textblob", CAST("hello world" AS BLOB));