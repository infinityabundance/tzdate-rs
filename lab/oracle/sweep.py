#!/usr/bin/env python3
"""TZDATE-RS.1 oracle sweep: compare compiled tzcode `date` vs `tzdate` (rs).

Deterministic paths only (`-r <fixed time_t>`): every strftime specifier across
many timestamps under -u; named zones with DST/odd offsets; and the option/`-r`/
operand error matrix. argv0 is normalized (getopt embeds it). The no-`-r`
current-time path is classified (live clock), not byte-compared.
"""
import os, subprocess, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OR = os.path.join(ROOT, "lab/admit/tzdb-2026b/date")
RS = os.path.join(ROOT, "target/release/tzdate")
ZD = os.path.join(ROOT, "lab/oracle/tzdir")

def norm(s):
    return s.replace(OR, "date").replace(RS, "date")

def run(binary, args, tz=None):
    env = dict(os.environ, LC_ALL="C.UTF-8", TZDIR=ZD)
    if tz is not None:
        env["TZ"] = tz
    else:
        env.pop("TZ", None)
    p = subprocess.run([binary] + args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    return norm(p.stdout.decode("utf-8", "replace")), norm(p.stderr.decode("utf-8", "replace")), p.returncode

PASS = 0
FAIL = 0
FAILED = []

def cmp(label, args, tz=None):
    global PASS, FAIL
    oo, oe, orc = run(OR, args, tz)
    ro, re_, rrc = run(RS, args, tz)
    if oo == ro and oe == re_ and orc == rrc:
        PASS += 1
    else:
        FAIL += 1
        FAILED.append(label)
        if len(FAILED) <= 20:
            print(f"FAIL {label}")
            if oo != ro:
                print(f"   stdout oracle=[{oo!r}] rs=[{ro!r}]")
            if oe != re_:
                print(f"   stderr oracle=[{oe!r}] rs=[{re_!r}]")
            if orc != rrc:
                print(f"   rc oracle={orc} rs={rrc}")

# Timestamps: epoch, pre-1970, 2001, 32-bit boundary, 2038, far future, leap-day,
# year boundaries, ISO-week edges (Jan 1 / Dec 31 of various years).
TIMES = [0, 1, -1, -86400, 1000000000, 2147483647, 2147483648, 951782400,  # 2000-02-29
         -2208988800, 4102444800, 1609459200, 1640995199, 978307199, 915148800,
         1483228800, 1451606400, 1262304000, 1234567890, -315619200, 13569465600]
# Every conversion specifier + composites + the default %+.
SPECS = list("aAbBcCdDeFgGhHIjklmMnpPrRsStTuUVwWxXyYzZ%+")
FMTS = ["+%" + c for c in SPECS] + [
    "+%Y-%m-%dT%H:%M:%S%z", "+%a %b %e %H:%M:%S %Z %Y", "+week %V of %G (%g) day %u",
    "+%U|%W|%V", "+%I%p %l %k", "+lit%% %nN %tT done", "+%-d %_d %0d %^a %Oy %Ey",
]

# --- UTC specifier sweep ---
for t in TIMES:
    cmp(f"utc default t={t}", ["-u", "-r", str(t)])
    for f in FMTS:
        cmp(f"utc t={t} {f}", ["-u", "-r", str(t), f])

# --- named zones (localtime via TZif): DST, odd offsets, +14, southern DST ---
ZONES = ["America/New_York", "Europe/London", "Asia/Kathmandu", "Australia/Lord_Howe",
         "Pacific/Kiritimati", "Asia/Kolkata", "Pacific/Chatham", "Europe/Lisbon",
         "America/Argentina/Buenos_Aires", "Asia/Tehran", "UTC", "Etc/GMT-14"]
ZTIMES = [0, 1000000000, 1610000000, 1620000000, 1615705199, 1615705200, 1700000000,
          -1000000000, 1300000000, 1331431200, 2147483647]
for z in ZONES:
    for t in ZTIMES:
        cmp(f"zone {z} t={t}", ["-r", str(t)], tz=z)
        cmp(f"zone {z} t={t} fmt", ["-r", str(t), "+%a %Y-%m-%d %H:%M:%S %Z %z %s %j %V"], tz=z)

# --- POSIX UTC via TZ=UTC0 and -c ---
for t in [0, 1000000000]:
    cmp(f"tz=UTC0 t={t}", ["-r", str(t)], tz="UTC0")
    cmp(f"-c t={t}", ["-c", "-r", str(t)])

# --- error / edge matrix ---
ERRS = [
    ["-Z"], ["-q"], ["-u", "foo"], ["-u", "+%Y", "extra"], ["foo", "bar"],
    ["-r"], ["-r", "notanumber"], ["-r", "0", "-r", "1"], ["-r", ""],
    ["-r", "0x10"], ["-r", "010"], ["-r", "-5"], ["-r", "+5"], ["-r", "99999999999999999999"],
    ["-r", "0", "-u"], ["-ur0"], ["-u", "-r0"], ["--", "+%Y"], ["-r", "12abc"],
    ["+%Y"], ["-u"], ["-uc"], ["-cu", "-r", "0"], ["+"], ["-u", "+"],
]
for e in ERRS:
    cmp("err " + " ".join(repr(x) for x in e), e, tz="America/New_York")

print("==========================")
print(f"PASS={PASS} FAIL={FAIL}")
if FAILED:
    print("failed sample:", FAILED[:25])
sys.exit(1 if FAIL else 0)
