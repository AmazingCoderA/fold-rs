//! GUI-клон оригинального окна Fold.
//!
//! Раскладка НЕ выдумана и не снята «на глаз»: все координаты, размеры,
//! цвета и размеры шрифтов взяты из бинарного DFM-ресурса `TFORM1`
//! оригинального `03_Fold.exe` (cm. `analysis/dfm_values.txt`).
//!
//! Ключевое открытие: скриншот 894x656 — это окно оригинала,
//! масштабированное на 7/8. Настоящий клиент формы: 1016x694.
//! Поэтому GUI рисуется в НАТИВНЫХ координатах DFM вообще без
//! масштабирования — это убирает «кривоту» и размытие линий/текста.
//!
//! Добавлен только 20-пиксельный слой меню сверху (в оригинале меню —
//! системное, вне клиентской области), чтобы все кнопки были доступны.

use crate::grid::{build_map, MapGrid};
use crate::io::{parse, serialize};
use crate::model::{Pick, Profile, Settings, MAP_HEIGHT_M, MAP_WIDTH_M};
use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, TextureHandle};
use std::sync::Arc;

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
/// Поля ввода в порядке Tab (TabOrder из DFM + системные поля).
const FIELDS: [&str; 7] = [
    "v_field",
    "f_field",
    "qual_field",
    "x_field",
    "y_field",
    "xg_field",
    "yg_field",
];

/// Цвета Delphi из DFM (clXxx → RGB).
const CL_OLIVE: Color32 = Color32::from_rgb(128, 128, 0);
const CL_TEAL: Color32 = Color32::from_rgb(0, 128, 128);
const CL_GREEN: Color32 = Color32::from_rgb(0, 128, 0);
const CL_NAVY: Color32 = Color32::from_rgb(0, 0, 128);
const CL_PURPLE: Color32 = Color32::from_rgb(128, 0, 128);
const CL_MAROON: Color32 = Color32::from_rgb(128, 0, 0);
const CL_MONEY_GREEN: Color32 = Color32::from_rgb(153, 204, 153);

#[derive(Clone, Copy)]
enum Face {
    Reg,
    Bold,
    BoldItalic,
    Symbol,
}

struct Theme {
    name: &'static str,
    stops: [(f32, [u8; 3]); 5],
}

/// Первая палитра — точная из original; остальные — «разнообразнее по цветам».
const THEMES: [Theme; 5] = [
    Theme {
        name: "Классика",
        stops: [
            (0.00, [70, 96, 205]),
            (0.25, [82, 204, 188]),
            (0.50, [82, 203, 80]),
            (0.75, [202, 202, 63]),
            (1.00, [198, 80, 64]),
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
    /// Ключ пересчёта текстуры: (угол, ширина, высота, палитра).
    map_key: Option<(f64, usize, usize, usize)>,
}

impl FoldApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let fonts_ok = install_fonts(cc);
        let settings = Settings {
            velocity: 1200.0,
            ..Default::default()
        };
        let profiles = vec![Profile {
            id: 0,
            x0: 100.0,
            y0: 100.0,
            x1: 3100.0,
            y1: 100.0,
            picks: vec![
                Pick {
                    trace: 1.0,
                    time: 3.465,
                },
                Pick {
                    trace: 3.0,
                    time: 3.474,
                },
                Pick {
                    trace: 5.0,
                    time: 3.290,
                },
                Pick {
                    trace: 7.0,
                    time: 3.524,
                },
                Pick {
                    trace: 10.0,
                    time: 3.520,
                },
            ],
        }];
        Self {
            profiles,
            settings,
            selected: 0,
            texture: None,
            last_ms: 0.0,
            dirty: true,
            v: "1200".into(),
            fval: "10".into(),
            quality: "95".into(),
            xang: "0".into(),
            yang: "0".into(),
            xg: "3100".into(),
            yg: "100".into(),
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
        }
    }

    fn font(&self, face: Face, size: f32) -> FontId {
        font_of(self.fonts_ok, face, size)
    }

    fn recalc(&mut self, ctx: &egui::Context) {
        let key = (
            self.settings.angle,
            self.settings.width,
            self.settings.height,
            self.theme,
        );
        self.dirty = false;
        if self.map_key == Some(key) && self.texture.is_some() {
            return;
        }

        let t = std::time::Instant::now();
        let g = build_map(&self.profiles, &self.settings);
        let (min, max) = bounds(&g);
        let span = (max - min).max(1e-6);
        let theme = self.theme;
        let mut img = egui::ColorImage::new([g.width, g.height], scale_color(0.0, theme));
        for (i, &d) in g.data.iter().enumerate() {
            if d.is_finite() {
                img.pixels[i] = palette(((d - min) / span).clamp(0.0, 1.0), theme);
            }
        }
        self.last_ms = t.elapsed().as_secs_f64() * 1000.0;
        match &mut self.texture {
            Some(tex) => tex.set(img, egui::TextureOptions::LINEAR),
            None => {
                self.texture =
                    Some(ctx.load_texture("struct_map", img, egui::TextureOptions::LINEAR));
            }
        }
        self.map_key = Some(key);
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
        self.status = format!("Профиль {}", idx + 1);
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

fn palette(u: f32, theme: usize) -> Color32 {
    let band = (u * BANDS as f32) as usize;
    let t = ((band.min(BANDS - 1) as f32 + 0.5) / BANDS as f32).clamp(0.0, 1.0);
    scale_color(t, theme)
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

/// Залитые чёрные виггл-лепестки (Ricker), как в original.
fn wiggle(ui: &egui::Ui, cx: f32, sec: Rect, pick_time: f64) {
    const N: usize = 160;
    let scale = (sec.height() / 174.0).max(0.2);
    let amp = 20.0 * scale;
    let freq = 18.0_f64;
    let step = sec.height() / N as f32;
    let mut pts = Vec::with_capacity(N + 1);
    for k in 0..=N {
        let fy = k as f32 / N as f32;
        let y = sec.min.y + fy * sec.height();
        let t = 3.6_f64 - f64::from(fy) * 0.4;
        let dt = (t - pick_time) * freq;
        let w = (1.0 - 2.0 * std::f64::consts::PI.powi(2) * dt * dt)
            * (-std::f64::consts::PI.powi(2) * dt * dt).exp();
        pts.push((y, cx + w as f32 * amp, w));
    }
    for &(y, x, w) in &pts {
        let x0 = x.min(cx);
        let x1 = x.max(cx);
        if (x1 - x0) > 0.6 && w.abs() > 0.02 {
            ui.painter().rect_filled(
                Rect::from_min_max(Pos2::new(x0, y), Pos2::new(x1, y + step + 0.6)),
                0.0,
                Color32::from_gray(15),
            );
        }
    }
    ui.painter().line_segment(
        [Pos2::new(cx, sec.min.y), Pos2::new(cx, sec.max.y)],
        Stroke::new(0.6_f32, Color32::from_gray(120)),
    );
    let line: Vec<Pos2> = pts.iter().map(|&(y, x, _)| Pos2::new(x, y)).collect();
    ui.painter().add(egui::Shape::line(
        line,
        Stroke::new(0.8_f32, Color32::BLACK),
    ));
}

// ── примитивы рисования в нативных координатах DFM ──────────────────────────

/// Rect из DFM-координат (x, y, width, height). Y сдвигается на слой меню.
fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_max(Pos2::new(x, y + MENU_H), Pos2::new(x + w, y + h + MENU_H))
}

fn p(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y + MENU_H)
}

fn font_of(fonts_ok: bool, face: Face, size: f32) -> FontId {
    if !fonts_ok {
        return FontId::proportional(size);
    }
    let name = match face {
        Face::Reg => "Arial",
        Face::Bold => "ArialB",
        Face::BoldItalic => "ArialBI",
        Face::Symbol => "Symbol",
    };
    FontId::new(size, FontFamily::Name(Arc::from(name)))
}

#[allow(clippy::too_many_arguments)]
fn label(
    ui: &egui::Ui,
    app: &FoldApp,
    x: f32,
    y: f32,
    s: impl Into<String>,
    face: Face,
    size: f32,
    color: Color32,
) {
    ui.painter().text(
        p(x, y),
        Align2::LEFT_TOP,
        s.into(),
        app.font(face, size),
        color,
    );
}

#[allow(clippy::too_many_arguments)]
fn centered(
    ui: &egui::Ui,
    app: &FoldApp,
    x: f32,
    y: f32,
    s: impl Into<String>,
    face: Face,
    size: f32,
    color: Color32,
) {
    ui.painter().text(
        p(x, y),
        Align2::CENTER_TOP,
        s.into(),
        app.font(face, size),
        color,
    );
}

/// Поле ввода VCL: белая подложка, утопленная обводка, синий фокус.
fn edit(ui: &mut egui::Ui, fonts_ok: bool, rect: Rect, val: &mut String, id: &'static str) -> bool {
    let id = egui::Id::new(id);
    let focused = ui.memory(|m| m.has_focus(id));
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    let dark = if focused {
        Color32::from_rgb(40, 110, 200)
    } else {
        Color32::from_gray(120)
    };
    let light = Color32::from_gray(250);
    ui.painter().line_segment(
        [rect.min, Pos2::new(rect.max.x, rect.min.y)],
        Stroke::new(1.0_f32, dark),
    );
    ui.painter().line_segment(
        [rect.min, Pos2::new(rect.min.x, rect.max.y)],
        Stroke::new(1.0_f32, dark),
    );
    ui.painter().line_segment(
        [Pos2::new(rect.min.x, rect.max.y), rect.max],
        Stroke::new(1.0_f32, light),
    );
    ui.painter().line_segment(
        [Pos2::new(rect.max.x, rect.min.y), rect.max],
        Stroke::new(1.0_f32, light),
    );
    let inner = Rect::from_min_max(
        Pos2::new(rect.min.x + 2.0, rect.min.y + 2.0),
        Pos2::new(rect.max.x - 2.0, rect.max.y - 2.0),
    );
    ui.put(
        inner,
        egui::TextEdit::singleline(val)
            .id(id)
            .frame(false)
            .font(font_of(fonts_ok, Face::Reg, 11.0)),
    )
    .changed()
}

fn button(ui: &mut egui::Ui, rect: Rect, id: &'static str, text: &str) -> bool {
    let mut clicked = false;
    ui.push_id(id, |ui| {
        let b = egui::Button::new(
            egui::RichText::new(text)
                .size(11.0)
                .color(Color32::from_gray(25)),
        )
        .fill(Color32::from_gray(238))
        .stroke(Stroke::new(1.0_f32, Color32::from_gray(125)));
        clicked = ui.put(rect, b).clicked();
    });
    clicked
}

impl eframe::App for FoldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dirty {
            self.recalc(ctx);
        }

        // Явный Tab/Shift+Tab по всем полям ввода (DFM TabOrder + доп. поля).
        let shift = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key: egui::Key::Tab,
                    pressed: true,
                    modifiers,
                    ..
                } => Some(modifiers.shift),
                _ => None,
            })
        });
        if let Some(shift) = shift {
            ctx.input_mut(|i| {
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::Key {
                            key: egui::Key::Tab,
                            ..
                        }
                    )
                })
            });
            let cur = ctx.memory(|m| m.focused());
            let pos = cur.and_then(|id| FIELDS.iter().position(|f| id == egui::Id::new(*f)));
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

        egui::CentralPanel::default().show(ctx, |ui| {
            let area = ui.max_rect();
            ui.painter().rect_filled(area, 0.0, Color32::WHITE); // DFM Color=clWhite

            // ── слой меню (20 px) ────────────────────────────────────────────
            let menu = Rect::from_min_max(area.min, Pos2::new(area.max.x, area.min.y + MENU_H));
            ui.painter().rect_filled(menu, 0.0, Color32::from_gray(236));
            ui.painter().line_segment(
                [
                    Pos2::new(area.min.x, menu.max.y),
                    Pos2::new(area.max.x, menu.max.y),
                ],
                Stroke::new(1.0_f32, Color32::from_gray(180)),
            );
            let mb = |x: f32, w: f32| {
                Rect::from_min_max(Pos2::new(x, 1.0), Pos2::new(x + w, MENU_H - 1.0))
            };
            if button(ui, mb(4.0, 78.0), "m_about", "Структура") {
                self.show_about = !self.show_about;
            }
            if button(ui, mb(86.0, 84.0), "m_profiles", "Профили") {
                self.show_profiles = !self.show_profiles;
            }
            if button(ui, mb(174.0, 118.0), "m_clear", "Стереть таблицу") {
                self.clear_table();
            }
            if button(ui, mb(296.0, 62.0), "m_io", "Строка") {
                self.io_text = serialize(&self.profiles, &self.settings);
                self.io_error.clear();
                self.show_io = true;
            }
            if button(ui, mb(860.0, 60.0), "m_theme", "Цвет") {
                self.theme = (self.theme + 1) % THEMES.len();
                self.dirty = true;
                self.status = format!("Палитра: {}", THEMES[self.theme].name);
            }
            if button(ui, mb(924.0, 88.0), "m_help", "Справка") {
                self.show_help = !self.show_help;
            }

            let theme = self.theme;

            // ── Label10: заголовок (Align=alTop, taCenter, BoldItalic 15) ─────
            centered(
                ui,
                self,
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
                self,
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
                r(828.0, 16.0, 65.0, 20.0),
                &mut self.xang,
                "x_field",
            ) {
                if let Ok(a) = self.xang.parse() {
                    self.settings.angle = a;
                    self.dirty = true;
                }
            }
            label(
                ui,
                self,
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
                r(934.0, 16.0, 65.0, 20.0),
                &mut self.yang,
                "y_field",
            ) && self.yang.parse::<f64>().is_ok()
            {
                // вторая координата — запасная, карту не меняет
            }

            // ── Panel1 (левая панель, bsSingle, clWhite) ─────────────────────
            let panel = r(PANEL.0, PANEL.1, PANEL.2, PANEL.3);
            ui.painter().rect_filled(panel, 0.0, Color32::WHITE);
            ui.painter()
                .rect_stroke(panel, 0.0, Stroke::new(1.0_f32, Color32::from_gray(140)));
            label(
                ui,
                self,
                12.0,
                44.0,
                "Параметры структуры",
                Face::Bold,
                13.0,
                CL_GREEN,
            );
            label(
                ui,
                self,
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
                r(64.0, 68.0, 77.0, 22.0),
                &mut self.v,
                "v_field",
            ) {
                if let Ok(x) = self.v.parse() {
                    self.settings.velocity = x;
                }
            }
            label(
                ui,
                self,
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
                r(64.0, 104.0, 77.0, 22.0),
                &mut self.fval,
                "f_field",
            ) && self.fval.parse::<f64>().is_ok()
            {
                // f из DFM не влияет на поверхность (см. analysis/map_reverse_report.md)
            }
            label(
                ui,
                self,
                12.0,
                140.0,
                "Координаты профиля",
                Face::Bold,
                13.0,
                CL_NAVY,
            );
            label(
                ui,
                self,
                32.0,
                162.0,
                "Профиль №",
                Face::Bold,
                11.0,
                CL_MAROON,
            );

            // StringGrid1: 2 колонки × 4 строки, DefaultColWidth=60, RowHeight=24.
            let grid = r(GRID.0, GRID.1, GRID.2, GRID.3);
            ui.painter().rect_filled(grid, 0.0, Color32::WHITE);
            for row in 0..4 {
                let y = grid.min.y + row as f32 * 24.0;
                let bg = if row == 0 {
                    CL_MONEY_GREEN
                } else {
                    Color32::WHITE
                };
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        Pos2::new(grid.min.x, y),
                        Pos2::new(grid.max.x, (y + 24.0).min(grid.max.y)),
                    ),
                    0.0,
                    bg,
                );
                let cells = [
                    format!(
                        "{:.0}",
                        self.profiles
                            .get(self.selected)
                            .map(|p| p.x0)
                            .unwrap_or(0.0)
                    ),
                    format!(
                        "{:.0}",
                        self.profiles
                            .get(self.selected)
                            .map(|p| p.y0)
                            .unwrap_or(0.0)
                    ),
                    format!(
                        "{:.0}",
                        self.profiles
                            .get(self.selected)
                            .map(|p| p.x1)
                            .unwrap_or(0.0)
                    ),
                    format!(
                        "{:.0}",
                        self.profiles
                            .get(self.selected)
                            .map(|p| p.y1)
                            .unwrap_or(0.0)
                    ),
                ];
                for col in 0..2 {
                    let idx = row * 2 + col;
                    if idx < cells.len() {
                        label(
                            ui,
                            self,
                            grid.min.x + 2.0 + col as f32 * 60.0,
                            y + 4.0,
                            cells[idx].clone(),
                            Face::Bold,
                            11.0,
                            Color32::BLACK,
                        );
                    }
                }
            }
            ui.painter()
                .rect_stroke(grid, 0.0, Stroke::new(1.0_f32, Color32::from_gray(140)));
            for col in 1..2 {
                let x = grid.min.x + col as f32 * 60.0;
                ui.painter().line_segment(
                    [Pos2::new(x, grid.min.y), Pos2::new(x, grid.max.y)],
                    Stroke::new(1.0_f32, Color32::from_gray(160)),
                );
            }
            for row in 1..4 {
                let y = grid.min.y + row as f32 * 24.0;
                ui.painter().line_segment(
                    [Pos2::new(grid.min.x, y), Pos2::new(grid.max.x, y)],
                    Stroke::new(1.0_f32, Color32::from_gray(160)),
                );
            }

            // Label8 из DFM: 'Таблица корреляции' (clPurple, Bold 13).
            label(
                ui,
                self,
                12.0,
                296.0,
                "Таблица корреляции",
                Face::Bold,
                13.0,
                CL_PURPLE,
            );

            // Memo1 (ссVertical) — пики t0, t1… с крестиком удаления строки.
            let memo = r(MEMO.0, MEMO.1, MEMO.2, MEMO.3);
            ui.painter().rect_filled(memo, 0.0, Color32::WHITE);
            ui.painter()
                .rect_stroke(memo, 0.0, Stroke::new(1.0_f32, Color32::from_gray(140)));
            let picks: Vec<Pick> = self
                .profiles
                .get(self.selected)
                .map(|p| p.picks.clone())
                .unwrap_or_default();
            let mut delete_row = None;
            for (i, pk) in picks.iter().enumerate() {
                let y = memo.min.y + 2.0 + i as f32 * 15.0;
                if y + 14.0 > memo.max.y {
                    break;
                }
                label(
                    ui,
                    self,
                    memo.min.x + 3.0,
                    y,
                    format!("t{} = {:.3} с", i, pk.time),
                    Face::Reg,
                    11.0,
                    Color32::BLACK,
                );
                let cross = Rect::from_min_max(
                    Pos2::new(memo.max.x - 13.0, y),
                    Pos2::new(memo.max.x - 2.0, y + 12.0),
                );
                let resp = ui.interact(cross, egui::Id::new(("del_pick", i)), Sense::click());
                let cc = if resp.hovered() {
                    Color32::from_rgb(200, 40, 30)
                } else {
                    Color32::from_gray(150)
                };
                ui.painter().line_segment(
                    [
                        Pos2::new(cross.min.x + 2.0, cross.min.y + 2.0),
                        Pos2::new(cross.max.x - 2.0, cross.max.y - 2.0),
                    ],
                    Stroke::new(1.0_f32, cc),
                );
                ui.painter().line_segment(
                    [
                        Pos2::new(cross.max.x - 2.0, cross.min.y + 2.0),
                        Pos2::new(cross.min.x + 2.0, cross.max.y - 2.0),
                    ],
                    Stroke::new(1.0_f32, cc),
                );
                if resp.clicked() {
                    delete_row = Some(i);
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
                self,
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
                r(116.0, 636.0, 41.0, 22.0),
                &mut self.quality,
                "qual_field",
            ) {
                if let Ok(q) = self.quality.parse() {
                    self.settings.quality = q;
                }
            }
            // ── карта Image1 (200,40,800,400) ─────────────────────────────────
            let map = r(MAP.0, MAP.1, MAP.2, MAP.3);
            ui.painter().rect_filled(map, 0.0, scale_color(0.0, theme));
            if let Some(tex) = &self.texture {
                ui.painter().image(
                    tex.id(),
                    map,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            ui.painter()
                .rect_stroke(map, 0.0, Stroke::new(1.0_f32, Color32::from_gray(90)));
            // 8 вертикальных (шаг 100 = 800/8) и 10 горизонтальных (шаг 40).
            for i in 0..=8 {
                let x = map.min.x + map.width() * i as f32 / 8.0;
                ui.painter().line_segment(
                    [Pos2::new(x, map.min.y), Pos2::new(x, map.max.y)],
                    Stroke::new(0.7_f32, Color32::from_gray(110)),
                );
            }
            for i in 0..=10 {
                let y = map.min.y + map.height() * i as f32 / 10.0;
                ui.painter().line_segment(
                    [Pos2::new(map.min.x, y), Pos2::new(map.max.x, y)],
                    Stroke::new(0.7_f32, Color32::from_gray(110)),
                );
            }
            for (i, pr) in self.profiles.iter().enumerate() {
                let a = p(
                    200.0 + pr.x0 as f32 / MAP_WIDTH_M as f32 * 800.0,
                    40.0 + pr.y0 as f32 / MAP_HEIGHT_M as f32 * 400.0,
                );
                let b = p(
                    200.0 + pr.x1 as f32 / MAP_WIDTH_M as f32 * 800.0,
                    40.0 + pr.y1 as f32 / MAP_HEIGHT_M as f32 * 400.0,
                );
                let col = if i == self.selected {
                    Color32::from_rgb(255, 60, 40)
                } else {
                    Color32::from_rgb(30, 80, 200)
                };
                ui.painter().line_segment(
                    [a, b],
                    Stroke::new(if i == self.selected { 2.5 } else { 1.5_f32 }, col),
                );
                ui.painter().circle_filled(a, 3.0, col);
                ui.painter().circle_filled(b, 3.0, col);
            }
            if let (Some(a), Some(b)) = (self.drag_start, self.drag_cur) {
                ui.painter()
                    .line_segment([a, b], Stroke::new(2.0_f32, Color32::YELLOW));
            }
            let mresp = ui.interact(map, egui::Id::new("map_area"), Sense::click_and_drag());
            if mresp.clicked() {
                self.drag_start = mresp.interact_pointer_pos();
                self.drag_cur = self.drag_start;
            }
            if mresp.dragged() {
                self.drag_cur = mresp.interact_pointer_pos().or(self.drag_cur);
            }
            if mresp.drag_stopped() {
                if let (Some(a), Some(b)) = (self.drag_start, self.drag_cur) {
                    if (b - a).length() > 6.0 {
                        let x0 = (a.x - map.min.x) / map.width() * MAP_WIDTH_M as f32;
                        let y0 = (a.y - map.min.y) / map.height() * MAP_HEIGHT_M as f32;
                        let x1 = (b.x - map.min.x) / map.width() * MAP_WIDTH_M as f32;
                        let y1 = (b.y - map.min.y) / map.height() * MAP_HEIGHT_M as f32;
                        self.add_profile(x0, y0, x1, y1);
                    }
                }
                self.drag_start = None;
                self.drag_cur = None;
            }

            // ── подписи и легенда Image3 ──────────────────────────────────────
            label(
                ui,
                self,
                200.0,
                20.0,
                "Структурная карта",
                Face::Bold,
                13.0,
                CL_OLIVE,
            );
            label(
                ui,
                self,
                456.0,
                448.0,
                "H min",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            label(
                ui,
                self,
                748.0,
                448.0,
                "H max",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            let leg = r(LEGEND.0, LEGEND.1, LEGEND.2, LEGEND.3);
            let bw = leg.width() / BANDS as f32;
            for i in 0..BANDS {
                let cell = Rect::from_min_max(
                    Pos2::new(leg.min.x + i as f32 * bw, leg.min.y),
                    Pos2::new(leg.min.x + (i + 1) as f32 * bw, leg.max.y),
                );
                ui.painter().rect_filled(
                    cell,
                    0.0,
                    scale_color((i as f32 + 0.5) / BANDS as f32, theme),
                );
            }
            ui.painter()
                .rect_stroke(leg, 0.0, Stroke::new(1.0_f32, Color32::from_gray(120)));

            // ── разрез Image2 (200,468,800,201) ───────────────────────────────
            label(
                ui,
                self,
                200.0,
                448.0,
                "Временной разрез",
                Face::Bold,
                13.0,
                CL_TEAL,
            );
            label(
                ui,
                self,
                960.0,
                452.0,
                "трассы",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            let sec = r(SEC.0, SEC.1, SEC.2, SEC.3);
            ui.painter()
                .rect_filled(sec, 0.0, Color32::from_rgb(253, 253, 252));
            for i in 0..=10 {
                let y = sec.min.y + sec.height() * i as f32 / 10.0;
                ui.painter().line_segment(
                    [Pos2::new(sec.min.x, y), Pos2::new(sec.max.x, y)],
                    Stroke::new(0.6_f32, Color32::from_gray(210)),
                );
            }
            for i in 1..=10 {
                let x = sec.min.x + sec.width() * i as f32 / 11.0;
                ui.painter().line_segment(
                    [Pos2::new(x, sec.min.y), Pos2::new(x, sec.max.y)],
                    Stroke::new(0.6_f32, Color32::from_gray(210)),
                );
            }
            ui.painter()
                .rect_stroke(sec, 0.0, Stroke::new(1.0_f32, Color32::from_gray(140)));
            for i in 1..=10 {
                if let Some(pk) = picks.iter().min_by(|a, b| {
                    (a.trace - i as f64)
                        .abs()
                        .partial_cmp(&(b.trace - i as f64).abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    if (pk.trace - i as f64).abs() <= 1.5 {
                        wiggle(ui, sec.min.x + sec.width() * i as f32 / 11.0, sec, pk.time);
                    }
                }
            }
            for pk in &picks {
                let c = Pos2::new(
                    sec.min.x + (pk.trace / 11.0) as f32 * sec.width(),
                    sec.max.y - ((pk.time - 3.2) / 0.4).clamp(0.0, 1.0) as f32 * sec.height(),
                );
                ui.painter()
                    .circle_filled(c, 3.0, Color32::from_rgb(220, 70, 30));
            }
            let sresp = ui.interact(sec, egui::Id::new("sec_area"), Sense::click());
            if sresp.clicked() {
                let pnt = sresp.interact_pointer_pos().unwrap_or(sec.min);
                let trace = ((pnt.x - sec.min.x) / sec.width() * 11.0) as f64;
                let time = 3.2_f64 + f64::from(1.0 - (pnt.y - sec.min.y) / sec.height()) * 0.4;
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

            // ── нижние подписи ────────────────────────────────────────────────
            label(
                ui,
                self,
                180.0,
                652.0,
                "t, с",
                Face::Bold,
                11.0,
                Color32::BLACK,
            );
            // Label9 original: 'by Viktor Romanov, 2007' (clMaroon, BoldUnderline).
            // Требуемая подпись сохранена, позиция/стиль — из DFM.
            label(
                ui,
                self,
                872.0,
                672.0,
                "Rust version Fold by Lion 2026",
                Face::BoldItalic,
                12.0,
                CL_MAROON,
            );
            label(
                ui,
                self,
                8.0,
                680.0,
                &self.status,
                Face::Reg,
                9.0,
                Color32::from_gray(120),
            );

            // ── окна ───────────────────────────────────────────────────────────
            if self.show_about {
                egui::Window::new("О программе")
                    .open(&mut self.show_about)
                    .show(ctx, |ui| {
                        ui.label("Fold — структурная карта отражающего горизонта");
                        ui.label(format!("Расчёт: {:.3} мс", self.last_ms));
                        ui.label("Палитра: ".to_owned() + THEMES[theme].name);
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
                        ui.label("Качество изображения — точность построения карты.");
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
                                        self.v = format!("{:.0}", settings.velocity);
                                        self.quality = format!("{:.0}", settings.quality);
                                        self.profiles = profiles;
                                        self.settings = settings;
                                        self.selected = 0;
                                        self.map_key = None;
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
    }
}
