//! Простой текстовый формат данных профилей.
//!
//! Формат:
//!   v <скорость>       по умолчанию 1500
//!   q <качество>       по умолчанию 95
//!   angle <градусы>   по умолчанию 0 (Edit4 в оригинале)
//!   profile <x0> <y0> <x1> <y1>
//!   pick <trace> <time>
//!   ...
//!   end
//! Пустые строки и строки, начинающиеся с `#`, игнорируются.

use crate::model::{Pick, Profile, Settings};

pub fn parse(input: &str) -> Result<(Vec<Profile>, Settings), String> {
    let mut settings = Settings::default();
    let mut profiles = Vec::new();
    let mut cur: Option<Profile> = None;

    for (lineno, raw) in input.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts[0] {
            "v" if parts.len() == 2 => {
                settings.velocity = parts[1].parse().map_err(|_| e(lineno))?
            }
            "q" if parts.len() == 2 => {
                settings.quality = parts[1].parse().map_err(|_| e(lineno))?
            }
            "angle" if parts.len() == 2 => {
                settings.angle = parts[1].parse().map_err(|_| e(lineno))?
            }
            "profile" if parts.len() == 5 => {
                if let Some(p) = cur.take() {
                    profiles.push(p);
                }
                let id = profiles.len();
                cur = Some(Profile {
                    id,
                    x0: parts[1].parse().map_err(|_| e(lineno))?,
                    y0: parts[2].parse().map_err(|_| e(lineno))?,
                    x1: parts[3].parse().map_err(|_| e(lineno))?,
                    y1: parts[4].parse().map_err(|_| e(lineno))?,
                    picks: Vec::new(),
                });
            }
            "pick" if parts.len() == 3 => {
                let p = cur.as_mut().ok_or_else(|| e(lineno))?;
                p.picks.push(Pick {
                    trace: parts[1].parse().map_err(|_| e(lineno))?,
                    time: parts[2].parse().map_err(|_| e(lineno))?,
                });
            }
            "end" => {
                if let Some(p) = cur.take() {
                    profiles.push(p);
                }
            }
            _ => return Err(e(lineno)),
        }
    }
    if let Some(p) = cur {
        profiles.push(p);
    }
    Ok((profiles, settings))
}

/// Сериализует готовое состояние в строку, которую понимает `parse`.
/// Используется для ввода/вывода готовой строки: скопировал — вставил — применил.
pub fn serialize(profiles: &[Profile], settings: &Settings) -> String {
    let mut s = String::new();
    s.push_str("# Fold — готовая строка данных\n");
    s.push_str(&format!("v {}\n", fmt_num(settings.velocity)));
    s.push_str(&format!("q {}\n", fmt_num(settings.quality)));
    s.push_str(&format!("angle {}\n", fmt_num(settings.angle)));
    for p in profiles {
        s.push_str(&format!(
            "profile {} {} {} {}\n",
            fmt_num(p.x0),
            fmt_num(p.y0),
            fmt_num(p.x1),
            fmt_num(p.y1)
        ));
        for pick in &p.picks {
            s.push_str(&format!(
                "pick {} {}\n",
                fmt_num(pick.trace),
                fmt_num(pick.time)
            ));
        }
        s.push_str("end\n");
    }
    s
}

fn fmt_num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{:.0}", v)
    } else {
        format!("{:.6}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn e(n: usize) -> String {
    format!("syntax error on line {}", n + 1)
}

pub fn to_pgm(grid: &crate::grid::MapGrid, path: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "P5\n{} {}\n65535", grid.width, grid.height)?;
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &v in &grid.data {
        if v.is_finite() {
            min = min.min(v);
            max = max.max(v);
        }
    }
    let span = (max - min).max(1.0);
    for &v in &grid.data {
        let p = if v.is_finite() {
            ((v - min) / span * 65535.0) as u16
        } else {
            0
        };
        f.write_all(&p.to_be_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_demo() {
        let (p, s) = parse(
            "# comment\nv 1600\nq 90\nangle 30\nprofile 0 0 1000 0\npick 1 0.5\npick 10 0.6\nend\n",
        )
        .unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].picks.len(), 2);
        assert_eq!(s.velocity, 1600.0);
        assert_eq!(s.quality, 90.0);
        assert_eq!(s.angle, 30.0);
    }

    #[test]
    fn bad_line() {
        assert!(parse("oops 1 2\n").is_err());
    }

    #[test]
    fn serialize_roundtrip() {
        let (p, s) = parse(
            "v 1200\nq 95\nangle 12.5\nprofile 100 200 3100 200\npick 1 3.465\npick 10 3.52\nend\n",
        )
        .unwrap();
        let text = serialize(&p, &s);
        let (p2, s2) = parse(&text).unwrap();
        assert_eq!(s2.velocity, s.velocity);
        assert_eq!(s2.quality, s.quality);
        assert_eq!(s2.angle, s.angle);
        assert_eq!(p2.len(), p.len());
        assert_eq!(p2[0].picks, p[0].picks);
        assert_eq!(p2[0].x1, p[0].x1);
    }
}
