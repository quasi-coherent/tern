-- tern:noTransaction,sqlite
CREATE TABLE tern_sqlite_plain_log (id INTEGER NOT NULL);
CREATE TRIGGER tern_sqlite_plain_trg AFTER INSERT ON tern_sqlite_plain
BEGIN
  INSERT INTO tern_sqlite_plain_log VALUES (new.x);
END;
INSERT INTO tern_sqlite_plain (x, y) VALUES (7, 'seven');
