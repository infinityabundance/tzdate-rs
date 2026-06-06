// A `strftime` covering the C-locale conversions tzcode's strftime.c produces,
// `include!`d into lib.rs. Scoped to `date.c`'s use — not a general library.
// Verified conversion-by-conversion against the compiled tzcode oracle.

const DAY_ABBR: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const DAY_FULL: [&str; 7] = [
    "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday",
];
const MON_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MON_FULL: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

/// `strftime(format, tm)` for the given broken-down time, original `time_t`
/// (`%s`), UT offset (`%z`) and zone abbreviation (`%Z`).
pub fn strftime(format: &str, tm: &Tm, t: i64, gmtoff: i64, zone: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = format.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c != '%' {
            out.push(c);
            i += 1;
            continue;
        }
        // Optional E/O modifier (ignored under the C locale).
        let mut j = i + 1;
        if j < chars.len() && (chars[j] == 'E' || chars[j] == 'O') {
            j += 1;
        }
        let spec = match chars.get(j) {
            Some(&s) => s,
            None => {
                // trailing '%' → output it literally.
                out.push('%');
                break;
            }
        };
        if let Some(s) = expand(spec, tm, t, gmtoff, zone) {
            out.push_str(&s);
            i = j + 1;
        } else {
            // Unrecognised conversion: tzcode emits '%' + the spec char verbatim
            // (a following normal char stays literal), e.g. "%-d" → "%-d".
            out.push('%');
            out.push(chars[i + 1]);
            i += 2;
        }
    }
    out
}

fn expand(spec: char, tm: &Tm, t: i64, gmtoff: i64, zone: &str) -> Option<String> {
    let two = |n: i64| format!("{n:02}");
    let sp2 = |n: i64| format!("{n:2}");
    let recur = |f: &str| strftime(f, tm, t, gmtoff, zone);
    let s = match spec {
        'a' => DAY_ABBR[tm.wday as usize].to_string(),
        'A' => DAY_FULL[tm.wday as usize].to_string(),
        'b' | 'h' => MON_ABBR[(tm.month - 1) as usize].to_string(),
        'B' => MON_FULL[(tm.month - 1) as usize].to_string(),
        'c' => recur("%a %b %e %H:%M:%S %Y"),
        'C' => two(div_floor(tm.year, 100)),
        'd' => two(tm.mday),
        'D' => recur("%m/%d/%y"),
        'e' => sp2(tm.mday),
        'F' => recur("%Y-%m-%d"),
        'g' => two(iso_week_year(tm).1.rem_euclid(100)),
        'G' => format!("{}", iso_week_year(tm).1),
        'H' => two(tm.hour),
        'I' => two(hour12(tm.hour)),
        'j' => format!("{:03}", tm.yday + 1),
        'k' => sp2(tm.hour),
        'l' => sp2(hour12(tm.hour)),
        'm' => two(tm.month),
        'M' => two(tm.min),
        'n' => "\n".to_string(),
        'p' => if tm.hour < 12 { "AM" } else { "PM" }.to_string(),
        'r' => recur("%I:%M:%S %p"),
        'R' => recur("%H:%M"),
        's' => format!("{t}"),
        'S' => two(tm.sec),
        't' => "\t".to_string(),
        'T' => recur("%H:%M:%S"),
        'u' => format!("{}", if tm.wday == 0 { 7 } else { tm.wday }),
        'U' => two(div_floor(tm.yday - tm.wday + 7, 7)),
        'V' => two(iso_week_year(tm).0),
        'w' => format!("{}", tm.wday),
        'W' => two(div_floor(tm.yday - ((tm.wday + 6) % 7) + 7, 7)),
        'x' => recur("%m/%d/%y"),
        'X' => recur("%H:%M:%S"),
        'y' => two(tm.year.rem_euclid(100)),
        'Y' => format!("{}", tm.year),
        'z' => {
            let sign = if gmtoff < 0 { '-' } else { '+' };
            let a = gmtoff.abs();
            format!("{sign}{:02}{:02}", a / 3600, a % 3600 / 60)
        }
        'Z' => zone.to_string(),
        '%' => "%".to_string(),
        '+' => recur("%a %b %e %H:%M:%S %Z %Y"),
        _ => return None,
    };
    Some(s)
}

fn hour12(h: i64) -> i64 {
    let h = h % 12;
    if h == 0 {
        12
    } else {
        h
    }
}

fn div_floor(a: i64, b: i64) -> i64 {
    let q = a / b;
    if (a % b != 0) && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

/// ISO-8601 week number and week-based year (`%V`, `%G`/`%g`).
fn iso_week_year(tm: &Tm) -> (i64, i64) {
    let doy = tm.yday + 1; // 1-based
    let w = if tm.wday == 0 { 7 } else { tm.wday }; // Mon=1 .. Sun=7
    let mut week = div_floor(doy - w + 10, 7);
    let mut iy = tm.year;
    if week < 1 {
        iy = tm.year - 1;
        week = weeks_in_year(iy);
    } else if week > 52 && week > weeks_in_year(tm.year) {
        week = 1;
        iy = tm.year + 1;
    }
    (week, iy)
}

fn weeks_in_year(y: i64) -> i64 {
    let p = |y: i64| (y + div_floor(y, 4) - div_floor(y, 100) + div_floor(y, 400)).rem_euclid(7);
    if p(y) == 4 || p(y - 1) == 3 {
        53
    } else {
        52
    }
}

