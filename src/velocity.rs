//! Пересчёт времени отражения в глубину.
//!
//! В оригинале используется средняя скорость `V ср` (Edit2, 1500 м/с).
//! Для однократного отражения глубина: H = V * t / 2.
//! В коде это видно по константе 2.0 (0x467428) и множителю угла π/180
//! при переходе к полярным координатам профиля.

/// Время (с) -> глубина (м).
#[inline]
pub fn time_to_depth(time_s: f64, velocity_mps: f64) -> f64 {
    velocity_mps * time_s * 0.5
}

/// Глубина (м) -> время (с).
#[inline]
pub fn depth_to_time(depth_m: f64, velocity_mps: f64) -> f64 {
    2.0 * depth_m / velocity_mps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let d = time_to_depth(2.0, 1500.0);
        assert_eq!(d, 1500.0);
        assert_eq!(depth_to_time(d, 1500.0), 2.0);
    }
}
