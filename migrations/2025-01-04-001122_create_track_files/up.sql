-- Your SQL goes here

CREATE TABLE track_files
(
    id           INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    barcode      TEXT,
    path         TEXT    NOT NULL,
    artist       TEXT,
    album        TEXT,
    title        TEXT,
    track_number INT,
    disc_number  INT,
    disc_total   INT,
    year         INT,
    processed    BOOLEAN NOT NULL DEFAULT 0
)