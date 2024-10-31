# examples

```
> $ docker compose up -d
> $ export DATABASE_URL=postgres://postgres:password@dockerhost:5433/postgres?sslmode=disable
> $ cargo b
> $ target/debug/pg-envvar-query migrate run
newly applied migrations [AppliedMigration { version: 1, description: "create table dmd test", content: "Q1JFQVRFIFRBQkxFIGRtZF90ZXN0KAogIGlkIHNlcmlhbCBQUklNQVJZIEtFWSwKICBjcmVhdGVkX2F0IHRpbWVzdGFtcHR6IE5PVCBOVUxMIERFRkFVTFQgbm93KCksCiAgeCBiaWdpbnQsCiAgeSB0ZXh0Cik7Cg==", duration_sec: 0 }, AppliedMigration { version: 2, description: "insert env value", content: "dXNlIGRlcnJpY2s6OkVycm9yOwp1c2UgZGVycmljazo6UXVlcnlCdWlsZGVyOwoKdXNlIHN1cGVyOjpFeGFtcGxlTWlncmF0ZTsKCiNbZGVyaXZlKFF1ZXJ5QnVpbGRlcildCiNbbWlncmF0aW9uKG5vX3RyYW5zYWN0aW9uLCBydW50aW1lID0gRXhhbXBsZU1pZ3JhdGUpXQpwdWIgc3RydWN0IEluc2VydFZhbHVlRnJvbUVudjsKCnB1YiBhc3luYyBmbiBidWlsZF9xdWVyeShydW50aW1lOiAmbXV0IEV4YW1wbGVNaWdyYXRlKSAtPiBSZXN1bHQ8U3RyaW5nLCBFcnJvcj4gewogICAgbGV0IHVzZXIgPSBydW50aW1lCiAgICAgICAgLmVudgogICAgICAgIC5nZXRfdmFyKCJVU0VSIikKICAgICAgICAuZXhwZWN0KCJjb3VsZCBub3QgZ2V0IGBVU0VSYCBmcm9tIGVudmlyb25tZW50Iik7CiAgICBsZXQgc3FsID0gZm9ybWF0ISgiSU5TRVJUIElOVE8gZG1kX3Rlc3QoeCwgeSkgVkFMVUVTICh7fSwgJ3t9Jyk7IiwgMjUsIHVzZXIpOwoKICAgIE9rKHNxbCkKfQo=", duration_sec: 0 }, AppliedMigration { version: 3, description: "create dmd test y idx", content: "LS0gZGVycmljazpub1RyYW5zYWN0aW9uCkNSRUFURSBJTkRFWCBDT05DVVJSRU5UTFkgSUYgTk9UIEVYSVNUUyBkbWRfdGVzdF95X2lkeCBPTiBkbWRfdGVzdCh5KTsK", duration_sec: 0 }]

> $ psql "postgres://postgres:password@dockerhost:5433/postgres?sslmode=disable"
psql (14.13 (Homebrew))
Type "help" for help.

postgres=# \dt
                List of relations
 Schema |        Name         | Type  |  Owner
--------+---------------------+-------+----------
 public | _derrick_migrations | table | postgres
 public | dmd_test            | table | postgres
(2 rows)

postgres=# select * from dmd_test
postgres-# ;
 id |          created_at           | x  |       y
----+-------------------------------+----+---------------
  1 | 2024-10-31 23:47:52.380714+00 | 25 | quasicoherent
(1 row)
...
```
