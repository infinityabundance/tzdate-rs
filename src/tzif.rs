// Broken-down time + a minimal, bounds-checked TZif reader, `include!`d into
// lib.rs. Scoped to exactly what `date.c`'s `localtime` needs — not a library.

/// Broken-down UTC-or-local time (the fields `strftime` reads).
pub struct Tm {
    pub year: i64,  // full Gregorian year (e.g. 2001)
    pub month: i64, // 1..=12
    pub mday: i64,  // 1..=31
    pub hour: i64,  // 0..=23
    pub min: i64,   // 0..=59
    pub sec: i64,   // 0..=60 (leap seconds not modelled here)
    pub wday: i64,  // 0=Sunday .. 6=Saturday
    pub yday: i64,  // 0..=365
}

fn floor_div(a: i64, b: i64) -> i64 {
    let q = a / b;
    if (a % b != 0) && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}
fn floor_mod(a: i64, b: i64) -> i64 {
    a - floor_div(a, b) * b
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// `gmtime(t)` — decompose a POSIX timestamp into UTC civil fields.
/// Uses Howard Hinnant's `civil_from_days` (days where 0 == 1970-01-01).
pub fn gmtime(t: i64) -> Tm {
    let days = floor_div(t, 86400);
    let secs = floor_mod(t, 86400);
    let hour = secs / 3600;
    let min = secs / 60 % 60;
    let sec = secs % 60;
    let wday = floor_mod(days + 4, 7); // 1970-01-01 was a Thursday (4)

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365] (Mar-based)
    let mp = (5 * doy + 2) / 153; // [0, 11] (Mar=0)
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = y + i64::from(m <= 2);

    // Day of year (0-based) from the civil date.
    let cum: [i64; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let mut yday = cum[(m - 1) as usize] + (d - 1);
    if m > 2 && is_leap(year) {
        yday += 1;
    }

    Tm {
        year,
        month: m,
        mday: d,
        hour,
        min,
        sec,
        wday,
        yday,
    }
}

/// Parse a TZif file and return the `localtime` triple for timestamp `t`.
/// Prefers the 64-bit v2+ block. Bounds-checked: malformed input → `None`,
/// never a panic.
fn tzif_localtime(b: &[u8], t: i64) -> Option<Localtime> {
    if b.len() < 44 || &b[0..4] != b"TZif" {
        return None;
    }
    let version = b[4];
    // First (v1, 32-bit) header at offset 20.
    let h1 = parse_header(b, 20)?;
    if version >= b'2' {
        // Skip the v1 data block, then parse the v2 block.
        let v1_data = h1.timecnt * 4
            + h1.timecnt
            + h1.typecnt * 6
            + h1.charcnt
            + h1.leapcnt * 8
            + h1.isstdcnt
            + h1.isutcnt;
        let v2_start = 20usize.checked_add(parse_header_size())?.checked_add(v1_data)?;
        if v2_start + 4 > b.len() || &b[v2_start..v2_start + 4] != b"TZif" {
            return None;
        }
        let h2_off = v2_start + 20;
        let h2 = parse_header(b, h2_off)?;
        let data_off = h2_off + parse_header_size();
        let footer = extract_footer(b, v2_start);
        return resolve(b, data_off, &h2, t, 8, footer.as_deref());
    }
    // v1-only file (no footer).
    let data_off = 20 + parse_header_size();
    resolve(b, data_off, &h1, t, 4, None)
}

/// The POSIX `TZ` footer between the final two newlines of the v2 block.
fn extract_footer(b: &[u8], v2_start: usize) -> Option<String> {
    let tail = b.get(v2_start..)?;
    let nl1 = tail.iter().rposition(|&c| c == b'\n')?;
    let nl0 = tail[..nl1].iter().rposition(|&c| c == b'\n')?;
    let s = &tail[nl0 + 1..nl1];
    if s.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(s).into_owned())
    }
}

struct TzHeader {
    isutcnt: usize,
    isstdcnt: usize,
    leapcnt: usize,
    timecnt: usize,
    typecnt: usize,
    charcnt: usize,
}

fn parse_header_size() -> usize {
    24 // 6 four-byte counts after the 20-byte magic/version/reserved prefix
}

fn parse_header(b: &[u8], off: usize) -> Option<TzHeader> {
    let g = |i: usize| -> Option<usize> {
        let s = off + i * 4;
        let v = u32::from_be_bytes(b.get(s..s + 4)?.try_into().ok()?);
        Some(v as usize)
    };
    Some(TzHeader {
        isutcnt: g(0)?,
        isstdcnt: g(1)?,
        leapcnt: g(2)?,
        timecnt: g(3)?,
        typecnt: g(4)?,
        charcnt: g(5)?,
    })
}

/// Resolve the type in effect at `t` from a header + its data block.
/// `tw` = transition-time width (4 for v1, 8 for v2). For `t` at or after the
/// last explicit transition, the POSIX `footer` rule governs (as `localtime`).
fn resolve(
    b: &[u8],
    data_off: usize,
    h: &TzHeader,
    t: i64,
    tw: usize,
    footer: Option<&str>,
) -> Option<Localtime> {
    if h.typecnt == 0 {
        return footer.and_then(|f| posix_tz_offset(f, t));
    }
    let trans_off = data_off;
    let typeidx_off = trans_off.checked_add(h.timecnt.checked_mul(tw)?)?;
    let ttinfo_off = typeidx_off.checked_add(h.timecnt)?;
    let desig_off = ttinfo_off.checked_add(h.typecnt.checked_mul(6)?)?;
    if desig_off.checked_add(h.charcnt)? > b.len() {
        return None;
    }

    let read_trans = |i: usize| -> i64 {
        let s = trans_off + i * tw;
        if tw == 8 {
            i64::from_be_bytes(b[s..s + 8].try_into().unwrap())
        } else {
            i32::from_be_bytes(b[s..s + 4].try_into().unwrap()) as i64
        }
    };

    // At/after the last explicit transition, defer to the footer rule (this is
    // how `localtime` projects modern slim/footer-terminated zones).
    if h.timecnt > 0 {
        if let Some(f) = footer {
            if t >= read_trans(h.timecnt - 1) {
                if let Some(l) = posix_tz_offset(f, t) {
                    return Some(l);
                }
            }
        }
    }

    // Otherwise, the type of the last transition <= t (else the first std type).
    let mut type_index: usize = first_standard_type(b, ttinfo_off, h.typecnt);
    for i in 0..h.timecnt {
        if read_trans(i) <= t {
            type_index = *b.get(typeidx_off + i)? as usize;
        } else {
            break;
        }
    }
    if type_index >= h.typecnt {
        return None;
    }

    let ti = ttinfo_off + type_index * 6;
    let utoff = i32::from_be_bytes(b.get(ti..ti + 4)?.try_into().ok()?) as i64;
    let isdst = *b.get(ti + 4)? != 0;
    let desigidx = *b.get(ti + 5)? as usize;
    let abbrev = read_abbr(b, desig_off, h.charcnt, desigidx)?;
    Some(Localtime { utoff, isdst, abbrev })
}

/// The first non-DST type (the rule `localtime` uses before the first
/// transition), else type 0.
fn first_standard_type(b: &[u8], ttinfo_off: usize, typecnt: usize) -> usize {
    for i in 0..typecnt {
        if b.get(ttinfo_off + i * 6 + 4) == Some(&0) {
            return i;
        }
    }
    0
}

fn read_abbr(b: &[u8], desig_off: usize, charcnt: usize, idx: usize) -> Option<String> {
    if idx >= charcnt {
        return None;
    }
    let start = desig_off + idx;
    let mut end = start;
    let limit = desig_off + charcnt;
    while end < limit && b[end] != 0 {
        end += 1;
    }
    Some(String::from_utf8_lossy(&b[start..end]).into_owned())
}
