//! GUI-клон оригинального окна Fold 1:1 (894x656).
//!
//! Раскладка восстановлена по единственному разрешённому скриншоту и DFM
//! оригинального .exe (лежит в корне как 03_Fold.exe):
//!   - меню: Структура / Профили / Стереть таблицу / Строка / Цвет / Справка
//!   - левый столбец: Параметры структуры, V ср, f, Координаты профиля,
//!     Профиль №, Таблица корреляции, Качество изображения
//!   - карта: 8 вертикальных и 10 горизонтальных делений (шаг 35 px в original)
//!   - низ: цветовая легенда 21 полоса, H min / H max, временной разрез

use crate::grid::{build_map, MapGrid};
use crate::io::{parse, serialize};
use crate::model::{Pick, Profile, Settings, MAP_HEIGHT_M, MAP_WIDTH_M};
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, TextureHandle};

const W: f32 = 894.0;
const H: f32 = 656.0;
const LEFT: f32 = 160.0;
const MAP_TOP: f32 = 83.0;
const MAP_BOTTOM: f32 = 432.0;
const SEC_TOP: f32 = 456.0;
const SEC_BOTTOM: f32 = 630.0;

/// Геометрия легенды снята пиксель-в-пиксель со скриншота:
/// x=440..653, y=436..453, 21 полоса, синий → голубой → зелёный → жёлтый → красный.
const LEGEND_X0: f32 = 440.0;
const LEGEND_X1: f32 = 653.0;
const LEGEND_Y0: f32 = 436.0;
const LEGEND_Y1: f32 = 453.0;
const BANDS: usize = 21;

/// Поля ввода левой панели в порядке обхода Tab.
const FIELDS: [&str; 4] = ["v_field", "f_field", "xg_field", "yg_field"];

struct Theme {
    name: &'static str,
    stops: [(f32, [u8; 3]); 5],
}

/// Палитры карты. Первая — точная палитра original; остальные — чтобы было
/// «разнообразнее по цветам», переключаются кнопкой «Цвет».
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
    grid: Option<MapGrid>,
    texture: Option<TextureHandle>,
    last_ms: f64,
    dirty: bool,

    v: String,
    fval: String,
    xg: String,
    yg: String,
    status: String,
    drag_start: Option<Pos2>,
    drag_cur: Option<Pos2>,
    show_about: bool,
    show_help: bool,
    show_io: bool,
    io_text: String,
    io_error: String,
    theme: usize,
    /// Ключ, при котором текстура уже пересчитана: (угол, ширина, высота, тема).
    /// Пока ключ не изменился — карту не трогаем вообще (не теряем кадры).
    map_key: Option<(f64, usize, usize, usize)>,
}

impl FoldApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut settings = Settings::default();
        settings.velocity = 1200.0;
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
            grid: None,
            texture: None,
            last_ms: 0.0,
            dirty: true,
            v: "1200".into(),
            fval: "10".into(),
            xg: "3100".into(),
            yg: "100".into(),
            status: String::new(),
            drag_start: None,
            drag_cur: None,
            show_about: false,
            show_help: false,
            show_io: false,
            io_text: String::new(),
            io_error: String::new(),
            theme: 0,
            map_key: None,
        }
    }

    fn recalc(&mut self, ctx: &egui::Context) {
        // Карта зависит только от угла, размеров сетки и палитры. Скорость,
        // качество и пики на неё не влияют — не пересчитываем зря.
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
        self.grid = Some(g);
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
        let y = (y0 + y1) * 0.5;
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
        self.yg = format!("{:.0}", y);
        self.status = format!("Профиль {} добавлен", id + 1);
        self.dirty = true;
    }

    fn clear_table(&mut self) {
        if let Some(p) = self.profiles.get_mut(self.selected) {
            p.picks.clear();
            self.dirty = true;
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

/// Диапазон глубин считается один раз за пересчёт, а не для каждого пикселя.
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

/// Цвет одного из 21 бина шкалы (ровно как в original).
fn palette(u: f32, theme: usize) -> Color32 {
    let band = (u * BANDS as f32) as usize;
    let t = ((band.min(BANDS - 1) as f32 + 0.5) / BANDS as f32).clamp(0.0, 1.0);
    scale_color(t, theme)
}

/// Интерполяция стопов выбранной палитры.
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

/// Сейсмическая трасса «виггл» в точности как в original: пухлые залитые
/// чёрные лепестки вокруг тонкой оси ampiтудой ~20 px (замерено по скриншоту:
/// у трассы x=562 главный лепесток занимает x=563..583, y=501..519).
/// Формируем Ricker-вейвлет: один широкий главный лепесток и два узких боковых.
fn wiggle(ui: &egui::Ui, cx: f32, sec: Rect, pick_time: f64) {
    const N: usize = 160;
    let scale = (sec.height() / 174.0).max(0.2);
    let amp = 20.0 * scale;
    let freq = 18.0_f64; // Гц: main lobe ≈ 22 px при диапазоне 0.4 с / 174 px
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
    // Заливаем каждый лепесток полосками — надёжнее, чем невыпуклый полигон.
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
    // Тонкая ось и контур поверх заливки — чёткие края без «мыла».
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

/// Рисует легенду-шкалу внизу: 21 полоса, подписи H min / H max и тонкая рамка.
fn color_legend(ui: &egui::Ui, rr: impl Fn(f32, f32, f32, f32) -> Rect, theme: usize) {
    let bar = rr(LEGEND_X0, LEGEND_Y0, LEGEND_X1, LEGEND_Y1);
    let bw = bar.width() / BANDS as f32;
    for i in 0..BANDS {
        let cell = Rect::from_min_max(
            Pos2::new(bar.min.x + i as f32 * bw, bar.min.y),
            Pos2::new(bar.min.x + (i + 1) as f32 * bw, bar.max.y),
        );
        ui.painter().rect_filled(
            cell,
            0.0,
            scale_color((i as f32 + 0.5) / BANDS as f32, theme),
        );
    }
    ui.painter()
        .rect_stroke(bar, 0.0, Stroke::new(1.0_f32, Color32::from_gray(120)));
}

fn text(ui: &egui::Ui, x: f32, y: f32, s: impl Into<String>, size: f32, color: Color32) {
    ui.painter().text(
        Pos2::new(x, y),
        egui::Align2::LEFT_TOP,
        s.into(),
        FontId::proportional(size),
        color,
    );
}

fn centered(ui: &egui::Ui, x: f32, y: f32, s: impl Into<String>, size: f32, color: Color32) {
    ui.painter().text(
        Pos2::new(x, y),
        egui::Align2::CENTER_TOP,
        s.into(),
        FontId::proportional(size),
        color,
    );
}

/// Поле ввода в стиле VCL: белая подложка, утопленная обводка, заметный фокус.
/// `frame(false)` у TextEdit убран — рамка рисуется здесь и не мигает при вводе.
fn edit(ui: &mut egui::Ui, rect: Rect, val: &mut String, id: &'static str) -> bool {
    let id = egui::Id::new(id);
    let focused = ui.memory(|m| m.has_focus(id));
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);

    let shadow = if focused {
        Color32::from_rgb(40, 110, 200)
    } else {
        Color32::from_gray(120)
    };
    let light = Color32::from_gray(250);
    ui.painter().line_segment(
        [rect.min, Pos2::new(rect.max.x, rect.min.y)],
        Stroke::new(1.0_f32, shadow),
    );
    ui.painter().line_segment(
        [rect.min, Pos2::new(rect.min.x, rect.max.y)],
        Stroke::new(1.0_f32, shadow),
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
        Pos2::new(rect.min.x + 3.0, rect.min.y + 2.0),
        Pos2::new(rect.max.x - 3.0, rect.max.y - 2.0),
    );
    ui.put(
        inner,
        egui::TextEdit::singleline(val)
            .id(id)
            .frame(false)
            .font(FontId::proportional(11.0)),
    )
    .changed()
}

/// Обычная egui-кнопка с фиксированным_fill/stroke: нет градиентных швов,
/// нет артефактов при масштабировании, работает фокус с клавиатуры.
fn button(ui: &mut egui::Ui, rect: Rect, id: &'static str, label: &str) -> bool {
    let mut clicked = false;
    ui.push_id(id, |ui| {
        let b = egui::Button::new(
            egui::RichText::new(label)
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

        // Tab / Shift+Tab: явный обход четырёх полей левой панели.
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
            let r = ui.max_rect();
            // масштабирование при изменении размера окна
            let sx = r.width() / W;
            let sy = r.height() / H;
            let px = |x: f32| r.min.x + x * sx;
            let py = |y: f32| r.min.y + y * sy;
            let rr = |x0: f32, y0: f32, x1: f32, y1: f32| {
                Rect::from_min_max(Pos2::new(px(x0), py(y0)), Pos2::new(px(x1), py(y1)))
            };
            let theme = self.theme;

            ui.painter()
                .rect_filled(r, 0.0, Color32::from_rgb(246, 246, 244));

            // ---- меню ----
            let menu = rr(0.0, 20.0, W, 48.0);
            ui.painter().rect_filled(menu, 0.0, Color32::from_gray(236));
            ui.painter().line_segment(
                [
                    Pos2::new(r.min.x, menu.max.y),
                    Pos2::new(r.max.x, menu.max.y),
                ],
                Stroke::new(1.0_f32, Color32::from_gray(180)),
            );
            if button(ui, rr(6.0, 24.0, 76.0, 44.0), "btn_about", "Структура") {
                self.show_about = !self.show_about;
            }
            if button(ui, rr(80.0, 24.0, 132.0, 44.0), "btn_profiles", "Профили")
                && !self.profiles.is_empty()
            {
                let next = (self.selected + 1) % self.profiles.len();
                self.select_profile(next);
            }
            if button(
                ui,
                rr(136.0, 24.0, 218.0, 44.0),
                "btn_clear",
                "Стереть таблицу",
            ) {
                self.clear_table();
            }
            if button(ui, rr(222.0, 24.0, 300.0, 44.0), "btn_io", "Строка") {
                self.io_text = serialize(&self.profiles, &self.settings);
                self.io_error.clear();
                self.show_io = true;
            }
            // Дополнительные кнопки справа — не задевают раскладку original.
            if button(ui, rr(712.0, 24.0, 790.0, 44.0), "btn_theme", "Цвет") {
                self.theme = (self.theme + 1) % THEMES.len();
                self.dirty = true;
                self.status = format!("Палитра: {}", THEMES[self.theme].name);
            }
            if button(ui, rr(794.0, 24.0, 880.0, 44.0), "btn_help", "Справка") {
                self.show_help = !self.show_help;
            }

            // ---- левый столбец ----
            let left = rr(0.0, 48.0, LEFT, H);
            ui.painter()
                .rect_filled(left, 0.0, Color32::from_rgb(240, 240, 236));
            ui.painter().line_segment(
                [
                    Pos2::new(left.min.x + 1.0 * sx, r.min.y),
                    Pos2::new(left.min.x + 1.0 * sx, r.max.y),
                ],
                Stroke::new(1.0_f32, Color32::from_gray(250)),
            );
            ui.painter().line_segment(
                [
                    Pos2::new(left.max.x, r.min.y),
                    Pos2::new(left.max.x, r.max.y),
                ],
                Stroke::new(1.0_f32, Color32::from_gray(180)),
            );
            ui.painter().line_segment(
                [Pos2::new(px(180.0), r.min.y), Pos2::new(px(180.0), r.max.y)],
                Stroke::new(1.0_f32, Color32::from_gray(180)),
            );

            text(
                ui,
                px(18.0),
                py(89.0),
                "Параметры структуры",
                12.0,
                Color32::from_gray(30),
            );
            text(ui, px(17.0), py(110.0), "V ср", 12.0, Color32::BLACK);
            if edit(ui, rr(66.0, 106.0, 124.0, 124.0), &mut self.v, "v_field") {
                if let Ok(x) = self.v.parse() {
                    self.settings.velocity = x; // в карту не входит — текстуру не трогаем
                }
            }
            text(ui, px(18.0), py(140.0), "f", 12.0, Color32::BLACK);
            edit(ui, rr(70.0, 136.0, 124.0, 154.0), &mut self.fval, "f_field");

            text(
                ui,
                px(18.0),
                py(173.0),
                "Координаты профиля",
                12.0,
                Color32::from_gray(30),
            );
            text(ui, px(35.0), py(192.0), "Профиль №", 12.0, Color32::BLACK);
            text(ui, px(37.0), py(254.0), "X", 12.0, Color32::BLACK);
            edit(ui, rr(91.0, 248.0, 151.0, 266.0), &mut self.xg, "xg_field");
            text(ui, px(37.0), py(277.0), "Y", 12.0, Color32::BLACK);
            edit(ui, rr(97.0, 271.0, 151.0, 289.0), &mut self.yg, "yg_field");

            text(
                ui,
                px(17.0),
                py(309.0),
                "Таблица корреляции",
                12.0,
                Color32::from_gray(30),
            );
            // Таблица корреляции — настоящая сетка, как StringGrid1 в original:
            // рамка x=34..150, шаг строки 12 px (замерено по пикселям).
            let table = rr(34.0, 328.0, 150.0, 566.0);
            ui.painter().rect_filled(table, 0.0, Color32::WHITE);
            ui.painter()
                .rect_stroke(table, 0.0, Stroke::new(1.0_f32, Color32::from_gray(140)));
            let picks_for_table: Vec<Pick> = self
                .profiles
                .get(self.selected)
                .map(|p| p.picks.clone())
                .unwrap_or_default();
            let rows = ((table.height() / (12.0 * (H / 656.0))).floor() as usize).max(1);
            let row_h = table.height() / rows as f32;
            let mut delete_row = None;
            for (i, pk) in picks_for_table.iter().take(rows).enumerate() {
                let y0 = table.min.y + i as f32 * row_h;
                let line_col = if i % 2 == 0 {
                    Color32::from_gray(228)
                } else {
                    Color32::from_gray(240)
                };
                ui.painter().line_segment(
                    [Pos2::new(table.min.x, y0), Pos2::new(table.max.x, y0)],
                    Stroke::new(0.6_f32, line_col),
                );
                text(
                    ui,
                    table.min.x + 4.0,
                    y0 + (row_h - 10.0 * (H / 656.0)) * 0.5,
                    format!("t{} = {:.3} с", i, pk.time),
                    9.0,
                    Color32::from_gray(35),
                );
                // Крестик удаления строки — «если не там, можно стереть одну».
                let cross = Rect::from_min_max(
                    Pos2::new(table.max.x - 12.0 * sx, y0 + 1.0),
                    Pos2::new(table.max.x - 2.0 * sx, y0 + row_h - 1.0),
                );
                let resp = ui.interact(cross, egui::Id::new(("del_pick", i)), Sense::click());
                let cc = if resp.hovered() {
                    Color32::from_rgb(200, 40, 30)
                } else {
                    Color32::from_gray(140)
                };
                let m = 2.5_f32 * sx;
                ui.painter().line_segment(
                    [
                        Pos2::new(cross.min.x + m, cross.min.y + m),
                        Pos2::new(cross.max.x - m, cross.max.y - m),
                    ],
                    Stroke::new(1.0_f32, cc),
                );
                ui.painter().line_segment(
                    [
                        Pos2::new(cross.max.x - m, cross.min.y + m),
                        Pos2::new(cross.min.x + m, cross.max.y - m),
                    ],
                    Stroke::new(1.0_f32, cc),
                );
                if resp.clicked() {
                    delete_row = Some(i);
                }
            }
            if let Some(i) = delete_row {
                if let Some(p) = self.profiles.get_mut(self.selected) {
                    if i < p.picks.len() {
                        p.picks.remove(i);
                        self.dirty = true;
                        self.status = format!("Строка {} удалена", i + 1);
                    }
                }
            }
            if button(ui, rr(15.0, 566.0, 82.0, 585.0), "btn_prev", "Назад") && self.selected > 0
            {
                self.select_profile(self.selected - 1);
            }
            if button(ui, rr(88.0, 566.0, 155.0, 585.0), "btn_next", "Далее")
                && self.selected + 1 < self.profiles.len()
            {
                self.select_profile(self.selected + 1);
            }
            if button(
                ui,
                rr(15.0, 589.0, 155.0, 608.0),
                "btn_clear2",
                "Стереть таблицу",
            ) {
                self.clear_table();
            }

            text(
                ui,
                px(38.0),
                py(615.0),
                "Качество изображения",
                11.0,
                Color32::from_gray(40),
            );

            // ---- верхняя карта ----
            centered(
                ui,
                r.min.x + r.width() * 0.5,
                py(50.0),
                "Построение структурной карты отражающего горизонта",
                14.0,
                Color32::from_gray(25),
            );
            text(
                ui,
                px(181.0),
                py(68.0),
                "Структурная карта",
                12.0,
                Color32::from_gray(40),
            );
            text(
                ui,
                px(160.0),
                py(64.0),
                "Y, м",
                11.0,
                Color32::from_gray(40),
            );
            text(
                ui,
                px(702.0),
                py(64.0),
                "X, м",
                11.0,
                Color32::from_gray(40),
            );

            let map_rect = rr(180.0, MAP_TOP, 880.0, MAP_BOTTOM);
            ui.painter()
                .rect_filled(map_rect, 0.0, scale_color(0.0, theme));
            ui.painter()
                .rect_stroke(map_rect, 0.0, Stroke::new(1.0_f32, Color32::from_gray(90)));
            if let Some(tex) = &self.texture {
                ui.painter().image(
                    tex.id(),
                    map_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            // Сетка original: 8 вертикальных делений и 10 горизонтальных,
            // шаг по Y = 349/10 ≈ 35 px (совпадает с пиксельным разбором).
            for i in 0..=8 {
                let x = map_rect.min.x + map_rect.width() * i as f32 / 8.0;
                ui.painter().line_segment(
                    [Pos2::new(x, map_rect.min.y), Pos2::new(x, map_rect.max.y)],
                    Stroke::new(0.7_f32, Color32::from_gray(110)),
                );
            }
            for i in 0..=10 {
                let y = map_rect.min.y + map_rect.height() * i as f32 / 10.0;
                ui.painter().line_segment(
                    [Pos2::new(map_rect.min.x, y), Pos2::new(map_rect.max.x, y)],
                    Stroke::new(0.7_f32, Color32::from_gray(110)),
                );
            }
            // Подписи оси X — через деление, чтобы не засорять карту.
            for i in [2usize, 4, 6, 8] {
                let x = map_rect.min.x + map_rect.width() * i as f32 / 8.0;
                centered(
                    ui,
                    x,
                    map_rect.min.y + 2.0,
                    format!("{}", i * 1000),
                    8.0,
                    Color32::from_gray(45),
                );
            }
            for (i, p) in self.profiles.iter().enumerate() {
                let a = Pos2::new(
                    px(180.0 + p.x0 as f32 / MAP_WIDTH_M as f32 * 700.0),
                    py(MAP_TOP + p.y0 as f32 / MAP_HEIGHT_M as f32 * (MAP_BOTTOM - MAP_TOP)),
                );
                let b = Pos2::new(
                    px(180.0 + p.x1 as f32 / MAP_WIDTH_M as f32 * 700.0),
                    py(MAP_TOP + p.y1 as f32 / MAP_HEIGHT_M as f32 * (MAP_BOTTOM - MAP_TOP)),
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
                    .line_segment([a, b], Stroke::new(2.0_f32, Color32::from_rgb(255, 255, 0)));
            }
            let mresp = ui.interact(map_rect, egui::Id::new("map_area"), Sense::click_and_drag());
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
                        let x0 = (a.x - map_rect.min.x) / map_rect.width() * MAP_WIDTH_M as f32;
                        let y0 = (a.y - map_rect.min.y) / map_rect.height() * MAP_HEIGHT_M as f32;
                        let x1 = (b.x - map_rect.min.x) / map_rect.width() * MAP_WIDTH_M as f32;
                        let y1 = (b.y - map_rect.min.y) / map_rect.height() * MAP_HEIGHT_M as f32;
                        self.add_profile(x0, y0, x1, y1);
                    }
                }
                self.drag_start = None;
                self.drag_cur = None;
            }

            // ---- цветовая шкала + подписи H min / H max ----
            color_legend(ui, &rr, theme);
            text(
                ui,
                px(400.0),
                py(441.0),
                "H min",
                10.0,
                Color32::from_gray(50),
            );
            text(
                ui,
                px(660.0),
                py(441.0),
                "H max",
                10.0,
                Color32::from_gray(50),
            );

            // ---- временной разрез ----
            text(
                ui,
                px(181.0),
                py(441.0),
                "Временной разрез",
                13.0,
                Color32::from_gray(30),
            );
            text(
                ui,
                px(840.0),
                py(441.0),
                "трассы",
                11.0,
                Color32::from_gray(40),
            );
            let sec = rr(180.0, SEC_TOP, 880.0, SEC_BOTTOM);
            ui.painter()
                .rect_filled(sec, 0.0, Color32::from_rgb(253, 253, 252));
            ui.painter()
                .rect_stroke(sec, 0.0, Stroke::new(1.0_f32, Color32::from_gray(140)));
            text(
                ui,
                px(184.0),
                py(472.0),
                "t, с",
                10.0,
                Color32::from_gray(70),
            );
            for i in 1..=10 {
                let x = sec.min.x + sec.width() * i as f32 / 11.0;
                ui.painter().line_segment(
                    [Pos2::new(x, sec.min.y), Pos2::new(x, sec.max.y)],
                    Stroke::new(0.6_f32, Color32::from_gray(210)),
                );
                centered(
                    ui,
                    x,
                    sec.min.y + 2.0,
                    i.to_string(),
                    10.0,
                    Color32::from_gray(60),
                );
            }
            // 10 делений по времени — как в original (шаг ≈17 px).
            for i in 0..=10 {
                let y = sec.min.y + sec.height() * i as f32 / 10.0;
                ui.painter().line_segment(
                    [Pos2::new(sec.min.x, y), Pos2::new(sec.max.x, y)],
                    Stroke::new(0.6_f32, Color32::from_gray(210)),
                );
                text(
                    ui,
                    sec.min.x + 2.0,
                    y + 1.0,
                    format!("{:.2}", 3.6 - 0.04 * i as f64),
                    8.0,
                    Color32::from_gray(130),
                );
            }
            let picks: Vec<Pick> = self
                .profiles
                .get(self.selected)
                .map(|p| p.picks.clone())
                .unwrap_or_default();
            // Виггл-трассы вместо одной ломаной — как на оригинальном разрезе.
            for i in 1..=10 {
                if let Some(pk) = picks.iter().min_by(|a, b| {
                    ((a.trace - i as f64).abs())
                        .partial_cmp(&((b.trace - i as f64).abs()))
                        .unwrap_or(std::cmp::Ordering::Equal)
                }) {
                    if (pk.trace - i as f64).abs() <= 1.5 {
                        let cx = sec.min.x + sec.width() * i as f32 / 11.0;
                        wiggle(ui, cx, sec, pk.time);
                    }
                }
            }
            for p in &picks {
                let c = Pos2::new(
                    sec.min.x + (p.trace / 11.0) as f32 * sec.width(),
                    sec.max.y - ((p.time - 3.2) / 0.4).clamp(0.0, 1.0) as f32 * sec.height(),
                );
                ui.painter()
                    .circle_filled(c, 3.0, Color32::from_rgb(220, 70, 30));
            }
            let sresp = ui.interact(sec, egui::Id::new("sec_area"), Sense::click());
            if sresp.clicked() {
                let p = sresp.interact_pointer_pos().unwrap_or(sec.min);
                let trace = ((p.x - sec.min.x) / sec.width() * 11.0) as f64;
                let time = (3.2 + (1.0 - (p.y - sec.min.y) / sec.height()) * 0.4) as f64;
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

            text(
                ui,
                px(700.0),
                py(635.0),
                "Rust version Fold by Lion 2026",
                11.0,
                Color32::from_gray(90),
            );
            text(
                ui,
                px(17.0),
                py(640.0),
                &self.status,
                9.0,
                Color32::from_gray(120),
            );

            if self.show_about {
                egui::Window::new("О программе")
                    .open(&mut self.show_about)
                    .show(ctx, |ui| {
                        ui.label("Fold — структурная карта отражающего горизонта");
                        ui.label(format!("Расчёт: {:.3} мс", self.last_ms));
                        ui.label("Rust version Fold by Lion 2026");
                    });
            }

            if self.show_help {
                egui::Window::new("Справка")
                    .open(&mut self.show_help)
                    .default_size([430.0, 300.0])
                    .show(ctx, |ui| {
                        ui.label("Fold — быстрый порт программы 2007 года.");
                        ui.separator();
                        ui.label("V ср — средняя скорость, f — параметр сигнала.");
                        ui.label("Профиль задаётся перетаскиванием мыши по карте.");
                        ui.label("Пик на временном разрезе ставится кликом мыши.");
                        ui.label("Назад / Далее — листание профилей.");
                        ui.label("Строка — экспорт и импорт готовой строки данных.");
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
                        ui.label(
                            "Скопируйте готовую строку или вставьте свою и нажмите «Применить».",
                        );
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
                                        let vel = format!("{:.0}", settings.velocity);
                                        let qual = format!("{:.0}", settings.quality);
                                        self.profiles = profiles;
                                        self.settings = settings;
                                        self.selected = 0;
                                        self.v = vel;
                                        self.fval = qual;
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
