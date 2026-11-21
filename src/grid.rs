//! Построение структурной карты по профилям.
//!
//! Оригинал: 0x466980 готовил для каждого профиля отсчёты с шагом 200 м,
//! 0x465D68 формировал сетку, 0x466250 считал вес качества с константами
//! 0.4 / 0.6 и радиусом 2000 м (0x466418).

use crate::model::{Profile, Settings, MAP_HEIGHT_M, MAP_WIDTH_M};
use crate::velocity::time_to_depth;

/// Карта глубин. `data[y*width+x]`, NaN = нет данных.
pub struct MapGrid {
    pub width: usize,
    pub height: usize,
    pub data: Vec<f32>,
}

impl MapGrid {
    #[inline]
    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.data[y * self.width + x]
    }
}

#[derive(Clone, Copy)]
struct Sample {
    x: f64,
    y: f64,
    depth: f64,
    weight: f64,
}

#[inline]
fn quality_weight(quality: f64) -> f64 {
    // 0x4663F4 = 0.6, 0x4663E8 = 0.4;quality в %
    (0.6 + 0.4 * (quality / 100.0).clamp(0.0, 1.0)) * 0.5
}

/// Линейная интерполяция пиков по номеру трассы.
fn depth_at(profile: &Profile, trace: f64, settings: &Settings) -> Option<f64> {
    let p = &profile.picks;
    if p.is_empty() {
        return None;
    }
    if p.len() == 1 {
        return Some(time_to_depth(p[0].time, settings.velocity));
    }
    let mut i = 0;
    while i + 1 < p.len() && p[i + 1].trace < trace {
        i += 1;
    }
    if i + 1 >= p.len() {
        return Some(time_to_depth(p[p.len() - 1].time, settings.velocity));
    }
    let a = p[i];
    let b = p[i + 1];
    let span = b.trace - a.trace;
    let t = if span.abs() < f64::EPSILON {
        0.0
    } else {
        ((trace - a.trace) / span).clamp(0.0, 1.0)
    };
    let time = a.time + (b.time - a.time) * t;
    Some(time_to_depth(time, settings.velocity))
}

/// Строит карту. Сложность O(узлы * отсчёты),但没有 allocate в цикле.
pub fn build_map(profiles: &[Profile], settings: &Settings) -> MapGrid {
    let (w, h) = (settings.width, settings.height);
    let mut data = vec![f32::NAN; w * h];
    if profiles.is_empty() || w == 0 || h == 0 {
        return MapGrid {
            width: w,
            height: h,
            data,
        };
    }

    let mut samples: Vec<Sample> = Vec::with_capacity(512);
    for pr in profiles {
        let len = pr.length();
        if len <= 0.0 || pr.picks.is_empty() {
            continue;
        }
        let n = ((len / settings.step) as usize + 1).max(2);
        let ux = (pr.x1 - pr.x0) / len;
        let uy = (pr.y1 - pr.y0) / len;
        let first = pr.picks[0].trace;
        let last = pr.picks[pr.picks.len() - 1].trace;
        let qw = quality_weight(settings.quality);

        for i in 0..n {
            let s = (i as f64) * settings.step;
            let trace = first + (last - first) * (s / len);
            if let Some(depth) = depth_at(pr, trace, settings) {
                samples.push(Sample {
                    x: pr.x0 + ux * s,
                    y: pr.y0 + uy * s,
                    depth,
                    weight: qw,
                });
            }
        }
    }
    if samples.is_empty() {
        return MapGrid {
            width: w,
            height: h,
            data,
        };
    }

    let dx = MAP_WIDTH_M / (w - 1).max(1) as f64;
    let dy = MAP_HEIGHT_M / (h - 1).max(1) as f64;
    let radius2 = 2000.0f64 * 2000.0; // 0x466418
    let scale2 = 40_000.0f64; // (200 м)^2

    for gy in 0..h {
        let my = gy as f64 * dy;
        let row = gy * w;
        for gx in 0..w {
            let mx = gx as f64 * dx;
            let mut num = 0.0f64;
            let mut den = 0.0f64;
            for sm in &samples {
                let ex = mx - sm.x;
                let ey = my - sm.y;
                let r2 = ex * ex + ey * ey;
                if r2 > radius2 {
                    continue;
                }
                let wgt = sm.weight / (1.0 + r2 / scale2);
                num += sm.depth * wgt;
                den += wgt;
            }
            if den > 0.0 {
                data[row + gx] = (num / den) as f32;
            }
        }
    }

    MapGrid {
        width: w,
        height: h,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Pick;

    #[test]
    fn depth_conversion_and_interpolation() {
        let pr = Profile {
            id: 0,
            x0: 0.0,
            y0: 0.0,
            x1: 4000.0,
            y1: 0.0,
            picks: vec![
                Pick {
                    trace: 0.0,
                    time: 1.0,
                },
                Pick {
                    trace: 100.0,
                    time: 2.0,
                },
            ],
        };
        let s = Settings::default();
        // H = V*t/2
        assert_eq!(depth_at(&pr, 0.0, &s), Some(750.0));
        assert_eq!(depth_at(&pr, 100.0, &s), Some(1500.0));
        // середина: t=1.5 c -> 1125 m
        assert_eq!(depth_at(&pr, 50.0, &s), Some(1125.0));
    }

    #[test]
    fn grid_is_finite_near_profile() {
        let pr = Profile {
            id: 0,
            x0: 0.0,
            y0: 2000.0,
            x1: 8000.0,
            y1: 2000.0,
            picks: vec![
                Pick {
                    trace: 0.0,
                    time: 1.0,
                },
                Pick {
                    trace: 100.0,
                    time: 1.0,
                },
            ],
        };
        let g = build_map(&[pr], &Settings::default());
        assert!(g.get(20, 10).is_finite());
        assert_eq!(g.data.len(), g.width * g.height);
    }
}
