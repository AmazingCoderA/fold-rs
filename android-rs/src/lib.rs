//! JNI-мост для Android-приложения Fold.
//!
//! Формат входного массива legacy-метода `buildMap`:
//!   [n_profiles,
//!    x0,y0,x1,y1,n_picks, (trace,time)*n_picks,
//!    ...]
//! Возврат legacy-метода:
//!   [width, height, depth0, depth1, ...]  (нет данных = -9999.0)
//!
//! Новые методы возвращают готовые ARGB-буферы:
//!   buildMapPixels → [800, 400, argb...]
//!   buildSectionPixels → [800, 201, t_min_bits, t_max_bits, n_traces, argb...]

use fold::grid::{build_map, build_map_native, build_section, map_color_classic, SECTION_SAMPLES};
use fold::model::{
    Pick, Profile, Settings, MAP_IMG_H, MAP_IMG_W, SECTION_IMG_H, SECTION_IMG_W,
};
use jni::objects::{JClass, JFloatArray};
use jni::sys::{jfloat, jfloatArray, jint, jintArray};
use jni::JNIEnv;

const NODATA: f32 = -9999.0;
const ARGB_BLACK: i32 = -16777216; // 0xFF000000
const ARGB_WHITE: i32 = -1; // 0xFFFFFFFF
const ARGB_GRID: i32 = 0xFFD2_D2D2u32 as i32; // $D2D2D2 из 0x467739

#[inline]
fn argb(rgb: [u8; 3]) -> i32 {
    (0xFF00_0000u32 | (u32::from(rgb[0]) << 16) | (u32::from(rgb[1]) << 8) | u32::from(rgb[2]))
        as i32
}

fn make_settings(velocity: jfloat, quality: jfloat, angle: jfloat) -> Settings {
    Settings {
        velocity: (velocity as f64).clamp(1000.0, 7000.0),
        quality: quality as f64,
        angle: (angle as f64).clamp(0.0, 360.0),
        ..Settings::default()
    }
}

fn grid_bounds(data: &[f32]) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &v in data {
        if v.is_finite() {
            min = min.min(v);
            max = max.max(v);
        }
    }
    if !min.is_finite() {
        return (0.0, 1.0);
    }
    if (max - min).abs() < 1e-6 {
        max = min + 1.0;
    }
    (min, max)
}

/// Дословная растеризация `0x46641C` в буфер Image1 800×400, та же что и в GUI.
fn map_pixels(settings: &Settings) -> Vec<i32> {
    let grid = build_map_native(&[], settings);
    let (min, max) = grid_bounds(&grid.data);
    let span = (max - min).max(1e-6);

    let mut img = vec![argb(map_color_classic(0.0)); MAP_IMG_W * MAP_IMG_H];
    let half = 10.0_f64 / settings.quality.clamp(0.1, 9.99);
    let back = half.trunc() as i32;
    let front = (2.0_f64 * half).trunc() as i32;

    for gx in 0..grid.width {
        let sx = (gx * MAP_IMG_W / grid.width) as i32;
        for gy in 0..grid.height {
            let d = grid.data[gy * grid.width + gx];
            if !d.is_finite() {
                continue;
            }
            let sy = MAP_IMG_H as i32 - (gy * MAP_IMG_H / grid.height) as i32;
            let u = ((d - min) / span).clamp(0.0, 1.0);
            let color = argb(map_color_classic(u));
            // Delphi Canvas.FillRect: right/bottom не включаются.
            let x0 = (sx - back).max(0) as usize;
            let x1 = (sx + front).min(MAP_IMG_W as i32) as usize;
            let y0 = (sy - back).max(0) as usize;
            let y1 = (sy + front).min(MAP_IMG_H as i32) as usize;
            for y in y0..y1 {
                img[y * MAP_IMG_W + x0..y * MAP_IMG_W + x1].fill(color);
            }
        }
    }
    img
}

#[inline]
fn put_pixel(img: &mut [i32], x: i32, y: i32, color: i32) {
    if x >= 0 && y >= 0 && (x as usize) < SECTION_IMG_W && (y as usize) < SECTION_IMG_H {
        img[y as usize * SECTION_IMG_W + x as usize] = color;
    }
}

#[inline]
fn hline(img: &mut [i32], x0: i32, x1: i32, y: i32, color: i32) {
    for x in x0.min(x1)..=x0.max(x1) {
        put_pixel(img, x, y, color);
    }
}

#[inline]
fn vline(img: &mut [i32], x: i32, y0: i32, y1: i32, color: i32) {
    for y in y0.min(y1)..=y0.max(y1) {
        put_pixel(img, x, y, color);
    }
}

/// Один отрезок `TCanvas.LineTo`; отрицательные амплитуды соединяют предыдущую точку.
fn line(img: &mut [i32], x0: i32, y0: i32, x1: i32, y1: i32, color: i32) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        put_pixel(img, x, y, color);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Дословная растеризация `0x465D68` в буфер Image2 800×201, та же что и в GUI.
fn section_pixels(data: &fold::grid::SectionData) -> Vec<i32> {
    let mut img = vec![ARGB_WHITE; SECTION_IMG_W * SECTION_IMG_H];
    let w = SECTION_IMG_W as f32;
    let h = SECTION_IMG_H as f32;
    let span = data.t_max - data.t_min;
    let scale_t = h / span;
    let grid_y = h / 10.0; // 0x466150 = 10
    let x_step = w / (data.n as f32 + 1.0);

    for i in 1..=data.n {
        let x = (i as f32 * x_step).trunc() as i32;
        vline(&mut img, x, 1, SECTION_IMG_H as i32 - 1, ARGB_GRID);
    }
    for i in 1..=11 {
        let y = (i as f32 * grid_y).trunc() as i32;
        hline(&mut img, 0, SECTION_IMG_W as i32 - 1, y, ARGB_GRID);
    }

    let mut sum = 1e-8f32; // 0x322BCC77
    for &s in data.samples.iter().take(SECTION_SAMPLES) {
        sum += s.wrapping_mul(s) as f32;
    }
    let rms = f64::from((sum / SECTION_SAMPLES as f32).sqrt());

    for trace in 0..data.n {
        let x0 = ((trace + 1) as f32 * x_step).trunc() as i32;
        let mut cur = (x0, 0i32); // 0x465FE1: MoveTo(x0, 0)
        for j in 0..SECTION_SAMPLES {
            let s = data.samples[trace * SECTION_SAMPLES + j];
            let tj = data.t_min + j as f32 / SECTION_SAMPLES as f32 * span;
            let y = ((tj - data.t_min) * scale_t).trunc() as i32;
            let amp = (0.12f64 / rms * f64::from(s) * f64::from(x_step)).trunc() as i32;
            let x1 = x0 + amp;
            if amp < 0 {
                // 0x46608B: LineTo без MoveTo.
                line(&mut img, cur.0, cur.1, x1, y, ARGB_BLACK);
            } else {
                // 0x4676CC: MoveTo(x0,y) + LineTo(x1,y).
                hline(&mut img, x0, x1, y, ARGB_BLACK);
            }
            cur = (x1, y);
        }
    }
    img
}

fn int_array(env: &JNIEnv<'_>, out: &[i32]) -> jintArray {
    match env.new_int_array(out.len() as i32) {
        Ok(arr) => {
            if env.set_int_array_region(&arr, 0, out).is_err() {
                return std::ptr::null_mut();
            }
            arr.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)] // JNI гарантирует не-null указатель.
pub extern "system" fn Java_com_fold_app_FoldNative_buildMap<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    data: jfloatArray,
    velocity: jfloat,
    quality: jfloat,
) -> jfloatArray {
    let data = unsafe { JFloatArray::from_raw(data) };
    let len = match env.get_array_length(&data) {
        Ok(n) => n as usize,
        Err(_) => return std::ptr::null_mut(),
    };
    let mut raw = vec![0.0f32; len];
    if env.get_float_array_region(&data, 0, &mut raw).is_err() {
        return std::ptr::null_mut();
    }

    let mut pos = 0usize;
    let n_profiles = if pos < len { raw[pos] as usize } else { 0 };
    pos += 1;

    let mut profiles = Vec::with_capacity(n_profiles);
    for id in 0..n_profiles {
        if pos + 5 > len {
            break;
        }
        let x0 = raw[pos] as f64;
        let y0 = raw[pos + 1] as f64;
        let x1 = raw[pos + 2] as f64;
        let y1 = raw[pos + 3] as f64;
        let n_picks = raw[pos + 4] as usize;
        pos += 5;
        let mut picks = Vec::with_capacity(n_picks);
        for _ in 0..n_picks {
            if pos + 2 > len {
                break;
            }
            picks.push(Pick {
                trace: raw[pos] as f64,
                time: raw[pos + 1] as f64,
            });
            pos += 2;
        }
        profiles.push(Profile {
            id,
            x0,
            y0,
            x1,
            y1,
            picks,
        });
    }

    let mut settings = make_settings(velocity, quality, 0.0);
    settings.width = 41;
    settings.height = 21;
    let grid = build_map(&profiles, &settings);

    let mut out = Vec::with_capacity(2 + grid.data.len());
    out.push(grid.width as f32);
    out.push(grid.height as f32);
    for &v in &grid.data {
        out.push(if v.is_finite() { v } else { NODATA });
    }

    match env.new_float_array(out.len() as i32) {
        Ok(arr) => {
            if env.set_float_array_region(&arr, 0, &out).is_err() {
                return std::ptr::null_mut();
            }
            arr.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// `[800, 400, argb...]` — structural map exactly as the desktop GUI.
#[no_mangle]
pub extern "system" fn Java_com_fold_app_FoldNative_buildMapPixels<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    velocity: jfloat,
    quality: jfloat,
    angle: jfloat,
) -> jintArray {
    let settings = make_settings(velocity, quality, angle);
    let pixels = map_pixels(&settings);
    let mut out = Vec::with_capacity(2 + pixels.len());
    out.push(MAP_IMG_W as jint);
    out.push(MAP_IMG_H as jint);
    out.extend_from_slice(&pixels);
    int_array(&env, &out)
}

/// `[800, 201, t_min_bits, t_max_bits, n_traces, argb...]` — time section.
#[no_mangle]
pub extern "system" fn Java_com_fold_app_FoldNative_buildSectionPixels<'local>(
    env: JNIEnv<'local>,
    _class: JClass<'local>,
    x0: jfloat,
    y0: jfloat,
    x1: jfloat,
    y1: jfloat,
    velocity: jfloat,
    quality: jfloat,
    angle: jfloat,
) -> jintArray {
    let settings = make_settings(velocity, quality, angle);
    let profile = Profile {
        id: 0,
        x0: x0 as f64,
        y0: y0 as f64,
        x1: x1 as f64,
        y1: y1 as f64,
        picks: Vec::new(),
    };
    let section = build_section(&profile, &settings);
    let pixels = section_pixels(&section);

    let mut out = Vec::with_capacity(5 + pixels.len());
    out.push(SECTION_IMG_W as jint);
    out.push(SECTION_IMG_H as jint);
    out.push(section.t_min.to_bits() as jint);
    out.push(section.t_max.to_bits() as jint);
    out.push(section.n as jint);
    out.extend_from_slice(&pixels);
    int_array(&env, &out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_buffer_is_dfm_image1_size() {
        let settings = make_settings(1500.0, 3.0, 0.0);
        let pixels = map_pixels(&settings);
        assert_eq!(pixels.len(), MAP_IMG_W * MAP_IMG_H);
        // Первый пиксель закрыт прямоугольником FillRect, но всегда непрозрачен.
        assert_ne!((pixels[0] as u32) & 0xFF00_0000u32, 0);
    }

    #[test]
    fn section_buffer_is_dfm_image2_size() {
        let settings = make_settings(1500.0, 3.0, 0.0);
        let profile = Profile {
            id: 0,
            x0: 100.0,
            y0: 100.0,
            x1: 3100.0,
            y1: 100.0,
            picks: Vec::new(),
        };
        let section = build_section(&profile, &settings);
        assert_eq!(section.n, 16);
        assert_eq!(section.samples.len(), 16 * SECTION_SAMPLES);
        let pixels = section_pixels(&section);
        assert_eq!(pixels.len(), SECTION_IMG_W * SECTION_IMG_H);
        assert!(pixels.contains(&ARGB_BLACK));
        assert!(pixels.contains(&ARGB_GRID));
    }
}
