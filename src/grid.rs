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

/// Внутренняя форма `0x466250` с уже вычисленными `cos(theta)`/`sin(theta)`.
/// В сетке 301×150 (и особенно 1000×500) тригонометрия для каждой ячейки
/// недопустимо дорога, поэтому `sample_grid` считает её один раз.
#[inline]
fn horizon_depth_ctst(x: f32, y: f32, ct: f64, st: f64) -> f32 {
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

/// Точный аналог `0x466250` для одной точки.
///
/// `x`, `y` — координаты карты в метрах (`f32`, как в original),
/// `theta` — угол в радианах. Возвращает глубину `H` в метрах.
#[inline]
pub fn horizon_depth(x: f32, y: f32, theta: f64) -> f32 {
    horizon_depth_ctst(x, y, theta.cos(), theta.sin())
}

/// Угол поверхности из `0x46641C`: θ = (Edit4 − 90°) · π/180.
#[inline]
pub fn surface_theta(settings: &Settings) -> f64 {
    (settings.angle - 90.0) * std::f64::consts::PI / 180.0
}

#[inline]
fn theta_rad(settings: &Settings) -> f64 {
    surface_theta(settings)
}

/// Быстрый генератор Delphi из `0x402BC8` (`System.Random`).
///
/// ```text
/// seed = seed * $08088405 + 1  (mod 2^32)
/// r    = seed * 2^-32          (`[0x46A068]` = 2.3283064365386963e-10)
/// ```
/// Используется `0x466980` при синтезе отсчётов временного разреза.
#[derive(Debug, Clone)]
pub struct DelphiRandom {
    seed: u32,
}

impl DelphiRandom {
    #[inline]
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.seed
    }

    #[inline]
    pub fn next_f64(&mut self) -> f64 {
        f64::from(self.next_u32()) * (1.0 / 4_294_967_296.0)
    }

    /// Нормальное распределение из `0x423FF4` (Box–Muller):
    /// `u = 2·Random−1`, `r = u1²+u2² < 1`,
    /// `g = sqrt(−2·ln(r)/r)·u1`, результат `g·sigma + mean`.
    pub fn normal(&mut self, sigma: f64, mean: f64) -> f64 {
        loop {
            let u1 = 2.0 * self.next_f64() - 1.0;
            let u2 = 2.0 * self.next_f64() - 1.0;
            let r = u1 * u1 + u2 * u2;
            if r > 0.0 && r < 1.0 {
                return (-2.0 * r.ln() / r).sqrt() * u1 * sigma + mean;
            }
        }
    }
}

/// Сигма шума синтетического разреза из `0x423FF4` (ext80 при вызове).
pub const SECTION_NOISE_SIGMA: f64 = 0.006;
/// Сдвиг времени отсчёта из `0x467444` (ext80 = 0.004).
pub const SECTION_TIME_SHIFT: f64 = 0.004;
/// Число отсчётов на трассу из заголовка буфера `0x465C23` (`0x12C`).
pub const SECTION_SAMPLES: usize = 300;

/// Число трасс временного разреза из `0x466980`:
/// `n = trunc(len / 200) + 1`, с поправкой `0x466AF7`, если последний шаг
/// вылезает за длину профиля. Оригинал ограничивает буфер 100 трассами.
pub fn section_trace_count(profile: &Profile) -> usize {
    let len = profile.length();
    let mut n = (len / 200.0).trunc() as usize + 1;
    if n > 1 && (n - 1) as f64 * 200.0 >= len + 200.0 {
        n -= 1;
    }
    n.clamp(1, 100)
}

/// Время пика по вертикальной координате Image2 из `0x468780`:
/// `t = t_min + Y/Height·(t_max−t_min)`. Ось времени в original направлена
/// вниз — та же ориентация, что и у отсчётов в `0x465D68`.
#[inline]
pub fn section_time_from_y(y: f64, height: f64, bounds: (f64, f64)) -> f64 {
    let k = (y / height).clamp(0.0, 1.0);
    bounds.0 + k * (bounds.1 - bounds.0)
}

/// Общая выборка аналитической поверхности.
///
/// `include_end` — публичная сетка (`build_map`) включает обе границы карты,
/// тогда как внутренняя сетка оригинала (`0x46641C`) использует
/// `x=(i−1)·8000/N`, `y=(j−1)·4000/ny` и правую границу не включает.
fn sample_grid(w: usize, h: usize, settings: &Settings, include_end: bool) -> MapGrid {
    let mut data = vec![f32::NAN; w * h];
    let theta = theta_rad(settings);
    let ct = theta.cos();
    let st = theta.sin();
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
            data[row + gx] = horizon_depth_ctst(x, y, ct, st);
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
    let n = section_trace_count(profile);
    let step = settings.step.max(1.0);
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for i in 0..n {
        // 0x466980 шагает ровно по 200 м от начала профиля, а не делит
        // отрезок на n−1 равных частей.
        let k = if len <= 0.0 {
            0.0
        } else {
            ((i as f64 * step) / len).min(1.0)
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
    // 0x46745C / 0x467468: Grafana-заголовок буфера сужает диапазон.
    min += 0.084;
    max -= 0.02;
    if max - min < 1e-6 {
        let mid = (min + max) * 0.5;
        min = mid - 0.05;
        max = mid + 0.05;
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
        // Небольшое изменение скорости: при слишком сильном уменьшении
        // raw-диапазон становится меньше поправок и original включает
        // fallback min=mid−0.05/max=mid+0.05.
        let factor = 1.1;
        let b = Settings {
            velocity: a.velocity * factor,
            ..a.clone()
        };
        let (amin, amax) = profile_time_bounds(&p, &a);
        let (bmin, bmax) = profile_time_bounds(&p, &b);
        assert!(amin.is_finite() && amax > amin);
        // 0x46745C/0x467468 сужают диапазон на константы до сравнения:
        // raw_min = amin − 0.084, raw_max = amax + 0.02. Рост V в `factor`
        // раз делит raw-диапазон на `factor`, после чего поправки применяются
        // снова.
        assert!((bmin - ((amin - 0.084) / factor + 0.084)).abs() < 1e-6);
        assert!((bmax - ((amax + 0.02) / factor - 0.02)).abs() < 1e-6);
    }

    #[test]
    fn delphi_random_matches_binary_lcg() {
        let mut r = DelphiRandom::new(0);
        // 1-й вызов: seed = 0*0x08088405+1 = 1 → 1*2^-32.
        assert_eq!(r.next_u32(), 1);
        assert!((r.next_f64() - 2.0f64.powi(-32) * 0x0808_8406 as f64).abs() < 1e-15);

        // Повторяемость: одинаковое зерно даёт одинаковую нормальную-seq.
        let mut a = DelphiRandom::new(12345);
        let mut b = DelphiRandom::new(12345);
        for _ in 0..32 {
            let x = a.normal(SECTION_NOISE_SIGMA, SECTION_TIME_SHIFT);
            let y = b.normal(SECTION_NOISE_SIGMA, SECTION_TIME_SHIFT);
            assert_eq!(x.to_bits(), y.to_bits());
        }
    }

    #[test]
    fn section_trace_count_follows_original_step() {
        let mk = |len: f64| Profile {
            id: 0,
            x0: 0.0,
            y0: 0.0,
            x1: len,
            y1: 0.0,
            picks: Vec::new(),
        };
        assert_eq!(section_trace_count(&mk(0.0)), 1);
        assert_eq!(section_trace_count(&mk(200.0)), 2);
        assert_eq!(section_trace_count(&mk(199.0)), 1);
        assert_eq!(section_trace_count(&mk(3000.0)), 16);
        assert_eq!(section_trace_count(&mk(100_000.0)), 100);
    }

    #[test]
    fn section_time_grows_downward() {
        let bounds = (1.0, 3.0);
        assert!((section_time_from_y(0.0, 200.0, bounds) - 1.0).abs() < 1e-12);
        assert!((section_time_from_y(100.0, 200.0, bounds) - 2.0).abs() < 1e-12);
        assert!((section_time_from_y(200.0, 200.0, bounds) - 3.0).abs() < 1e-12);
        // Клик за пределами Image2 не должен ломать время.
        assert_eq!(section_time_from_y(-10.0, 200.0, bounds), 1.0);
        assert_eq!(section_time_from_y(300.0, 200.0, bounds), 3.0);
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
