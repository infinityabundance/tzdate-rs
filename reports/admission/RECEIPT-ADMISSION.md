# RECEIPT — admission (tzdb-2026b)

The pinned upstream reference for TZDATE-RS.1.

| Artifact | sha256 |
|----------|--------|
| `tzdb-2026b.tar.lz` (bundle) | `ffad46a04c8d1624197056630af475a35f3556d0887f028ac1bd33b7d47dc653` |
| `date.c` (212 lines) | `6c093549ddbbc94bde79a313cf37e7fe479ad007464c44f91d002d7c2b819d24` |
| `date.1` (man page) | `ba0a0c284607062c…` |
| `localtime.c` | `b8c25773516c07af…` (linked into the oracle; not ported as a library) |
| `strftime.c` | `119fd12398d687c3…` (linked into the oracle; not ported as a library) |
| oracle binary `date` | `4920a1532f87825acd65af71d843879829ff9ff0dfc7a60b6085d93b88527ad7` |

- **Signature:** the bundle's detached OpenPGP signature verifies **GOODSIG /
  VALIDSIG** under the published tz key fingerprint `7E37 92A9 D8AC F7D6 33BC
  1588 ED97 E90E 62AA 7E34` ("Paul Eggert").
- **Build:** `make date` → `DATEOBJS = date.o localtime.o strftime.o`,
  `gcc (GCC) 16.1.1`.
- **Oracle env (pinned):** `LC_ALL=C.UTF-8`, `TZ=<zone>`, `TZDIR=<zic-compiled
  tree>` (`zic -d <TZDIR> tzdata.zi`).
- The bundle / extracted source / compiled oracle are **pinned by hash, never
  vendored** (gitignored); re-fetchable from data.iana.org.
