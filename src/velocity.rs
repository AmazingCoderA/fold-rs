//! Пересчёт времени отражения в глубину.
//!
//! В оригинале используется средняя скорость `V ср` (Edit2, 1500 м/с,
//! глобал `0x46CC30`). В `0x466980` время получается как `t = 2 * H / V`:
//! константа `2.0` находится по адресу `0x467428`, а `H` приходит из
//! `0x466250`. Обратное преобразование `H = V * t / 2` совпадает с
//! физикой однократного отражения.

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
