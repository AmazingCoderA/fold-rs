//! JNI-мост для Android-приложения Fold.
//!
//! Формат входного массива `data`:
//!   [n_profiles,
//!    x0,y0,x1,y1,n_picks, (trace,time)*n_picks,
//!    ...]
//! Возврат:
//!   [width, height, depth0, depth1, ...]  (нет данных = -9999.0)

use fold::grid::build_map;
use fold::model::{Pick, Profile, Settings};
use jni::objects::{JClass, JFloatArray};
use jni::sys::{jfloat, jfloatArray};
use jni::JNIEnv;

const NODATA: f32 = -9999.0;

#[no_mangle]
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
        profiles.push(Profile { id, x0, y0, x1, y1, picks });
    }

    let mut settings = Settings::default();
    settings.velocity = velocity as f64;
    settings.quality = quality as f64;
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
