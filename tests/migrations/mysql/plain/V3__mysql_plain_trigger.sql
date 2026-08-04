-- tern:noTransaction,mysql
CREATE TABLE tern_mysql_plain_log (id BIGINT NOT NULL);
CREATE TRIGGER tern_mysql_plain_trg AFTER INSERT ON tern_mysql_plain
FOR EACH ROW
BEGIN
  IF NEW.x IS NOT NULL THEN
    INSERT INTO tern_mysql_plain_log VALUES (NEW.x);
  END IF;
END;
INSERT INTO tern_mysql_plain (x, y) VALUES (7, 'it\'s fine');
