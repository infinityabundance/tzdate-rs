// POSIX TZ-string parser + DST projector, `include!`d into lib.rs. This is the
// `localtime` behaviour for times beyond a TZif file's last explicit transition
// (the trailing footer rule) and for a `TZ=`-string zone. Scoped to `date.c`'s
// `localtime` needs — not a general library.

/// days since 1970-01-01 for a Gregorian (y, m, d) — inverse of `gmtime`.
fn days_from_civil(mut y: i64, m: i64, d: i64) -> i64 {
    y -= i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn is_leap_y(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// One parsed POSIX `TZ` string.
struct PosixTz {
    std_abbr: String,
    std_off: i64, // UT offset (seconds east of UT)
    dst: Option<PosixDst>,
}
struct PosixDst {
    abbr: String,
    off: i64, // UT offset during DST
    start: Rule,
    end: Rule,
}
/// A transition rule date + time-of-day (seconds, default 02:00:00).
struct Rule {
    kind: RuleKind,
    time: i64,
}
enum RuleKind {
    Mwd(i64, i64, i64), // month (1-12), week (1-5, 5=last), day (0=Sun..6=Sat)
    Julian1(i64),       // Jn: 1..365, Feb 29 never counted
    Julian0(i64),       // n: 0..365, Feb 29 counted
}

/// Parse a `TZ` string; returns `None` if it is not a POSIX form.
fn parse_posix_tz(s: &str) -> Option<PosixTz> {
    let b: Vec<char> = s.chars().collect();
    let mut p = 0;
    let (std_abbr, std_off) = parse_name_offset(&b, &mut p)?;
    if p >= b.len() {
        return Some(PosixTz {
            std_abbr,
            std_off,
            dst: None,
        });
    }
    // DST name + optional offset (default = std_off + 3600 east, i.e. one hour ahead).
    let (dabbr, doff_opt) = parse_name_offset_optional(&b, &mut p)?;
    let dst_off = doff_opt.unwrap_or(std_off + 3600);
    // ,start[/time],end[/time]
    if b.get(p) != Some(&',') {
        return None;
    }
    p += 1;
    let start = parse_rule(&b, &mut p)?;
    if b.get(p) != Some(&',') {
        return None;
    }
    p += 1;
    let end = parse_rule(&b, &mut p)?;
    Some(PosixTz {
        std_abbr,
        std_off,
        dst: Some(PosixDst {
            abbr: dabbr,
            off: dst_off,
            start,
            end,
        }),
    })
}

/// Parse `<name>` or bare letters, then a required signed offset → UT offset.
fn parse_name_offset(b: &[char], p: &mut usize) -> Option<(String, i64)> {
    let name = parse_abbr(b, p)?;
    let off = parse_offset(b, p)?; // POSIX offset (west-positive)
    Some((name, -off)) // UT offset = -(west-positive)
}
/// Same, but the offset is optional (DST defaults to std+1h).
fn parse_name_offset_optional(b: &[char], p: &mut usize) -> Option<(String, Option<i64>)> {
    let name = parse_abbr(b, p)?;
    if matches!(b.get(*p), Some('-') | Some('+')) || b.get(*p).map(|c| c.is_ascii_digit()).unwrap_or(false) {
        let off = parse_offset(b, p)?;
        Some((name, Some(-off)))
    } else {
        Some((name, None))
    }
}

fn parse_abbr(b: &[char], p: &mut usize) -> Option<String> {
    if b.get(*p) == Some(&'<') {
        let start = *p + 1;
        let mut q = start;
        while q < b.len() && b[q] != '>' {
            q += 1;
        }
        if b.get(q) != Some(&'>') {
            return None;
        }
        let name: String = b[start..q].iter().collect();
        *p = q + 1;
        Some(name)
    } else {
        let start = *p;
        while *p < b.len() && b[*p].is_ascii_alphabetic() {
            *p += 1;
        }
        if *p == start {
            return None;
        }
        Some(b[start..*p].iter().collect())
    }
}

/// POSIX offset `hh[:mm[:ss]]`, optional sign (default `+`); west-positive.
fn parse_offset(b: &[char], p: &mut usize) -> Option<i64> {
    let neg = match b.get(*p) {
        Some('-') => {
            *p += 1;
            true
        }
        Some('+') => {
            *p += 1;
            false
        }
        _ => false,
    };
    let hh = parse_int(b, p)?;
    let mut mm = 0;
    let mut ss = 0;
    if b.get(*p) == Some(&':') {
        *p += 1;
        mm = parse_int(b, p)?;
        if b.get(*p) == Some(&':') {
            *p += 1;
            ss = parse_int(b, p)?;
        }
    }
    let v = hh * 3600 + mm * 60 + ss;
    Some(if neg { -v } else { v })
}

fn parse_int(b: &[char], p: &mut usize) -> Option<i64> {
    let start = *p;
    while *p < b.len() && b[*p].is_ascii_digit() {
        *p += 1;
    }
    if *p == start {
        return None;
    }
    b[start..*p].iter().collect::<String>().parse().ok()
}

/// Parse `Mm.w.d` | `Jn` | `n`, with optional `/time` (default 02:00:00).
fn parse_rule(b: &[char], p: &mut usize) -> Option<Rule> {
    let kind = match b.get(*p) {
        Some('M') => {
            *p += 1;
            let m = parse_int(b, p)?;
            if b.get(*p) != Some(&'.') {
                return None;
            }
            *p += 1;
            let w = parse_int(b, p)?;
            if b.get(*p) != Some(&'.') {
                return None;
            }
            *p += 1;
            let d = parse_int(b, p)?;
            RuleKind::Mwd(m, w, d)
        }
        Some('J') => {
            *p += 1;
            RuleKind::Julian1(parse_int(b, p)?)
        }
        Some(c) if c.is_ascii_digit() => RuleKind::Julian0(parse_int(b, p)?),
        _ => return None,
    };
    let time = if b.get(*p) == Some(&'/') {
        *p += 1;
        parse_signed_time(b, p)?
    } else {
        7200 // 02:00:00
    };
    Some(Rule { kind, time })
}

fn parse_signed_time(b: &[char], p: &mut usize) -> Option<i64> {
    let neg = match b.get(*p) {
        Some('-') => {
            *p += 1;
            true
        }
        Some('+') => {
            *p += 1;
            false
        }
        _ => false,
    };
    let hh = parse_int(b, p)?;
    let mut mm = 0;
    let mut ss = 0;
    if b.get(*p) == Some(&':') {
        *p += 1;
        mm = parse_int(b, p)?;
        if b.get(*p) == Some(&':') {
            *p += 1;
            ss = parse_int(b, p)?;
        }
    }
    let v = hh * 3600 + mm * 60 + ss;
    Some(if neg { -v } else { v })
}

/// The local-civil day-of-year (0-based) of a rule in year `y`.
fn rule_yday(kind: &RuleKind, y: i64) -> i64 {
    match *kind {
        RuleKind::Julian1(n) => {
            // 1..365, Feb 29 never counted: yday0 = n-1, +1 if after Feb in leap year.
            let mut yd = n - 1;
            if n >= 60 && is_leap_y(y) {
                yd += 1;
            }
            yd
        }
        RuleKind::Julian0(n) => n, // 0..365, Feb 29 counted
        RuleKind::Mwd(m, w, d) => {
            let cum: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
            let leap = is_leap_y(y);
            let mdays = [
                31,
                if leap { 29 } else { 28 },
                31,
                30,
                31,
                30,
                31,
                31,
                30,
                31,
                30,
                31,
            ];
            let first_yday = cum[(m - 1) as usize] + if m > 2 && leap { 1 } else { 0 };
            // weekday of the 1st of month m:
            let first_days = days_from_civil(y, m, 1);
            let first_wday = (first_days + 4).rem_euclid(7); // 0=Sun
            // first day-of-month with weekday d:
            let mut dom = 1 + (d - first_wday).rem_euclid(7);
            dom += (w - 1) * 7;
            if dom > mdays[(m - 1) as usize] {
                dom -= 7; // w==5 (last) overshoot
            }
            first_yday + (dom - 1)
        }
    }
}

/// The UT instant of a rule's transition in year `y`, given the UT offset in
/// effect just before the transition (`off_before`).
fn rule_ut(rule: &Rule, y: i64, off_before: i64) -> i64 {
    let yd = rule_yday(&rule.kind, y);
    let day0 = days_from_civil(y, 1, 1) + yd;
    let local = day0 * 86400 + rule.time;
    local - off_before
}

/// Project a POSIX `TZ` string for timestamp `t` → `(utoff, isdst, abbr)`.
fn posix_tz_offset(s: &str, t: i64) -> Option<Localtime> {
    let tz = parse_posix_tz(s)?;
    let dst = match &tz.dst {
        None => {
            return Some(Localtime {
                utoff: tz.std_off,
                isdst: false,
                abbrev: tz.std_abbr,
            })
        }
        Some(d) => d,
    };
    // Determine the local year, then the DST window for that year. The rules
    // repeat annually, so an off-by-one at the Jan/Dec boundary still yields the
    // correct DST verdict (both hemispheres handled by the start<=end test).
    let y = gmtime(t + tz.std_off).year;
    let start = rule_ut(&dst.start, y, tz.std_off); // before start: std in effect
    let end = rule_ut(&dst.end, y, dst.off); // before end: dst in effect
    let in_dst = if start <= end {
        start <= t && t < end
    } else {
        t >= start || t < end
    };
    if in_dst {
        Some(Localtime {
            utoff: dst.off,
            isdst: true,
            abbrev: dst.abbr.clone(),
        })
    } else {
        Some(Localtime {
            utoff: tz.std_off,
            isdst: false,
            abbrev: tz.std_abbr.clone(),
        })
    }
}
