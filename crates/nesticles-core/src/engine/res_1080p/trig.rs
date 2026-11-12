//! Integer sine. Degrees stay in millidegrees so a 2.8° step does not need floats.

pub fn sin_mdeg(mdeg: i32) -> i32 {
    let m = mdeg.rem_euclid(360_000);
    let d0 = m / 1000;
    let frac = m % 1000;
    let a = sin_deg(d0) as i64;
    let b = sin_deg(d0 + 1) as i64;
    (a + (b - a) * i64::from(frac) / 1000) as i32
}

pub fn cos_mdeg(mdeg: i32) -> i32 {
    sin_mdeg(mdeg.saturating_add(90_000))
}

fn sin_deg(deg: i32) -> i32 {
    let d = deg.rem_euclid(360);
    let (neg, x) = if d <= 180 {
        (false, d)
    } else {
        (true, d - 180)
    };
    let x = i64::from(x);
    let num = 4 * x * (180 - x);
    let den = 40500 - x * (180 - x);
    let mut s = if den == 0 { 0 } else { num * 4096 / den };
    if neg {
        s = -s;
    }
    s as i32
}
