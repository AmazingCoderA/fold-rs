//! Построение структурной карты по восстановленному алгоритму 2007 года.
//!
//! Точный источник — локальный reverse-engineering отчёт
//! `analysis/map_reverse_report.md`:
//!
//! * `0x466250` — аналитическая поверхность `H(x, y, θ)`;
//! * `0x46641C` — сетка карты, min/max и 21 цветовая полоса;
//! * `0x466980` — временной разрез, где `t = 2H/V`.
//!
//! Важно: в original качество изображения и пики профилей **не участвуют** в
//! расчёте карты. `0.4 / 0.6` — коэффициенты эллиптического расстояния,
//! `2000` — база глубины, а не радиус отсечения.

use crate::model::{Profile, Settings, MAP_HEIGHT_M, MAP_WIDTH_M};

/// Карта глубин. `data[y*width+x]`, значение = `H` в метрах.
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

/// Delphi `Round` из `0x402970`: функция выставляет FPU `RC=11`
/// (`or CW, 0x0F00`) и делает `fistp qword`, то есть **усекает к нулю**.
#[inline]
fn delphi_round(v: f64) -> f32 {
    v.trunc() as i64 as f32
}

/// Точный аналог `0x466250`.
///
/// `x`, `y` — координаты карты в метрах (`f32`, как в original),
/// `theta` — угол в радианах. Возвращает глубину `H` в метрах.
pub fn horizon_depth(x: f32, y: f32, theta: f64) -> f32 {
    let ct = theta.cos();
    let st = theta.sin();

    // 0x466260..0x4662D0 — поворот точки.
    let xr = delphi_round(ct * f64::from(x)) - delphi_round(st * f64::from(y));
    let yr = delphi_round(st * f64::from(x)) + delphi_round(ct * f64::from(y));

    // 0x4662D1..0x466358 — поворот центра (8000, 4000) из 0x46AE48/0x46AE4C.
    let xc = delphi_round(ct * MAP_WIDTH_M) - delphi_round(st * MAP_HEIGHT_M);
    let yc = delphi_round(st * MAP_WIDTH_M) + delphi_round(ct * MAP_HEIGHT_M);

    // 0x46635C..0x46637A — центрирование: делить на 2.0 (0x4663E4).
    let dx = xr - xc / 2.0f32;
    let dy = yr - yc / 2.0f32;

    // 0x46637C..0x46639A: 0.4*dx² + 0.6*dy², abs, sqrt (0x4663E8/0x4663F4).
    let d = (0.4f64 * f64::from(dx) * f64::from(dx) + 0.6f64 * f64::from(dy) * f64::from(dy))
        .abs()
        .sqrt() as f32;

    // 0x46639E..0x4663BF: |dx-210| + 3.4*d, sqrt, *26, d/...
    let denom = 26.0f64 * ((f64::from(dx) - 210.0f64).abs() + 3.4f64 * f64::from(d)).sqrt();
    let q = (f64::from(d) / denom) as f32;

    // 0x4663C3..0x4663D2: 250*q + 2000 (0x466414 / 0x466418).
    250.0f32 * q + 2000.0f32
}

/// Угол поверхности из `0x46641C`: θ = (Edit4 − 90°) · π/180.
#[inline]
fn theta_rad(settings: &Settings) -> f64 {
    (settings.angle - 90.0) * std::f64::consts::PI / 180.0
}

/// Общая выборка аналитической поверхности.
///
/// `include_end` — публичная сетка (`build_map`) включает обе границы карты,
/// тогда как внутренняя сетка оригинала (`0x46641C`) использует
/// `x=(i−1)·8000/N`, `y=(j−1)·4000/ny` и правую границу не включает.
fn sample_grid(w: usize, h: usize, settings: &Settings, include_end: bool) -> MapGrid {
    let mut data = vec![f32::NAN; w * h];
    let theta = theta_rad(settings);
    let dx = if include_end {
        MAP_WIDTH_M / (w - 1) as f64
    } else {
        MAP_WIDTH_M / w as f64
    };
    let dy = if include_end {
        MAP_HEIGHT_M / (h - 1) as f64
    } else {
        MAP_HEIGHT_M / h as f64
    };
    for gy in 0..h {
        let y = (gy as f64 * dy) as f32;
        let row = gy * w;
        for gx in 0..w {
            let x = (gx as f64 * dx) as f32;
            data[row + gx] = horizon_depth(x, y, theta);
        }
    }
    MapGrid {
        width: w,
        height: h,
        data,
    }
}

/// Строит карту. Публичные размеры остаются `settings.width × settings.height`,
/// как и в рендере PGM/CLI.
///
/// Профили и пики в расчёт карты не входят: `0x46641C` использует только угол
/// из Edit4 и шаг сетки из Edit1 (`Settings::quality`).
pub fn build_map(profiles: &[Profile], settings: &Settings) -> MapGrid {
    let _ = profiles;
    sample_grid(
        settings.width.max(2),
        settings.height.max(2),
        settings,
        true,
    )
}

/// Внутренняя сетка оригинала из `0x46641C` (используется GUI для текстуры).
///
/// `N = trunc(quality·100)+1`, `ny = N·Image1.Height/Image1.Width` (301×150
/// при quality=3). Значения quality ограничены `0x467540` диапазоном 0.1..9.99.
pub fn build_map_native(profiles: &[Profile], settings: &Settings) -> MapGrid {
    let _ = profiles;
    sample_grid(
        settings.map_nodes_x(),
        settings.map_nodes_y(),
        settings,
        false,
    )
}

/// Диапазон времени временного разреза по `0x466980`.
///
/// Вдоль активного профиля шагают по 200 м (`0x46742C`) и считают
/// `t = 2·H/V`; min/max затем задают вертикальный масштаб `Image2`.
pub fn profile_time_bounds(profile: &Profile, settings: &Settings) -> (f64, f64) {
    let theta = theta_rad(settings);
    let dx = profile.x1 - profile.x0;
    let dy = profile.y1 - profile.y0;
    let len = dx.hypot(dy);
    let n = ((len / 200.0).trunc() as usize + 1).clamp(1, 10_000);
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for i in 0..n {
        let k = if n == 1 {
            0.0
        } else {
            i as f64 / (n - 1) as f64
        };
        let x = (profile.x0 + k * dx) as f32;
        let y = (profile.y0 + k * dy) as f32;
        let t = 2.0 * f64::from(horizon_depth(x, y, theta)) / settings.velocity.max(1.0);
        if t.is_finite() {
            min = min.min(t);
            max = max.max(t);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return (0.0, 1.0);
    }
    if max - min < 1e-6 {
        min -= 0.05;
        max += 0.05;
    }
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Pick;

    #[test]
    fn horizon_formula_matches_reference() {
        // θ = 0: xr=x, yr=y, xc=8000, yc=4000 ⇒ dx=-4000, dy=-2000.
        // d = sqrt(0.4*16e6 + 0.6*4e6) = sqrt(8.8e6).
        let h = horizon_depth(0.0, 0.0, 0.0);
        assert!((h - 2238.5).abs() < 1.0, "H(0,0) = {h}");
        // База 2000 м и масштаб 250 дают узкий диапазон вокруг 2000.
        assert!(h > 2000.0 && h < 2500.0);
    }

    #[test]
    fn delphi_round_truncates_toward_zero() {
        assert_eq!(delphi_round(2.9), 2.0);
        assert_eq!(delphi_round(-2.9), -2.0);
    }

    #[test]
    fn grid_is_finite_and_uses_public_dimensions() {
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
        let s = Settings::default();
        let g = build_map(&[pr], &s);
        assert_eq!((g.width, g.height), (s.width, s.height));
        assert_eq!(g.data.len(), g.width * g.height);
        assert!(g.data.iter().all(|v| v.is_finite()));
        // angle=0 ⇒ θ=-90°; поверхность всё ещё конечна и отлична от константы.
        assert!(g.data.iter().any(|v| (*v - g.data[0]).abs() > 1.0));
    }

    #[test]
    fn native_grid_uses_original_node_counts() {
        let s = Settings::default();
        let g = build_map_native(&[], &s);
        assert_eq!((g.width, g.height), (s.map_nodes_x(), s.map_nodes_y()));
        assert_eq!((g.width, g.height), (301, 150));
        assert!(g.data.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn profile_time_bounds_follow_velocity() {
        let p = Profile {
            id: 0,
            x0: 100.0,
            y0: 100.0,
            x1: 3100.0,
            y1: 100.0,
            picks: Vec::new(),
        };
        let a = Settings::default();
        let b = Settings {
            velocity: a.velocity * 2.0,
            ..a.clone()
        };
        let (amin, amax) = profile_time_bounds(&p, &a);
        let (bmin, bmax) = profile_time_bounds(&p, &b);
        assert!(amin.is_finite() && amax > amin);
        assert!((bmin - amin * 0.5).abs() < 1e-6);
        assert!((bmax - amax * 0.5).abs() < 1e-6);
    }

    #[test]
    fn angle_changes_surface() {
        let profiles: Vec<Profile> = Vec::new();
        let a = Settings {
            angle: 0.0,
            ..Default::default()
        };
        let b = Settings {
            angle: 45.0,
            ..Default::default()
        };
        let ga = build_map(&profiles, &a);
        let gb = build_map(&profiles, &b);
        assert!(ga
            .data
            .iter()
            .zip(&gb.data)
            .any(|(x, y)| (x - y).abs() > 1.0));
    }
}
