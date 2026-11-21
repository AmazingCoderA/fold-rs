//! Простой текстовый формат данных профилей.
//!
//! Формат:
//!   v <скорость>       по умолчанию 1500
//!   q <качество>       по умолчанию 95
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

fn e(n: usize) -> String {
    format!("syntax error on line {}", n + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_demo() {
        let (p, s) =
            parse("# comment\nv 1600\nq 90\nprofile 0 0 1000 0\npick 1 0.5\npick 10 0.6\nend\n")
                .unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].picks.len(), 2);
        assert_eq!(s.velocity, 1600.0);
        assert_eq!(s.quality, 90.0);
    }

    #[test]
    fn bad_line() {
        assert!(parse("oops 1 2\n").is_err());
    }
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
