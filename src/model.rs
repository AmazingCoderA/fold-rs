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

#[derive(Debug, Clone)]
pub struct Settings {
    /// Средняя скорость, м/с (Edit2 = 1500).
    pub velocity: f64,
    /// Качество изображения, % (Edit3 = 95).
    pub quality: f64,
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
            quality: 95.0,
            step: 200.0,
            width: (MAP_WIDTH_M / 200.0) as usize + 1,  // 41
            height: (MAP_HEIGHT_M / 200.0) as usize + 1, // 21
        }
    }
}
