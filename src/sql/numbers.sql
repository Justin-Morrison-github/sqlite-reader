PRAGMA foreign_keys = ON;

-- Drop existing tables if rerunning
DROP TABLE IF EXISTS numbers;


--------------------------------------------------
-- NUMBERS TABLE (integer edge cases)
--------------------------------------------------
CREATE TABLE numbers (
    id INTEGER PRIMARY KEY,
    small_int INTEGER,
    big_int INTEGER,
    negative INTEGER,
    zero INTEGER
);

INSERT INTO numbers (small_int, big_int, negative, zero) VALUES
(1, 9223372036854775807, -1, 0),
(127, 128, -128, 0),
(255, 256, -32768, 0);