-- tern:noTransaction
CREATE INDEX test_c1_idx ON test (c1);

CREATE INDEX test_c2_c1_idx ON test (c2, c1);
