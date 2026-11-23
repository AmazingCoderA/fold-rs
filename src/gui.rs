//! GUI-клон оригинального окна Fold.
//!
//! Раскладка НЕ выдумана и не снята «на глаз»: все координаты, размеры,
//! цвета и размеры шрифтов взяты из бинарного DFM-ресурса `TFORM1`
//! оригинального `03_Fold.exe` (cm. `analysis/dfm_values.txt`).
//!
//! Ключевое открытие: скриншот 894x656 — это окно оригинала,
//! масштабированное на 7/8. Настоящий клиент формы: 1016x694.
//! GUI рисуется в координатах DFM, а при изменении размера окна применяется
//! ОДИН масштаб на обе оси — пропорции и линии не «кривятся».
//!
//! Добавлен только 20-пиксельный слой меню сверху (в оригинале меню —
//! системное, вне клиентской области), чтобы все кнопки были доступны.

use crate::grid::{
    build_map_native, build_section, profile_time_bounds, section_time_from_y, MapGrid, SectionData,
};
use crate::io::{fmt_num, parse, serialize};
use crate::model::{
    Pick, Profile, Settings, MAP_HEIGHT_M, MAP_IMG_H, MAP_IMG_W, MAP_WIDTH_M, SECTION_IMG_H,
    SECTION_IMG_W,
};
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, TextureHandle};
use std::sync::{Arc, OnceLock};

/// Настоящий клиент формы из DFM: ClientWidth=1016, ClientHeight=694.
const CW: f32 = 1016.0;
/// Слой меню поверх клиентской области (в DFM меню системное).
const MENU_H: f32 = 20.0;

/// Контролы из DFM (координаты в клиентской области формы).
const PANEL: (f32, f32, f32, f32) = (4.0, 40.0, 173.0, 629.0); // Panel1
const MAP: (f32, f32, f32, f32) = (200.0, 40.0, 800.0, 400.0); // Image1
const SEC: (f32, f32, f32, f32) = (200.0, 468.0, 800.0, 201.0); // Image2
const LEGEND: (f32, f32, f32, f32) = (496.0, 444.0, 245.0, 21.0); // Image3
const GRID: (f32, f32, f32, f32) = (32.0, 180.0, 123.0, 101.0); // StringGrid1
const MEMO: (f32, f32, f32, f32) = (32.0, 320.0, 125.0, 305.0); // Memo1

const BANDS: usize = 21;
/// Четыре редактируемые ячейки `StringGrid1` (DFM: ColCount=2, RowCount=4).
const GRID_IDS: [&str; 4] = ["grid_0", "grid_1", "grid_2", "grid_3"];
/// TabOrder из DFM: Edit2(0) → StringGrid(1) → Edit1(2) → Edit3(3) →
/// Memo1(4) → Edit4(1) → Edit5(2). Ячейки таблицы развёрнуты в четыре
/// остановки, а `memo_field` — невидимая цель фокуса для Memo1.
const FIELDS: [&str; 10] = [
    "v_field",
    "grid_0",
    "grid_1",
    "grid_2",
    "grid_3",
    "qual_field",
    "f_field",
    "memo_field",
    "x_field",
    "y_field",
];

/// Единое преобразование «DFM → экран». Один масштаб на обе оси: окно можно
/// растягивать, но пропорции и линии при этом не «кривятся».
#[derive(Clone, Copy)]
struct View {
    scale: f32,
    origin: Pos2,
}

impl View {
    fn p(self, x: f32, y: f32) -> Pos2 {
        Pos2::new(
            self.origin.x + x * self.scale,
            self.origin.y + (y + MENU_H) * self.scale,
        )
    }

    fn r(self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_max(self.p(x, y), self.p(x + w, y + h))
    }

    fn fs(self, size: f32) -> f32 {
        (size * self.scale).max(4.0)
    }
}

/// 0x466198 / 0x4661F0: карта (metres) → пиксели Image1 (800×400).
/// Ось Y в original направлена вниз, а на карте — вверх, поэтому здесь есть
/// вертикальное отражение. Текстура карты использует тот же flip в `recalc`.
fn map_to_image(x: f64, y: f64) -> (f32, f32) {
    let sx = (x / MAP_WIDTH_M * (MAP_IMG_W - 1) as f64).trunc() as f32;
    let sy = (MAP_IMG_H - 1) as f32 - (y / MAP_HEIGHT_M * (MAP_IMG_H - 1) as f64).trunc() as f32;
    (sx, sy)
}

/// Обратное преобразование пикселей Image1 → метры карты.
fn image_to_map(sx: f32, sy: f32) -> (f64, f64) {
    let w = (MAP_IMG_W - 1) as f64;
    let h = (MAP_IMG_H - 1) as f64;
    (
        (f64::from(sx) / w * MAP_WIDTH_M).clamp(0.0, MAP_WIDTH_M),
        ((h - f64::from(sy)) / h * MAP_HEIGHT_M).clamp(0.0, MAP_HEIGHT_M),
    )
}

/// Цвета Delphi из DFM (clXxx → RGB).
const CL_OLIVE: Color32 = Color32::from_rgb(128, 128, 0);
const CL_TEAL: Color32 = Color32::from_rgb(0, 128, 128);
const CL_GREEN: Color32 = Color32::from_rgb(0, 128, 0);
const CL_NAVY: Color32 = Color32::from_rgb(0, 0, 128);
const CL_PURPLE: Color32 = Color32::from_rgb(128, 0, 128);
const CL_MAROON: Color32 = Color32::from_rgb(128, 0, 0);
/// `clMoneyGreen` из DFM = `$00C0DCC0`.
const CL_MONEY_GREEN: Color32 = Color32::from_rgb(192, 220, 192);

#[derive(Clone, Copy)]
enum Face {
    Reg,
    Bold,
    BoldItalic,
    Symbol,
    BoldUnderline,
}

struct Theme {
    name: &'static str,
    stops: [(f32, [u8; 3]); 5],
}

/// Первая палитра — точная из original; остальные — «разнообразнее по цветам».
const THEMES: [Theme; 5] = [
    Theme {
        name: "Классика",
        // Точная формула 0x46641C: (65,65+560t,205) → (65,205,205) →
        // (65+560(t−.5),205,65) → (205,205−560(t−.75),65).
        stops: [
            (0.00, [65, 65, 205]),
            (0.25, [65, 205, 205]),
            (0.50, [65, 205, 65]),
            (0.75, [205, 205, 65]),
            (1.00, [205, 65, 65]),
        ],
    },
    Theme {
        name: "Океан",
        stops: [
            (0.00, [12, 32, 78]),
            (0.25, [16, 88, 140]),
            (0.50, [32, 160, 190]),
            (0.75, [120, 215, 225]),
            (1.00, [230, 248, 252]),
        ],
    },
    Theme {
        name: "Пламя",
        stops: [
            (0.00, [30, 8, 10]),
            (0.25, [132, 20, 22]),
            (0.50, [222, 72, 20]),
            (0.75, [250, 172, 42]),
            (1.00, [255, 246, 205]),
        ],
    },
    Theme {
        name: "Лайм",
        stops: [
            (0.00, [18, 50, 22]),
            (0.25, [40, 122, 52]),
            (0.50, [112, 192, 62]),
            (0.75, [204, 228, 84]),
            (1.00, [250, 250, 212]),
        ],
    },
    Theme {
        name: "Графит",
        stops: [
            (0.00, [28, 32, 38]),
            (0.25, [84, 92, 102]),
            (0.50, [146, 154, 162]),
            (0.75, [206, 212, 218]),
            (1.00, [252, 252, 252]),
        ],
    },
];

pub struct FoldApp {
    profiles: Vec<Profile>,
    settings: Settings,
    selected: usize,
    texture: Option<TextureHandle>,
    last_ms: f64,
    dirty: bool,

    // Поля ввода. Значения по умолчанию — из DFM Edit2/Edit3/Edit1/Edit4/Edit5.
    v: String,
    fval: String,
    quality: String,
    xang: String,
    yang: String,
    xg: String,
    yg: String,
    /// Редактируемые значения StringGrid1: x0, y0, x1, y1.
    grid_text: [String; 4],
    status: String,

    drag_start: Option<Pos2>,
    drag_cur: Option<Pos2>,
    show_about: bool,
    show_help: bool,
    show_io: bool,
    show_profiles: bool,
    io_text: String,
    io_error: String,
    theme: usize,
    /// Arial загружен из C:\Windows\Fonts; иначе fallback на встроенный шрифт.
    fonts_ok: bool,
    /// Ключ пересчёта текстуры карты: (угол, качество, палитра).
    map_key: Option<(f64, f64, usize)>,
    /// Текстура Image2: полный буфер 300 отсчётов на трассу из `0x465BD4`.
    section_tex: Option<TextureHandle>,
    /// Вертикальный масштаб временного разреза из `0x466980`: (t min, t max).
    time_bounds: (f64, f64),
    /// Ключ пересчёта `time_bounds`: (профиль, x0, y0, x1, y1, V, угол).
    /// Пики не влияют на масштаб, поэтому перемещение мышью не вызывает
    /// повторного прохода по 200-метровым отсчётам профиля.
    time_key: Option<(usize, f64, f64, f64, f64, f64, f64)>,
    /// Текущее преобразование координат (пересчитывается при ресайзе окна).
    view: View,
    /// Фокус на предыдущем кадре: нужен, чтобы при уходе из поля показать
    /// фактически применённое (и ограниченное диапазоном) значение, а не «1» вместо 1000.
    last_focused: Option<egui::Id>,
    /// DFM `Image2.OnMouseDown`: пик ставится в момент нажатия, а не на отпускании.
    /// Флаг защищает от повторного добавления в следующих кадрах.
    sec_pointer_down: bool,
}

impl FoldApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let fonts_ok = install_fonts(cc);
        // Значения полей — строго из DFM: Edit2='1500', Edit1='3', Edit3='95',
        // Edit4='0', Edit5='0'.
        let settings = Settings::default();
        let base = Profile {
            id: 0,
            x0: 100.0,
            y0: 100.0,
            x1: 3100.0,
            y1: 100.0,
            picks: Vec::new(),
        };
        let time_bounds = profile_time_bounds(&base, &settings);
        let time_key = Some((
            0,
            base.x0,
            base.y0,
            base.x1,
            base.y1,
            settings.velocity,
            settings.angle,
        ));
        let span = time_bounds.1 - time_bounds.0;
        let demos = [
            (1.0_f64, 0.20_f64),
            (3.0, 0.35),
            (5.0, 0.50),
            (7.0, 0.65),
            (10.0, 0.80),
        ];
        let profiles = vec![Profile {
            picks: demos
                .iter()
                .map(|&(trace, k)| Pick {
                    trace,
                    time: time_bounds.0 + k * span,
                })
                .collect(),
            ..base
        }];
        Self {
            profiles,
            settings,
            selected: 0,
            texture: None,
            last_ms: 0.0,
            dirty: true,
            v: "1500".into(),
            fval: "95".into(),
            quality: "3".into(),
            xang: "0".into(),
            yang: "0".into(),
            xg: "3100".into(),
            yg: "100".into(),
            grid_text: ["100".into(), "100".into(), "3100".into(), "100".into()],
            status: String::new(),
            drag_start: None,
            drag_cur: None,
            show_about: false,
            show_help: false,
            show_io: false,
            show_profiles: false,
            io_text: String::new(),
            io_error: String::new(),
            theme: 0,
            fonts_ok,
            map_key: None,
            section_tex: None,
            time_bounds,
            time_key,
            view: View {
                scale: 1.0,
                origin: Pos2::ZERO,
            },
            last_focused: None,
            sec_pointer_down: false,
        }
    }

    /// Пересчитывает Range и текстуру Image2 только при изменении профиля, V
    /// или угла. Текстура содержит все 300 отсчётов на трассу, как original.
    fn refresh_time_bounds(&mut self, ctx: &egui::Context) {
        let key = self.profiles.get(self.selected).map(|p| {
            (
                self.selected,
                p.x0,
                p.y0,
                p.x1,
                p.y1,
                self.settings.velocity,
                self.settings.angle,
            )
        });
        if self.time_key == key && self.section_tex.is_some() {
            return;
        }

        let image = match self.profiles.get(self.selected) {
            Some(profile) => {
                let data = build_section(profile, &self.settings);
                self.time_bounds = (f64::from(data.t_min), f64::from(data.t_max));
                section_image(&data)
            }
            None => {
                self.time_bounds = (0.0, 1.0);
                egui::ColorImage::new([SECTION_IMG_W, SECTION_IMG_H], Color32::WHITE)
            }
        };
        match self.section_tex.as_mut() {
            Some(tex) => tex.set(image, egui::TextureOptions::NEAREST),
            None => {
                self.section_tex =
                    Some(ctx.load_texture("time_section", image, egui::TextureOptions::NEAREST));
            }
        }
        self.time_key = key;
    }

    fn recalc(&mut self, ctx: &egui::Context) {
        let key = (self.settings.angle, self.settings.quality, self.theme);
        self.dirty = false;
        if self.map_key == Some(key) && self.texture.is_some() {
            return;
        }

        let t = std::time::Instant::now();
        let g = build_map_native(&self.profiles, &self.settings);
        let (min, max) = bounds(&g);
        let span = (max - min).max(1e-6);
        let theme = self.theme;

        // Дословная растеризация 0x46641C: карта рисуется на Image1 (800x400)
        // прямоугольниками 10/step пикселей с жёсткими границами. Порядок
        // обхода i (X) -> j (Y) важен: поздние прямоугольники перекрывают
        // ранние, как в Delphi Canvas.FillRect.
        let mut img = egui::ColorImage::new([MAP_IMG_W, MAP_IMG_H], scale_color(0.0, theme));
        let half = 10.0_f64 / self.settings.quality.clamp(0.1, 9.99);
        let back = half.trunc() as i32;
        let front = (2.0_f64 * half).trunc() as i32;
        for gx in 0..g.width {
            let sx = (gx * MAP_IMG_W / g.width) as i32;
            for gy in 0..g.height {
                let d = g.data[gy * g.width + gx];
                if !d.is_finite() {
                    continue;
                }
                let sy = MAP_IMG_H as i32 - (gy * MAP_IMG_H / g.height) as i32;
                let color = palette(((d - min) / span).clamp(0.0, 1.0), theme);
                // Delphi FillRect: right/bottom не включаются.
                let x0 = (sx - back).max(0) as usize;
                let x1 = (sx + front).min(MAP_IMG_W as i32) as usize;
                let y0 = (sy - back).max(0) as usize;
                let y1 = (sy + front).min(MAP_IMG_H as i32) as usize;
                for y in y0..y1 {
                    let row = y * MAP_IMG_W;
                    img.pixels[row + x0..row + x1].fill(color);
                }
            }
        }
        self.last_ms = t.elapsed().as_secs_f64() * 1000.0;
        // Текстура уже имеет пиксельное разрешение Image1, INTERP не нужен.
        match &mut self.texture {
            Some(tex) => tex.set(img, egui::TextureOptions::NEAREST),
            None => {
                self.texture =
                    Some(ctx.load_texture("struct_map", img, egui::TextureOptions::NEAREST));
            }
        }
        self.map_key = Some(key);
        // TextureHandle::set не вызывает репейнт сама; все текущие изменения
        // input-driven, но страховка не мешает и убирает риск «старой» карты.
        ctx.request_repaint();
    }

    fn add_profile(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        let id = self.profiles.len();
        self.profiles.push(Profile {
            id,
            x0: x0 as f64,
            y0: y0 as f64,
            x1: x1 as f64,
            y1: y1 as f64,
            picks: vec![
                Pick {
                    trace: 1.0,
                    time: 3.465,
                },
                Pick {
                    trace: 5.0,
                    time: 3.520,
                },
                Pick {
                    trace: 10.0,
                    time: 3.500,
                },
            ],
        });
        self.selected = id;
        self.xg = format!("{:.0}", x1);
        self.yg = format!("{:.0}", (y0 + y1) * 0.5);
        self.sync_grid_text();
        self.status = format!("Профиль {} добавлен", id + 1);
        self.dirty = true;
    }

    fn clear_table(&mut self) {
        if let Some(p) = self.profiles.get_mut(self.selected) {
            p.picks.clear();
            self.dirty = true;
            self.status = "Таблица очищена".into();
        }
    }

    fn select_profile(&mut self, idx: usize) {
        if idx >= self.profiles.len() {
            return;
        }
        self.selected = idx;
        let p = &self.profiles[idx];
        self.xg = format!("{:.0}", p.x1);
        self.yg = format!("{:.0}", (p.y0 + p.y1) * 0.5);
        self.sync_grid_text();
        self.status = format!("Профиль {}", idx + 1);
    }

    /// Перезагрузить четыре ячейки из активного профиля (смена профиля/импорт).
    fn sync_grid_text(&mut self) {
        let Some(p) = self.profiles.get(self.selected) else {
            self.grid_text = Default::default();
            return;
        };
        self.grid_text = [
            format!("{:.0}", p.x0),
            format!("{:.0}", p.y0),
            format!("{:.0}", p.x1),
            format!("{:.0}", p.y1),
        ];
    }

    /// Применить одну ячейку к профилю. При `restore` неверный текст вернуть
    /// к числовому значению профиля (используется при потере фокуса/Enter).
    fn commit_grid_cell(&mut self, idx: usize, restore: bool) {
        let Ok(value) = self.grid_text[idx].trim().parse::<f64>() else {
            if restore {
                self.grid_text[idx] = self
                    .profiles
                    .get(self.selected)
                    .map(|p| match idx {
                        0 => format!("{:.0}", p.x0),
                        1 => format!("{:.0}", p.y0),
                        2 => format!("{:.0}", p.x1),
                        _ => format!("{:.0}", p.y1),
                    })
                    .unwrap_or_default();
            }
            return;
        };
        if let Some(p) = self.profiles.get_mut(self.selected) {
            match idx {
                0 => p.x0 = value,
                1 => p.y0 = value,
                2 => p.x1 = value,
                _ => p.y1 = value,
            }
            self.dirty = true;
        }
    }

    /// Восстанавливает текст поля после потери фокуса. V/угол/качество
    /// показывают значение из настроек: пользователь мог ввести «1», а
    /// original-кламп превратил его в 1000 — в поле должно быть 1000.
    fn normalize_lost_field(&mut self, id: egui::Id) {
        if id == egui::Id::new("v_field") {
            self.v = fmt_num(self.settings.velocity);
        } else if id == egui::Id::new("qual_field") {
            self.quality = fmt_num(self.settings.quality);
        } else if id == egui::Id::new("x_field") {
            self.xang = fmt_num(self.settings.angle);
        } else if id == egui::Id::new("f_field") && self.fval.parse::<f64>().is_err() {
            self.fval = "95".into();
        } else if id == egui::Id::new("y_field") && self.yang.parse::<f64>().is_err() {
            self.yang = "0".into();
        }
    }
}

/// Загружает Arial (и его начертания) + Symbol из системных шрифтов Windows.
/// DFM явно задаёт Font.Name='Arial', Symbol и Font.Height -11/-13/-15/-16.
fn install_fonts(cc: &eframe::CreationContext<'_>) -> bool {
    let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let files = [
        ("Arial", "arial.ttf"),
        ("ArialB", "arialbd.ttf"),
        ("ArialI", "ariali.ttf"),
        ("ArialBI", "arialbi.ttf"),
        ("Symbol", "symbol.ttf"),
    ];
    let mut fonts = egui::FontDefinitions::default();
    for (name, file) in files {
        let path = std::path::Path::new(&dir).join("Fonts").join(file);
        let Ok(bytes) = std::fs::read(&path) else {
            return false;
        };
        fonts
            .font_data
            .insert(name.into(), egui::FontData::from_owned(bytes));
        fonts
            .families
            .insert(FontFamily::Name(name.into()), vec![name.into()]);
    }
    cc.egui_ctx.set_fonts(fonts);
    true
}

fn bounds(g: &MapGrid) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &v in &g.data {
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

/// 21 цвет каждой палитры считается один раз: раньше `scale_color` с циклом
/// по stops и `[u8; 3]` вызывался для каждого из 500 000 пикселей карты.
fn palette_lut() -> &'static [[Color32; BANDS]; THEMES.len()] {
    static LUT: OnceLock<[[Color32; BANDS]; THEMES.len()]> = OnceLock::new();
    LUT.get_or_init(|| {
        std::array::from_fn(|theme| {
            std::array::from_fn(|band| scale_color(band as f32 / BANDS as f32, theme))
        })
    })
}

#[inline]
fn band_color(band: usize, theme: usize) -> Color32 {
    palette_lut()[theme][band.min(BANDS - 1)]
}

#[inline]
fn palette(u: f32, theme: usize) -> Color32 {
    // 0x46641C сравнивает |H−(min+k·span/21)| <= span/42, то есть выбирает
    // ближайшую полосу — это `round(u·21)`, а не `floor`.
    let band = (u * BANDS as f32 + 0.5) as usize;
    band_color(band, theme)
}

fn scale_color(t: f32, theme: usize) -> Color32 {
    let stops = THEMES[theme].stops;
    for w in stops.windows(2) {
        if t <= w[1].0 {
            let k = (t - w[0].0) / (w[1].0 - w[0].0);
            let c = [0, 1, 2].map(|i| {
                (w[0].1[i] as f32 + (w[1].1[i] as f32 - w[0].1[i] as f32) * k).round() as u8
            });
            return Color32::from_rgb(c[0], c[1], c[2]);
        }
    }
    let c = stops[stops.len() - 1].1;
    Color32::from_rgb(c[0], c[1], c[2])
}

#[inline]
fn put_pixel(img: &mut egui::ColorImage, x: i32, y: i32, color: Color32) {
    let width = img.width();
    if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < img.height() {
        img.pixels[y as usize * width + x as usize] = color;
    }
}

#[inline]
fn hline(img: &mut egui::ColorImage, x0: i32, x1: i32, y: i32, color: Color32) {
    for x in x0.min(x1)..=x0.max(x1) {
        put_pixel(img, x, y, color);
    }
}

#[inline]
fn vline(img: &mut egui::ColorImage, x: i32, y0: i32, y1: i32, color: Color32) {
    for y in y0.min(y1)..=y0.max(y1) {
        put_pixel(img, x, y, color);
    }
}

/// Отрисовка одного отрезка `TCanvas.LineTo` (отрицательные амплитуды в
/// `0x46608B` соединяют предыдущую точку с новой, отсюда диагонали).
fn line(img: &mut egui::ColorImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Color32) {
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

/// Дословная растеризация `0x465D68` в 800×201 буфер Image2.
fn section_image(data: &SectionData) -> egui::ColorImage {
    let mut img = egui::ColorImage::new([SECTION_IMG_W, SECTION_IMG_H], Color32::WHITE);
    let w = SECTION_IMG_W as f32;
    let h = SECTION_IMG_H as f32;
    let span = data.t_max - data.t_min;
    let scale_t = h / span;
    let grid_y = h / 10.0; // 0x466150 = 10
    let x_step = w / (data.n as f32 + 1.0);
    let grid = Color32::from_gray(210); // 0x467739..0x467743: $D2D2D2

    // 0x465E4C: вертикальные линии, i = 1..n, y = 1..H−1.
    for i in 1..=data.n {
        let x = (i as f32 * x_step).trunc() as i32;
        vline(&mut img, x, 1, SECTION_IMG_H as i32 - 1, grid);
    }
    // 0x465F05: горизонтальные линии, i = 1..11 (11-я за краем контрола).
    for i in 1..=11 {
        let y = (i as f32 * grid_y).trunc() as i32;
        hline(&mut img, 0, SECTION_IMG_W as i32 - 1, y, grid);
    }

    // 0x465FB0: RMS считается ТОЛЬКО по первым 300 отсчётам первой трассы.
    let mut sum = 1e-8f32; // 0x322BCC77
    for &s in data.samples.iter().take(crate::grid::SECTION_SAMPLES) {
        sum += s.wrapping_mul(s) as f32;
    }
    let rms = f64::from((sum / crate::grid::SECTION_SAMPLES as f32).sqrt());

    // 0x466019: x = trunc(i*x_step), amp = trunc(0.12/rms*sample*x_step).
    for trace in 0..data.n {
        let x0 = ((trace + 1) as f32 * x_step).trunc() as i32;
        let mut cur = (x0, 0i32); // 0x465FE1: MoveTo(x0, 0)
        for j in 0..crate::grid::SECTION_SAMPLES {
            let s = data.samples[trace * crate::grid::SECTION_SAMPLES + j];
            let tj = data.t_min + j as f32 / crate::grid::SECTION_SAMPLES as f32 * span;
            let y = ((tj - data.t_min) * scale_t).trunc() as i32;
            let amp = (0.12f64 / rms * f64::from(s) * f64::from(x_step)).trunc() as i32;
            let x1 = x0 + amp;
            if amp < 0 {
                // 0x46608B: LineTo без MoveTo — connects previous endpoint.
                line(&mut img, cur.0, cur.1, x1, y, Color32::BLACK);
            } else {
                // 0x4676CC: MoveTo(x0,y) + LineTo(x1,y).
                hline(&mut img, x0, x1, y, Color32::BLACK);
            }
            cur = (x1, y);
        }
    }
    img
}

// ── примитивы рисования в координатах DFM ────────────────────────────────────

fn font_family(face: Face) -> &'static FontFamily {
    static FAMILIES: OnceLock<[FontFamily; 5]> = OnceLock::new();
    let families = FAMILIES.get_or_init(|| {
        [
            FontFamily::Name(Arc::from("Arial")),
            FontFamily::Name(Arc::from("ArialB")),
            FontFamily::Name(Arc::from("ArialBI")),
            FontFamily::Name(Arc::from("Symbol")),
            FontFamily::Name(Arc::from("ArialB")),
        ]
    });
    match face {
        Face::Reg => &families[0],
        Face::Bold => &families[1],
        Face::BoldItalic => &families[2],
        Face::Symbol => &families[3],
        Face::BoldUnderline => &families[4],
    }
}

fn font_of(fonts_ok: bool, face: Face, size: f32) -> FontId {
    if !fonts_ok {
        return FontId::proportional(size);
    }
    FontId::new(size, font_family(face).clone())
}

#[allow(clippy::too_many_arguments)]
fn label(
    ui: &egui::Ui,
    view: View,
    fonts_ok: bool,
    x: f32,
    y: f32,
    s: &str,
    face: Face,
    size: f32,
    color: Color32,
) {
    ui.painter().text(
        view.p(x, y),
        Align2::LEFT_TOP,
        s,
        font_of(fonts_ok, face, view.fs(size)),
        color,
    );
}

#[allow(clippy::too_many_arguments)]
fn centered(
    ui: &egui::Ui,
    view: View,
    fonts_ok: bool,
    x: f32,
    y: f32,
    s: &str,
    face: Face,
    size: f32,
    color: Color32,
) {
    ui.painter().text(
        view.p(x, y),
        Align2::CENTER_TOP,
        s,
        font_of(fonts_ok, face, view.fs(size)),
        color,
    );
}

/// Подпись Label9 из DFM: Bold + Underline, прижата к правому краю.
#[allow(clippy::too_many_arguments)]
fn underlined_right_label(
    ui: &egui::Ui,
    view: View,
    fonts_ok: bool,
    x: f32,
    y: f32,
    s: &str,
    size: f32,
    color: Color32,
) {
    let font = font_of(fonts_ok, Face::BoldUnderline, view.fs(size));
    let galley = ui
        .painter()
        .layout(s.to_owned(), font.clone(), color, f32::INFINITY);
    let p = view.p(x, y);
    ui.painter()
        .text(p, Align2::RIGHT_TOP, s.to_owned(), font, color);
    let uy = p.y + view.fs(size) + view.scale;
    ui.painter().line_segment(
        [Pos2::new(p.x - galley.rect.width(), uy), Pos2::new(p.x, uy)],
        Stroke::new(view.scale.max(0.7), color),
    );
}

/// Поле ввода VCL: белая подложка, утопленная обводка, синий фокус.
fn edit(
    ui: &mut egui::Ui,
    fonts_ok: bool,
    view: View,
    rect: Rect,
    val: &mut String,
    id: &'static str,
) -> bool {
    let id = egui::Id::new(id);
    let focused = ui.memory(|m| m.has_focus(id));
    let stroke_w = view.scale.max(0.6);
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    let dark = if focused {
        Color32::from_rgb(40, 110, 200)
    } else {
        Color32::from_gray(120)
    };
    let light = Color32::from_gray(250);
    ui.painter().line_segment(
        [rect.min, Pos2::new(rect.max.x, rect.min.y)],
        Stroke::new(stroke_w, dark),
    );
    ui.painter().line_segment(
        [rect.min, Pos2::new(rect.min.x, rect.max.y)],
        Stroke::new(stroke_w, dark),
    );
    ui.painter().line_segment(
        [Pos2::new(rect.min.x, rect.max.y), rect.max],
        Stroke::new(stroke_w, light),
    );
    ui.painter().line_segment(
        [Pos2::new(rect.max.x, rect.min.y), rect.max],
        Stroke::new(stroke_w, light),
    );
    let pad = 2.0 * view.scale;
    let inner = Rect::from_min_max(
        Pos2::new(rect.min.x + pad, rect.min.y + pad),
        Pos2::new(rect.max.x - pad, rect.max.y - pad),
    );
    ui.put(
        inner,
        egui::TextEdit::singleline(val)
            .id(id)
            .frame(false)
            .font(font_of(fonts_ok, Face::Reg, view.fs(11.0))),
    )
    .changed()
}

/// Ячейка `StringGrid1`: фон из DFM, редактируемый текст Arial Bold.
fn grid_cell(
    ui: &mut egui::Ui,
    fonts_ok: bool,
    view: View,
    rect: Rect,
    bg: Color32,
    val: &mut String,
    id: &'static str,
) -> bool {
    let id = egui::Id::new(id);
    ui.painter().rect_filled(rect, 0.0, bg);
    if ui.memory(|m| m.has_focus(id)) {
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(view.scale.max(0.8), Color32::from_rgb(40, 110, 200)),
        );
    }
    let pad = 2.0 * view.scale;
    let inner = Rect::from_min_max(
        Pos2::new(rect.min.x + pad, rect.min.y + pad),
        Pos2::new(rect.max.x - pad, rect.max.y - pad),
    );
    ui.put(
        inner,
        egui::TextEdit::singleline(val)
            .id(id)
            .frame(false)
            .font(font_of(fonts_ok, Face::Bold, view.fs(13.0))),
    )
    .changed()
}

fn button(ui: &mut egui::Ui, view: View, rect: Rect, id: &'static str, text: &str) -> bool {
    let mut clicked = false;
    ui.push_id(id, |ui| {
        let b = egui::Button::new(
            egui::RichText::new(text)
                .size(view.fs(11.0))
                .color(Color32::from_gray(25)),
        )
        .fill(Color32::from_gray(238))
        .stroke(Stroke::new(view.scale.max(0.6), Color32::from_gray(125)));
        clicked = ui.put(rect, b).clicked();
    });
    clicked
}

impl eframe::App for FoldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dirty {
            self.recalc(ctx);
        }
        // Вертикальный масштаб разреза обновляется непосредственно перед
        // отрисовкой Image2 — там уже известны актуальные профиль/координаты.

        // Поле, которое только что потеряло фокус, показывает фактически
        // применённое значение (с учётом клампов original).
        let focused_now = ctx.memory(|m| m.focused());
        if let Some(prev) = self.last_focused {
            if focused_now != Some(prev) {
                self.normalize_lost_field(prev);
            }
        }

        // Явная навигация: Tab/Shift+Tab и Enter переходят между полями DFM.
        // Событие удаляется из ввода, чтобы egui не двигал фокус второй раз
        // (раньше из-за этого фокус «прыгал» на соседнее поле).
        let nav = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key: egui::Key::Tab,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((true, modifiers.shift)),
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    ..
                } => Some((false, false)),
                _ => None,
            })
        });
        if let Some((is_tab, shift)) = nav {
            let cur = ctx.memory(|m| m.focused());
            let pos = cur.and_then(|id| FIELDS.iter().position(|f| id == egui::Id::new(*f)));
            if is_tab || pos.is_some() {
                if !is_tab {
                    if let Some(p) = pos.filter(|p| (1..=4).contains(p)) {
                        self.commit_grid_cell(p - 1, true);
                    }
                }
                ctx.input_mut(|i| {
                    i.events.retain(|e| {
                        !matches!(
                            e,
                            egui::Event::Key {
                                key: egui::Key::Tab | egui::Key::Enter,
                                ..
                            }
                        )
                    })
                });
                let next = match pos {
                    Some(i) => (i + if shift { FIELDS.len() - 1 } else { 1 }) % FIELDS.len(),
                    None => {
                        if shift {
                            FIELDS.len() - 1
                        } else {
                            0
                        }
                    }
                };
                ctx.memory_mut(|m| m.request_focus(egui::Id::new(FIELDS[next])));
            }
        }

        // Горячие клавиши меню из DFM: F2/F3/F4/Del/Alt+Del.
        let shortcuts = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some((*key, modifiers.alt)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        // Delete в textual field должен удалять символ, а не строку пика.
        let in_text_field = focused_now
            .map(|id| FIELDS.iter().any(|f| id == egui::Id::new(*f)))
            .unwrap_or(false);
        for (key, alt) in shortcuts {
            match key {
                egui::Key::F2 => self.show_about = !self.show_about,
                egui::Key::F3 => self.show_profiles = !self.show_profiles,
                egui::Key::F4 => self.clear_table(),
                egui::Key::Delete if !in_text_field => {
                    if alt {
                        self.clear_table();
                    } else if let Some(pr) = self.profiles.get_mut(self.selected) {
                        pr.picks.pop();
                        self.dirty = true;
                    }
                }
                _ => {}
            }
        }

        // Ячейка, которую покинули, должна показать число из профиля, если
        // пользователь оставил незавершённый ввод («-», «1.», пустоту).
        for (i, id) in GRID_IDS.iter().enumerate() {
            let focused = ctx.memory(|m| m.focused()) == Some(egui::Id::new(*id));
            if !focused && self.grid_text[i].trim().parse::<f64>().is_err() {
                self.commit_grid_cell(i, true);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let area = ui.max_rect();
            ui.painter().rect_filled(area, 0.0, Color32::WHITE); // DFM Color=clWhite

            // Один масштаб на обе оси: при ресайзе сохраняются пропорции DFM.
            let total_h = 694.0 + MENU_H;
            let scale = (area.width() / CW).min(area.height() / total_h).max(0.25);
            let view = View {
                scale,
                origin: Pos2::new(
                    area.min.x + (area.width() - CW * scale) * 0.5,
                    area.min.y + (area.height() - total_h * scale) * 0.5,
                ),
            };
            self.view = view;

            // ── слой меню (20 px в координатах DFM) ──────────────────────────
            let menu = view.r(0.0, -MENU_H, CW, MENU_H);
            ui.painter().rect_filled(menu, 0.0, Color32::from_gray(236));
            ui.painter().line_segment(
                [
                    Pos2::new(menu.min.x, menu.max.y),
                    Pos2::new(menu.max.x, menu.max.y),
                ],
                Stroke::new(view.scale.max(0.6), Color32::from_gray(180)),
            );
            let mb = |x: f32, w: f32| view.r(x, -MENU_H + 1.0, w, MENU_H - 2.0);
            // Кэпшены строго из DFM: 'Структура ' с завершающим пробелом и
            // 'Cтереть таблицу' с латинской C (как в оригинале).
            if button(ui, view, mb(4.0, 78.0), "m_about", "Структура ") {
                self.show_about = !self.show_about;
            }
            if button(ui, view, mb(86.0, 84.0), "m_profiles", "Профили") {
                self.show_profiles = !self.show_profiles;
            }
            if button(ui, view, mb(174.0, 118.0), "m_clear", "Cтереть таблицу") {
                self.clear_table();
            }
            if button(ui, view, mb(296.0, 62.0), "m_io", "Строка") {
                self.io_text = serialize(&self.profiles, &self.settings);
                self.io_error.clear();
                self.show_io = true;
            }
            if button(ui, view, mb(860.0, 60.0), "m_theme", "Цвет") {
                self.theme = (self.theme + 1) % THEMES.len();
                self.dirty = true;
                self.status = format!("Палитра: {}", THEMES[self.theme].name);
            }
            if button(ui, view, mb(924.0, 88.0), "m_help", "Справка") {
                self.show_help = !self.show_help;
            }

            let theme = self.theme;

            // ── Label10: заголовок (Align=alTop, taCenter, BoldItalic 15) ─────
            centered(
                ui,
                view,
                self.fonts_ok,
                CW * 0.5,
                0.0,
                "Построение структурной карты отражающего горизонта",
                Face::BoldItalic,
                15.0,
                Color32::from_gray(0),
            );

            // ── верхние поля Edit4/Edit5 и подписи Х, м / Y, м ───────────────
            label(
                ui,
                view,
                self.fonts_ok,
                796.0,
                16.0,
                "Х, м",
                Face::Bold,
                13.0,
                Color32::BLACK,
            );
            if edit(
                ui,
                self.fonts_ok,
                view,
                view.r(828.0, 16.0, 65.0, 20.0),
                &mut self.xang,
                "x_field",
            ) {
                if let Ok(a) = self.xang.parse::<f64>() {
                    // 0x4674AC: integer из Edit4 с clamp 0..360, затем −90°.
                    self.settings.angle = a.clamp(0.0, 360.0);
                    self.dirty = true;
                }
            }
            label(
                ui,
                view,
                self.fonts_ok,
                900.0,
                16.0,
                "Y, м",
                Face::Bold,
                13.0,
                Color32::BLACK,
            );
            if edit(
                ui,
                self.fonts_ok,
                view,
                view.r(934.0, 16.0, 65.0, 20.0),
                &mut self.yang,
                "y_field",
            ) && self.yang.parse::<f64>().is_ok()
            {
                // вторая координата — запасная, карту не меняет
            }

            // ── Panel1 (левая панель, bsSingle, clWhite) ─────────────────────
            let panel = view.r(PANEL.0, PANEL.1, PANEL.2, PANEL.3);
            ui.painter().rect_filled(panel, 0.0, Color32::WHITE);
            ui.painter().rect_stroke(
                panel,
                0.0,
                Stroke::new(view.scale.max(0.6), Color32::from_gray(140)),
            );
            label(
                ui,
                view,
                self.fonts_ok,
                12.0,
                44.0,
                "Параметры структуры",
                Face::Bold,
                13.0,
                CL_GREEN,
            );
            label(
                ui,
                view,
                self.fonts_ok,
                12.0,
                68.0,
                "V ср",
                Face::Bold,
                13.0,
                Color32::BLACK,
            );
            if edit(
                ui,
                self.fonts_ok,
                view,
                view.r(64.0, 68.0, 77.0, 22.0),
                &mut self.v,
                "v_field",
            ) {
                if let Ok(x) = self.v.parse::<f64>() {
                    // 0x4674AC: V ср из Edit2 с clamp 1000..7000 м/с.
                    self.settings.velocity = x.clamp(1000.0, 7000.0);
                }
            }
            label(
                ui,
                view,
                self.fonts_ok,
                12.0,
                100.0,
                "f",
                Face::Symbol,
                16.0,
                Color32::BLACK,
            );
            if edit(
                ui,
                self.fonts_ok,
                view,
                view.r(64.0, 104.0, 77.0, 22.0),
                &mut self.fval,
                "f_field",
            ) && self.fval.parse::<f64>().is_ok()
            {
                // f из DFM не влияет на поверхность (см. analysis/map_reverse_report.md)
            }
            label(
                ui,
                view,
                self.fonts_ok,
                12.0,
                140.0,
                "Координаты профиля",
                Face::Bold,
                13.0,
                CL_NAVY,
            );
            label(
                ui,
                view,
                self.fonts_ok,
                32.0,
                162.0,
                "Профиль №",
                Face::Bold,
                11.0,
                CL_MAROON,
            );

            // StringGrid1 из DFM: ColCount=2, RowCount=4, DefaultColWidth=60,
            // RowHeights=24, FixedColor=clMoneyGreen, FixedRows=0, Options=goEditing.
            // FixedCols в DFM не переопределён ⇒ по умолчанию равен 1, поэтому
            // money-green заливается первый столбец, а не первая строка.
            let grid = view.r(GRID.0, GRID.1, GRID.2, GRID.3);
            let cell_w = 60.0 * view.scale;
            let row_h = 24.0 * view.scale;
            ui.painter().rect_filled(grid, 0.0, Color32::WHITE);
            for row in 0..4 {
                let y = grid.min.y + row as f32 * row_h;
                for col in 0..2 {
                    let idx = row * 2 + col;
                    if idx >= GRID_IDS.len() {
                        continue;
                    }
                    let bg = if col == 0 {
                        CL_MONEY_GREEN
                    } else {
                        Color32::WHITE
                    };
                    ui.painter().rect_filled(
                        Rect::from_min_max(
                            Pos2::new(grid.min.x + col as f32 * cell_w, y),
                            Pos2::new(
                                grid.min.x + (col + 1) as f32 * cell_w,
                                (y + row_h).min(grid.max.y),
                            ),
                        ),
                        0.0,
                        bg,
                    );
                    let cell = Rect::from_min_max(
                        Pos2::new(
                            grid.min.x + col as f32 * cell_w + view.scale,
                            y + view.scale,
                        ),
                        Pos2::new(
                            grid.min.x + (col + 1) as f32 * cell_w - view.scale,
                            y + row_h - view.scale,
                        ),
                    );
                    let changed = grid_cell(
                        ui,
                        self.fonts_ok,
                        view,
                        cell,
                        bg,
                        &mut self.grid_text[idx],
                        GRID_IDS[idx],
                    );
                    if changed {
                        self.commit_grid_cell(idx, false);
                    }
                }
            }
            let line_w = view.scale.max(0.6);
            ui.painter()
                .rect_stroke(grid, 0.0, Stroke::new(line_w, Color32::from_gray(140)));
            for col in 1..2 {
                let x = grid.min.x + col as f32 * cell_w;
                ui.painter().line_segment(
                    [Pos2::new(x, grid.min.y), Pos2::new(x, grid.max.y)],
                    Stroke::new(line_w, Color32::from_gray(160)),
                );
            }
            for row in 1..4 {
                let y = grid.min.y + row as f32 * row_h;
                ui.painter().line_segment(
                    [Pos2::new(grid.min.x, y), Pos2::new(grid.max.x, y)],
                    Stroke::new(line_w, Color32::from_gray(160)),
                );
            }

            // Label8 из DFM: 'Таблица корреляции' (clPurple, Bold 13).
            label(
                ui,
                view,
                self.fonts_ok,
                12.0,
                296.0,
                "Таблица корреляции",
                Face::Bold,
                13.0,
                CL_PURPLE,
            );

            // Memo1 (ссVertical) — пики t0, t1… с крестиком удаления строки.
            let memo = view.r(MEMO.0, MEMO.1, MEMO.2, MEMO.3);
            ui.painter().rect_filled(memo, 0.0, Color32::WHITE);
            ui.painter().rect_stroke(
                memo,
                0.0,
                Stroke::new(view.scale.max(0.6), Color32::from_gray(140)),
            );
            // DFM: Memo1.TabOrder=4. Регистрируем невидимую цель фокуса, чтобы
            // Tab проходил через таблицу пиков, а не пропускал её.
            let memo_focus = ui.interact(memo, egui::Id::new("memo_field"), Sense::click());
            if memo_focus.clicked() {
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("memo_field")));
            }
            // Без клона каждый кадр: заимствование только на время отрисовки
            // Memo1, затем оно отпускается перед удалением строки.
            let mut delete_row = None;
            {
                let picks: &[Pick] = self
                    .profiles
                    .get(self.selected)
                    .map(|p| p.picks.as_slice())
                    .unwrap_or(&[]);
                for (i, pk) in picks.iter().enumerate() {
                    // Текст и крестик используют ОДИН DFM-ряд: раньше крестик
                    // оказывался выше строки ввода из-за смешения систем и координат.
                    let dfm_y = MEMO.1 + 2.0 + i as f32 * 15.0;
                    let text_top = view.p(MEMO.0 + 3.0, dfm_y);
                    if text_top.y + 14.0 * view.scale > memo.max.y {
                        break;
                    }
                    label(
                        ui,
                        view,
                        self.fonts_ok,
                        MEMO.0 + 3.0,
                        dfm_y,
                        // 0x468830..0x4688AB: 't' + 1-based номер с нулевым
                        // паддингом до двух цифр + ' = ' + FloatToStrF(ffFixed, 3).
                        &format!("t{:02} = {:.3} с", i + 1, pk.time),
                        Face::Reg,
                        11.0,
                        Color32::BLACK,
                    );
                    let cross = Rect::from_min_max(
                        Pos2::new(memo.max.x - 13.0 * view.scale, text_top.y + view.scale),
                        Pos2::new(
                            memo.max.x - 2.0 * view.scale,
                            text_top.y + 13.0 * view.scale,
                        ),
                    );
                    let resp = ui.interact(cross, egui::Id::new(("del_pick", i)), Sense::click());
                    let cc = if resp.hovered() {
                        Color32::from_rgb(200, 40, 30)
                    } else {
                        Color32::from_gray(150)
                    };
                    let pad = 2.0 * view.scale;
                    ui.painter().line_segment(
                        [
                            Pos2::new(cross.min.x + pad, cross.min.y + pad),
                            Pos2::new(cross.max.x - pad, cross.max.y - pad),
                        ],
                        Stroke::new(view.scale.max(0.6), cc),
                    );
                    ui.painter().line_segment(
                        [
                            Pos2::new(cross.max.x - pad, cross.min.y + pad),
                            Pos2::new(cross.min.x + pad, cross.max.y - pad),
                        ],
                        Stroke::new(view.scale.max(0.6), cc),
                    );
                    if resp.clicked() {
                        delete_row = Some(i);
                    }
                }
            }
            if let Some(i) = delete_row {
                if let Some(prof) = self.profiles.get_mut(self.selected) {
                    if i < prof.picks.len() {
                        prof.picks.remove(i);
                        self.dirty = true;
                        self.status = format!("Строка {} удалена", i + 1);
                    }
                }
            }

            label(
                ui,
                view,
                self.fonts_ok,
                36.0,
                632.0,
                "Качество изображения",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            if edit(
                ui,
                self.fonts_ok,
                view,
                view.r(116.0, 636.0, 41.0, 22.0),
                &mut self.quality,
                "qual_field",
            ) {
                // 0x467540: float из Edit1, clamp 0.1..9.99; он задаёт N=trunc(q*100)+1.
                if let Ok(q) = self.quality.parse::<f64>() {
                    self.settings.quality = q.clamp(0.1, 9.99);
                    self.dirty = true;
                }
            }
            // ── карта Image1 (200,40,800,400) ─────────────────────────────────
            let map = view.r(MAP.0, MAP.1, MAP.2, MAP.3);
            ui.painter().rect_filled(map, 0.0, scale_color(0.0, theme));
            if let Some(tex) = &self.texture {
                ui.painter().image(
                    tex.id(),
                    map,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.painter().rect_stroke(
                map,
                0.0,
                Stroke::new(view.scale.max(0.6), Color32::from_gray(90)),
            );
            // 8 вертикальных (шаг 100 = 800/8) и 10 горизонтальных (шаг 40).
            for i in 0..=8 {
                let x = map.min.x + map.width() * i as f32 / 8.0;
                ui.painter().line_segment(
                    [Pos2::new(x, map.min.y), Pos2::new(x, map.max.y)],
                    Stroke::new(view.scale, Color32::from_gray(128)),
                );
            }
            for i in 0..=10 {
                let y = map.min.y + map.height() * i as f32 / 10.0;
                ui.painter().line_segment(
                    [Pos2::new(map.min.x, y), Pos2::new(map.max.x, y)],
                    Stroke::new(view.scale, Color32::from_gray(128)),
                );
            }
            for (i, pr) in self.profiles.iter().enumerate() {
                let (ax, ay) = map_to_image(pr.x0, pr.y0);
                let (bx, by) = map_to_image(pr.x1, pr.y1);
                let a = view.p(200.0 + ax, 40.0 + ay);
                let b = view.p(200.0 + bx, 40.0 + by);
                let col = if i == self.selected {
                    Color32::from_rgb(255, 60, 40)
                } else {
                    Color32::from_rgb(30, 80, 200)
                };
                ui.painter().line_segment(
                    [a, b],
                    Stroke::new(
                        (if i == self.selected { 2.5 } else { 1.5_f32 }) * view.scale,
                        col,
                    ),
                );
                ui.painter().circle_filled(a, 3.0 * view.scale, col);
                ui.painter().circle_filled(b, 3.0 * view.scale, col);
            }
            if let (Some(a), Some(b)) = (self.drag_start, self.drag_cur) {
                ui.painter()
                    .line_segment([a, b], Stroke::new(2.0_f32 * view.scale, Color32::YELLOW));
            }
            let mresp = ui.interact(map, egui::Id::new("map_area"), Sense::click_and_drag());
            // DFM: OnMouseDown=begin_prf. В egui clicked() приходит только на
            // отпускании и не срабатывает для решившегося drag — использовали
            // drag_started(), иначе линия профиля вообще не начиналась.
            if mresp.drag_started() {
                self.drag_start = mresp.interact_pointer_pos();
                self.drag_cur = self.drag_start;
            }
            if mresp.dragged() {
                self.drag_cur = mresp.interact_pointer_pos().or(self.drag_cur);
            }
            if mresp.drag_stopped() {
                if let (Some(a), Some(b)) = (self.drag_start, self.drag_cur) {
                    if (b - a).length() > 6.0 * view.scale {
                        let sx0 = (a.x - map.min.x) / map.width() * (MAP_IMG_W - 1) as f32;
                        let sy0 = (a.y - map.min.y) / map.height() * (MAP_IMG_H - 1) as f32;
                        let sx1 = (b.x - map.min.x) / map.width() * (MAP_IMG_W - 1) as f32;
                        let sy1 = (b.y - map.min.y) / map.height() * (MAP_IMG_H - 1) as f32;
                        let (x0, y0) = image_to_map(sx0, sy0);
                        let (x1, y1) = image_to_map(sx1, sy1);
                        self.add_profile(x0 as f32, y0 as f32, x1 as f32, y1 as f32);
                    }
                }
                self.drag_start = None;
                self.drag_cur = None;
            }

            // ── подписи и легенда Image3 ──────────────────────────────────────
            label(
                ui,
                view,
                self.fonts_ok,
                200.0,
                20.0,
                "Структурная карта",
                Face::Bold,
                13.0,
                CL_OLIVE,
            );
            label(
                ui,
                view,
                self.fonts_ok,
                456.0,
                448.0,
                "H min",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            label(
                ui,
                view,
                self.fonts_ok,
                748.0,
                448.0,
                "H max",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            let leg = view.r(LEGEND.0, LEGEND.1, LEGEND.2, LEGEND.3);
            let bw = leg.width() / BANDS as f32;
            for i in 0..BANDS {
                let cell = Rect::from_min_max(
                    Pos2::new(leg.min.x + i as f32 * bw, leg.min.y),
                    Pos2::new(leg.min.x + (i + 1) as f32 * bw, leg.max.y),
                );
                ui.painter().rect_filled(cell, 0.0, band_color(i, theme));
            }
            ui.painter().rect_stroke(
                leg,
                0.0,
                Stroke::new(view.scale.max(0.6), Color32::from_gray(120)),
            );

            // ── разрез Image2 (200,468,800,201) ───────────────────────────────
            label(
                ui,
                view,
                self.fonts_ok,
                200.0,
                448.0,
                "Временной разрез",
                Face::Bold,
                13.0,
                CL_TEAL,
            );
            label(
                ui,
                view,
                self.fonts_ok,
                960.0,
                452.0,
                "трассы",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            // Масштаб Image2, сетку и все 300 отсчётов берём ПОСЛЕ изменений
            // профиля в этом кадре, чтобы разрез не отставал от карты/таблицы.
            self.refresh_time_bounds(ui.ctx());
            let sec = view.r(SEC.0, SEC.1, SEC.2, SEC.3);
            let tb = self.time_bounds;
            let t_span = (tb.1 - tb.0).max(1e-6);
            // 0x465D68 целиком растеризуется в 800×201 текстуру с NEAREST —
            // это и original-сетка, и чёрные отсчёты, без per-frame Vec'ов.
            if let Some(tex) = &self.section_tex {
                ui.painter().image(
                    tex.id(),
                    sec,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.painter().rect_stroke(
                sec,
                0.0,
                Stroke::new(view.scale.max(0.6), Color32::from_gray(140)),
            );
            // DFM `Image2.OnMouseDown`: пик ставится в момент нажатия. `clicked()`
            // нужен только как fallback для очень быстрого клика, когда press и
            // release попали в один кадр; флаг не даёт добавить пик дважды.
            let sresp = ui.interact(sec, egui::Id::new("sec_area"), Sense::click_and_drag());
            let sec_down = sresp.is_pointer_button_down_on();
            let add_pick = (sec_down || sresp.clicked()) && !self.sec_pointer_down;
            if add_pick {
                if let Some(pnt) = sresp.interact_pointer_pos() {
                    let trace = ((pnt.x - sec.min.x) / sec.width() * 11.0) as f64;
                    // 0x468780: ось времени направлена вниз, как и в `0x465D68`.
                    let time = section_time_from_y(
                        f64::from(pnt.y - sec.min.y),
                        f64::from(sec.height()),
                        tb,
                    );
                    if self.profiles.is_empty() {
                        self.add_profile(100.0, 2000.0, 3100.0, 2000.0);
                    }
                    if let Some(pr) = self.profiles.get_mut(self.selected) {
                        pr.picks.push(Pick { trace, time });
                        pr.picks.sort_by(|a, b| {
                            a.trace
                                .partial_cmp(&b.trace)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        self.dirty = true;
                        self.status = format!("Пик: трасса {:.1}, t {:.3} с", trace, time);
                    }
                }
            }
            self.sec_pointer_down = sec_down;

            // Маркеры рисуем после вставки пика: новый пик виден в том же кадре.
            let picks: &[Pick] = self
                .profiles
                .get(self.selected)
                .map(|p| p.picks.as_slice())
                .unwrap_or(&[]);
            for pk in picks {
                let c = Pos2::new(
                    sec.min.x + (pk.trace / 11.0) as f32 * sec.width(),
                    sec.min.y + ((pk.time - tb.0) / t_span).clamp(0.0, 1.0) as f32 * sec.height(),
                );
                ui.painter()
                    .circle_filled(c, 3.0 * view.scale, Color32::from_rgb(220, 70, 30));
            }

            // ── нижние подписи ────────────────────────────────────────────────
            label(
                ui,
                view,
                self.fonts_ok,
                180.0,
                652.0,
                "t, с",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            // Label9 из DFM: clMaroon, Bold+Underline, x=872, y=672.
            // Текст длиннее оригинального, поэтому прижимаем его к правому
            // краю клиента — подпись всегда видна при любом размере окна.
            underlined_right_label(
                ui,
                view,
                self.fonts_ok,
                CW - 4.0,
                672.0,
                "Rust version Fold by Lion 2026",
                12.0,
                CL_MAROON,
            );
            // Служебный статус убран с нижней полосы: в DFM там только Label9.

            // ── окна ───────────────────────────────────────────────────────────
            if self.show_about {
                egui::Window::new("О программе")
                    .open(&mut self.show_about)
                    .show(ctx, |ui| {
                        ui.label("Fold — структурная карта отражающего горизонта");
                        ui.label(format!("Расчёт: {:.3} мс", self.last_ms));
                        ui.label("Палитра: ".to_owned() + THEMES[theme].name);
                        if !self.status.is_empty() {
                            ui.label(self.status.clone());
                        }
                        ui.label("Rust version Fold by Lion 2026");
                    });
            }

            if self.show_profiles {
                let mut close = false;
                let mut profiles_open = true;
                egui::Window::new("Профили")
                    .open(&mut profiles_open)
                    .default_size([220.0, 180.0])
                    .show(ctx, |ui| {
                        ui.label(format!(
                            "Профиль {} из {}",
                            self.selected + 1,
                            self.profiles.len()
                        ));
                        if ui.button("Назад").clicked() && self.selected > 0 {
                            self.select_profile(self.selected - 1);
                        }
                        if ui.button("Далее").clicked() && self.selected + 1 < self.profiles.len()
                        {
                            self.select_profile(self.selected + 1);
                        }
                        if ui.button("Ввести вручную").clicked() {
                            self.io_text = serialize(&self.profiles, &self.settings);
                            self.io_error.clear();
                            self.show_io = true;
                            close = true;
                        }
                        if ui.button("Удалить все").clicked() {
                            self.profiles.clear();
                            self.selected = 0;
                            self.sync_grid_text();
                            self.dirty = true;
                            self.status = "Все профили удалены".into();
                        }
                        if ui.button("Cтереть таблицу").clicked() {
                            self.clear_table();
                        }
                    });
                if close || !profiles_open {
                    self.show_profiles = false;
                }
            }

            if self.show_help {
                egui::Window::new("Справка")
                    .open(&mut self.show_help)
                    .default_size([450.0, 320.0])
                    .show(ctx, |ui| {
                        ui.label("Fold — быстрый порт программы 2007 года.");
                        ui.separator();
                        ui.label("V ср — средняя скорость, f — параметр сигнала.");
                        ui.label("Качество изображения — шаг сетки карты (0.1..9.99, как в DFM).");
                        ui.label("Х, м / Y, м — угол и координаты профиля.");
                        ui.label("Профиль задаётся перетаскиванием мыши по карте.");
                        ui.label("Пик на временном разрезе ставится кликом мыши.");
                        ui.label("Крестик в таблице удаляет одну строку.");
                        ui.label("Структура — о программе, Профили — навигация.");
                        ui.label("Строка — экспорт/импорт готовой строки данных.");
                        ui.label("Цвет — переключение палитры карты и легенды.");
                        ui.separator();
                        ui.label("Tab / Shift+Tab — переход между полями ввода.");
                        ui.label("Оригинал — забытый эксперимент. Лицензий и обещаний нет.");
                    });
            }

            let mut close_io = false;
            if self.show_io {
                egui::Window::new("Готовая строка данных")
                    .open(&mut self.show_io)
                    .default_size([540.0, 360.0])
                    .show(ctx, |ui| {
                        ui.label("Скопируйте строку или вставьте свою и нажмите «Применить».");
                        ui.add_sized(
                            [500.0, 240.0],
                            egui::TextEdit::multiline(&mut self.io_text)
                                .font(FontId::monospace(11.0))
                                .desired_rows(12),
                        );
                        if !self.io_error.is_empty() {
                            ui.colored_label(Color32::from_rgb(180, 40, 30), self.io_error.clone());
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Скопировать").clicked() {
                                ui.ctx().copy_text(self.io_text.clone());
                            }
                            if ui.button("Применить").clicked() {
                                match parse(&self.io_text) {
                                    Ok((profiles, settings)) => {
                                        let grid = profiles
                                            .first()
                                            .map(|p| {
                                                [
                                                    fmt_num(p.x0),
                                                    fmt_num(p.y0),
                                                    fmt_num(p.x1),
                                                    fmt_num(p.y1),
                                                ]
                                            })
                                            .unwrap_or_default();
                                        // Дробные значения из строки не должны
                                        // превращаться в целые в полях ввода.
                                        self.v = fmt_num(settings.velocity);
                                        self.quality = fmt_num(settings.quality);
                                        self.xang = fmt_num(settings.angle);
                                        self.yang = "0".into();
                                        self.profiles = profiles;
                                        self.settings = settings;
                                        self.selected = 0;
                                        self.xg = grid.get(2).cloned().unwrap_or_default();
                                        self.yg = grid.get(1).cloned().unwrap_or_default();
                                        self.grid_text = grid;
                                        self.map_key = None;
                                        self.time_key = None;
                                        self.dirty = true;
                                        self.io_error.clear();
                                        self.status =
                                            format!("Загружено профилей: {}", self.profiles.len());
                                    }
                                    Err(err) => self.io_error = err,
                                }
                            }
                            if ui.button("Закрыть").clicked() {
                                close_io = true;
                            }
                        });
                    });
            }
            if close_io {
                self.show_io = false;
            }
        });
        self.last_focused = ctx.memory(|m| m.focused());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_image_transform_is_inverted_with_y_flip() {
        // 0x466198/0x4661F0: (0,0) → левый нижний угол Image1,
        // (8000,4000) → правый верхний.
        assert_eq!(map_to_image(0.0, 0.0), (0.0, 399.0));
        assert_eq!(map_to_image(8000.0, 4000.0), (799.0, 0.0));

        for (x, y) in [
            (0.0, 0.0),
            (3100.0, 100.0),
            (8000.0, 4000.0),
            (1234.5, 2345.6),
        ] {
            let (sx, sy) = map_to_image(x, y);
            let (x2, y2) = image_to_map(sx, sy);
            assert!((x2 - x).abs() < 11.0, "{x},{y} → {sx},{sy} → {x2},{y2}");
            assert!((y2 - y).abs() < 11.0, "{x},{y} → {sx},{sy} → {x2},{y2}");
        }
    }

    #[test]
    fn palette_bands_match_map_colors() {
        // Легенда должна показывать те же 21 цветов, что и карта.
        assert_eq!(band_color(0, 0), palette(0.0, 0));
        assert_eq!(band_color(20, 0), palette(1.0, 0));
    }

    #[test]
    fn section_image_uses_dfm_grid_geometry() {
        let data = SectionData {
            t_min: 0.0,
            t_max: 1.0,
            n: 1,
            samples: vec![0; crate::grid::SECTION_SAMPLES],
        };
        let img = section_image(&data);
        assert_eq!((img.width(), img.height()), (SECTION_IMG_W, SECTION_IMG_H));
        // i=1 ⇒ y=trunc(201/10)=20, first horizontal grid line; x=0 is not
        // touched by sample traces (they start at x=trunc(800/2)=400).
        assert_eq!(img.pixels[20 * SECTION_IMG_W], Color32::from_gray(210));
        assert_eq!(img.pixels[0], Color32::WHITE);
    }
}
