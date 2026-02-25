PRAGMA foreign_keys = ON;

-- Drop existing tables if rerunning
DROP TABLE IF EXISTS mixed;


--------------------------------------------------
-- MIXED TYPES TABLE (type affinity tests)
--------------------------------------------------
CREATE TABLE mixed (
    id INTEGER PRIMARY KEY,
    anything,
    txt TEXT,
    num NUMERIC,
    realval REAL,
    blobval BLOB
);

INSERT INTO mixed (anything, txt, num, realval, blobval) VALUES
(42, "text", "123", 3.14, X'ABCD'),
("string", 999, "not a number", -2.5, X'00'),
(NULL, NULL, NULL, NULL, NULL);

--------------------------------------------------
-- Indexes (to test schema parsing)
--------------------------------------------------