/// Пик отражающего горизонта на одной трассе.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pick {
    /// Номер трассы (ось X временного разреза).
    pub trace: f64,
    /// Время в секундах (ось `t, с` в оригинале).
    pub time: f64,
}

/// Профиль: отрезок на структурной карте + набор пиков.
/// Координаты в метрах; ось Y в оригинале направлена вниз (0..-4000).
#[derive(Debug, Clone)]
pub struct Profile {
    pub id: usize,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub picks: Vec<Pick>,
}

impl Profile {
    #[inline]
    pub fn length(&self) -> f64 {
        let dx = self.x1 - self.x0;
        let dy = self.y1 - self.y0;
        (dx * dx + dy * dy).sqrt()
    }

    #[inline]
    pub fn azimuth(&self) -> f64 {
        (self.y1 - self.y0).atan2(self.x1 - self.x0)
    }
}

/// Физические размеры карты в метрах (константы 0x46AE48 / 0x46AE4C).
pub const MAP_WIDTH_M: f64 = 8000.0;
pub const MAP_HEIGHT_M: f64 = 4000.0;

/// Размер `Image2` в пикселях из DFM. `0x46641C` считает число узлов карты
/// по формуле `ny = N * Image2.Height / Image2.Width`.
pub const SECTION_IMG_W: usize = 800;
pub const SECTION_IMG_H: usize = 201;

#[derive(Debug, Clone)]
pub struct Settings {
    /// Средняя скорость, м/с (Edit2 = 1500).
    pub velocity: f64,
    /// Качество изображения (Edit1 = 3), шаг сетки карты в 0x46641C.
    /// Helper `0x467540` ограничивает значение диапазоном 0.1..10.
    pub quality: f64,
    /// Угол структурной карты в градусах (Edit4 = 0).
    pub angle: f64,
    /// Шаг выборки вдоль профиля, м (0x46742C = 200).
    pub step: f64,
    /// Число узлов карты по X/Y. По умолчанию шаг 200 м на 8000x4000 м.
    pub width: usize,
    pub height: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            velocity: 1500.0,
            quality: 3.0,
            angle: 0.0,
            step: 200.0,
            width: (MAP_WIDTH_M / 200.0) as usize + 1, // 41
            height: (MAP_HEIGHT_M / 200.0) as usize + 1, // 21
        }
    }
}

impl Settings {
    /// Число узлов карты по X из `0x46641C`: `N = trunc(quality*100)+1`.
    pub fn map_nodes_x(&self) -> usize {
        let q = self.quality.clamp(0.1, 10.0);
        ((q * 100.0).trunc() as usize + 1).clamp(2, 1001)
    }

    /// Число узлов карты по Y из `0x46641C`:
    /// `ny = N * Image2.Height / Image2.Width`.
    pub fn map_nodes_y(&self) -> usize {
        (self.map_nodes_x() * SECTION_IMG_H / SECTION_IMG_W).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_map_nodes_follow_dfm_quality() {
        let s = Settings::default();
        assert_eq!(s.quality, 3.0);
        assert_eq!(s.map_nodes_x(), 301);
        assert_eq!(s.map_nodes_y(), 75); // 301 * 201 / 800

        let coarse = Settings {
            quality: 0.5,
            ..Default::default()
        };
        assert_eq!(coarse.map_nodes_x(), 51);
        assert_eq!(coarse.map_nodes_y(), 12);
    }
}
