//! Number formatting for the HUD and markers (upper case, short).

pub fn dist(m: f64) -> String {
    let a = m.abs();
    let sign = if m < 0.0 { "-" } else { "" };
    if a < 1000.0 {
        format!("{sign}{:.0} M", a)
    } else if a < 100_000.0 {
        format!("{sign}{:.1} KM", a / 1000.0)
    } else if a < 10_000_000.0 {
        format!("{sign}{:.0} KM", a / 1000.0)
    } else {
        format!("{sign}{:.1}M KM", a / 1e9 * 1000.0)
    }
}

pub fn speed(v: f64) -> String {
    if v.abs() < 10.0 { format!("{:.1} M/S", v) } else { format!("{:.0} M/S", v) }
}

/// Duration: 12S, 4:32, 1H 23M, 2D 3H.
pub fn dur(s: f64) -> String {
    let s = s.max(0.0);
    if s < 60.0 {
        format!("{:.0}S", s.floor())
    } else if s < 3600.0 {
        format!("{}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32)
    } else if s < 86400.0 {
        format!("{}H {:02}M", (s / 3600.0) as u32, ((s % 3600.0) / 60.0) as u32)
    } else {
        format!("{}D {}H", (s / 86400.0) as u32, ((s % 86400.0) / 3600.0) as u32)
    }
}

/// Mission clock: T+00:01:23 or T+2D 03:04:05.
pub fn met(s: f64) -> String {
    let neg = s < 0.0;
    let s = s.abs();
    let d = (s / 86400.0) as u32;
    let h = ((s % 86400.0) / 3600.0) as u32;
    let m = ((s % 3600.0) / 60.0) as u32;
    let sec = (s % 60.0) as u32;
    let sign = if neg { "T-" } else { "T+" };
    if d > 0 { format!("{sign}{d}D {h:02}:{m:02}:{sec:02}") } else { format!("{sign}{h:02}:{m:02}:{sec:02}") }
}
