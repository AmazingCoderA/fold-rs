//! GUI-клон оригинального окна Fold 1:1 (894x656).
//!
//! Раскладка восстановлена по скриншоту:
//!   - строка меню: Структура / Профили / Стереть таблицу
//!   - левый столбец: Параметры структуры, V ср, f, Координаты профиля,
//!     Профиль №, Таблица корреляции, Качество изображения
//!   - верхняя область: "Построение структурной карты отражающего горизонта",
//!     цветная структурная карта с сеткой
//!   - нижняя область: "Временной разрез" с осью трасс 1..10

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

/// Цветовая шкала структурной карты из эталонного скриншота:
/// x=440..653, y=436..453, 21 полоса, синий → голубой → зелёный → жёлтый → красный.
const LEGEND_X0: f32 = 440.0;
const LEGEND_X1: f32 = 653.0;
const LEGEND_Y0: f32 = 436.0;
const LEGEND_Y1: f32 = 453.0;
const BANDS: usize = 21;

/// Точные RGB-стопы шкалы, снятые пиксель-в-пиксель со скриншота.
const SCALE_STOPS: [(f32, [u8; 3]); 5] = [
    (0.0, [70, 96, 205]),
    (0.25, [82, 204, 188]),
    (0.50, [82, 203, 80]),
    (0.75, [202, 202, 63]),
    (1.0, [198, 80, 64]),
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
    show_io: bool,
    io_text: String,
    io_error: String,
    /// Ключ, при котором текстура уже пересчитана: (угол, ширина, высота).
    /// Пока ключ не изменился — карту не трогаем вообще (не теряем кадры).
    map_key: Option<(f64, usize, usize)>,
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
            show_io: false,
            io_text: String::new(),
            io_error: String::new(),
            map_key: None,
        }
    }

    fn recalc(&mut self, ctx: &egui::Context) {
        // Карта зависит только от угла и размеров сетки. Скорость, качество,
        // координаты профиля и пики на неё не влияют — не пересчитываем зря.
        let key = (
            self.settings.angle,
            self.settings.width,
            self.settings.height,
        );
        self.dirty = false;
        if self.map_key == Some(key) && self.texture.is_some() {
            return;
        }

        let t = std::time::Instant::now();
        let g = build_map(&self.profiles, &self.settings);
        let (min, max) = bounds(&g);
        let span = (max - min).max(1e-6);
        let mut img = egui::ColorImage::new([g.width, g.height], Color32::from_rgb(70, 96, 205));
        for (i, &d) in g.data.iter().enumerate() {
            if d.is_finite() {
                img.pixels[i] = palette(((d - min) / span).clamp(0.0, 1.0));
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
fn palette(u: f32) -> Color32 {
    let band = (u * BANDS as f32) as usize;
    let t = ((band.min(BANDS - 1) as f32 + 0.5) / BANDS as f32).clamp(0.0, 1.0);
    scale_color(t)
}

/// Интерполяция эталонных стопов цветовой шкалы.
fn scale_color(t: f32) -> Color32 {
    for w in SCALE_STOPS.windows(2) {
        if t <= w[1].0 {
            let k = (t - w[0].0) / (w[1].0 - w[0].0);
            let c = [0, 1, 2].map(|i| {
                (w[0].1[i] as f32 + (w[1].1[i] as f32 - w[0].1[i] as f32) * k).round() as u8
            });
            return Color32::from_rgb(c[0], c[1], c[2]);
        }
    }
    let c = SCALE_STOPS[SCALE_STOPS.len() - 1].1;
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Сейсмическая трасса «виггл»: чёрная волна вокруг своей оси с пиком на времени pick.
fn wiggle(ui: &egui::Ui, cx: f32, sec: Rect, pick_time: f64) {
    const N: usize = 72;
    let mut pts = Vec::with_capacity(N + 1);
    for k in 0..=N {
        let fy = k as f32 / N as f32;
        let y = sec.min.y + fy * sec.height();
        let t = 3.2_f64 + (1.0 - f64::from(fy)) * 0.4;
        let dt = t - pick_time;
        let env = (-dt * dt / 0.00045).exp() as f32;
        let wave = (dt * 240.0).sin() as f32;
        pts.push(Pos2::new(cx + env * wave * 12.0, y));
    }
    ui.painter().add(egui::Shape::line(
        pts,
        Stroke::new(1.0_f32, Color32::from_gray(35)),
    ));
}

/// Рисует легенду-шкалу внизу: 21 полоса, подписи и тонкая рамка.
fn color_legend(ui: &egui::Ui, rr: impl Fn(f32, f32, f32, f32) -> Rect) {
    let bar = rr(LEGEND_X0, LEGEND_Y0, LEGEND_X1, LEGEND_Y1);
    let bw = bar.width() / BANDS as f32;
    for i in 0..BANDS {
        let cell = Rect::from_min_max(
            Pos2::new(bar.min.x + i as f32 * bw, bar.min.y),
            Pos2::new(bar.min.x + (i + 1) as f32 * bw, bar.max.y),
        );
        ui.painter()
            .rect_filled(cell, 0.0, scale_color((i as f32 + 0.5) / BANDS as f32));
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

fn edit(ui: &mut egui::Ui, rect: Rect, val: &mut String, id: &'static str) -> bool {
    ui.put(
        rect,
        egui::TextEdit::singleline(val)
            .id(egui::Id::new(id))
            .frame(false)
            .font(FontId::proportional(11.0)),
    )
    .changed()
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

fn vgrad(ui: &egui::Ui, r: Rect, top: Color32, bottom: Color32) {
    const N: usize = 6;
    for i in 0..N {
        let y0 = r.min.y + r.height() * i as f32 / N as f32;
        let y1 = r.min.y + r.height() * (i + 1) as f32 / N as f32;
        let c = lerp_color(top, bottom, i as f32 / (N - 1) as f32);
        ui.painter().rect_filled(
            Rect::from_min_max(Pos2::new(r.min.x, y0), Pos2::new(r.max.x, y1)),
            0.0,
            c,
        );
    }
}

/// Кнопка в стиле Delphi 2007: мягкий вертикальный градиент, тонкий бевель,
/// никакой кричащей подсветки. Рисуется painter'ом — не крадёт фокус и не
/// выбивается из общего плоского интерфейса.
fn button(ui: &mut egui::Ui, rect: Rect, id: &'static str, label: &str) -> bool {
    let resp = ui.interact(rect, egui::Id::new(id), Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    let (top, bottom, border) = if pressed {
        (
            Color32::from_gray(206),
            Color32::from_gray(228),
            Color32::from_gray(90),
        )
    } else if resp.hovered() {
        (
            Color32::from_gray(248),
            Color32::from_gray(229),
            Color32::from_gray(105),
        )
    } else {
        (
            Color32::from_gray(240),
            Color32::from_gray(219),
            Color32::from_gray(132),
        )
    };

    let inner = Rect::from_min_max(
        Pos2::new(rect.min.x + 1.0, rect.min.y + 1.0),
        Pos2::new(rect.max.x - 1.0, rect.max.y - 1.0),
    );
    vgrad(ui, inner, top, bottom);
    // верх/лево — светлый блик, низ/право — тень
    ui.painter().line_segment(
        [
            Pos2::new(rect.min.x + 1.0, rect.min.y + 1.0),
            Pos2::new(rect.max.x - 1.0, rect.min.y + 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_gray(252)),
    );
    ui.painter().line_segment(
        [
            Pos2::new(rect.min.x + 1.0, rect.min.y + 1.0),
            Pos2::new(rect.min.x + 1.0, rect.max.y - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_gray(250)),
    );
    ui.painter().line_segment(
        [
            Pos2::new(rect.min.x + 1.0, rect.max.y - 1.0),
            Pos2::new(rect.max.x - 1.0, rect.max.y - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_gray(170)),
    );
    ui.painter().line_segment(
        [
            Pos2::new(rect.max.x - 1.0, rect.min.y + 1.0),
            Pos2::new(rect.max.x - 1.0, rect.max.y - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_gray(170)),
    );
    ui.painter()
        .rect_stroke(rect, 0.0, Stroke::new(1.0_f32, border));
    ui.painter().text(
        rect.center() - egui::vec2(0.0, 0.5),
        egui::Align2::CENTER_CENTER,
        label,
        FontId::proportional(11.0),
        Color32::from_gray(25),
    );
    resp.clicked()
}

impl eframe::App for FoldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dirty {
            self.recalc(ctx);
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

            ui.painter().rect_filled(r, 0.0, Color32::from_gray(245));

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

            // ---- левый столбец ----
            let left = rr(0.0, 48.0, LEFT, H);
            ui.painter().rect_filled(left, 0.0, Color32::from_gray(240));
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
            if let Some(p) = self.profiles.get(self.selected) {
                for (i, pk) in p.picks.iter().take(15).enumerate() {
                    text(
                        ui,
                        px(39.0),
                        py(333.0 + i as f32 * 14.0),
                        format!("t{} = {:.3} с", i, pk.time),
                        11.0,
                        Color32::from_gray(40),
                    );
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
                px(702.0),
                py(64.0),
                "X, м",
                11.0,
                Color32::from_gray(40),
            );

            let map_rect = rr(180.0, MAP_TOP, 880.0, MAP_BOTTOM);
            ui.painter()
                .rect_filled(map_rect, 0.0, Color32::from_rgb(235, 242, 245));
            if let Some(tex) = &self.texture {
                ui.painter().image(
                    tex.id(),
                    map_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            for i in 0..=8 {
                let x = map_rect.min.x + map_rect.width() * i as f32 / 8.0;
                ui.painter().line_segment(
                    [Pos2::new(x, map_rect.min.y), Pos2::new(x, map_rect.max.y)],
                    Stroke::new(0.7_f32, Color32::from_gray(110)),
                );
            }
            for i in 0..=6 {
                let y = map_rect.min.y + map_rect.height() * i as f32 / 6.0;
                ui.painter().line_segment(
                    [Pos2::new(map_rect.min.x, y), Pos2::new(map_rect.max.x, y)],
                    Stroke::new(0.7_f32, Color32::from_gray(110)),
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

            // ---- цветовая шкала (была потеряна в предыдущей версии) ----
            color_legend(ui, &rr);

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
                px(800.0),
                py(441.0),
                "трассы",
                11.0,
                Color32::from_gray(40),
            );
            let sec = rr(180.0, SEC_TOP, 880.0, SEC_BOTTOM);
            ui.painter().rect_filled(sec, 0.0, Color32::from_gray(252));
            ui.painter()
                .rect_stroke(sec, 0.0, Stroke::new(1.0_f32, Color32::from_gray(150)));
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
            for i in 0..=4 {
                let y = sec.min.y + sec.height() * i as f32 / 4.0;
                ui.painter().line_segment(
                    [Pos2::new(sec.min.x, y), Pos2::new(sec.max.x, y)],
                    Stroke::new(0.6_f32, Color32::from_gray(210)),
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
