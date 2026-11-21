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
use crate::model::{Pick, Profile, Settings, MAP_HEIGHT_M, MAP_WIDTH_M};
use egui::{Color32, FontId, Pos2, Rect, Sense, Stroke, TextureHandle};

const W: f32 = 894.0;
const H: f32 = 656.0;
const LEFT: f32 = 180.0;
const MAP_TOP: f32 = 83.0;
const MAP_BOTTOM: f32 = 432.0;
const SEC_TOP: f32 = 456.0;
const SEC_BOTTOM: f32 = 613.0;

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
                Pick { trace: 1.0, time: 3.465 },
                Pick { trace: 3.0, time: 3.474 },
                Pick { trace: 5.0, time: 3.290 },
                Pick { trace: 7.0, time: 3.524 },
                Pick { trace: 10.0, time: 3.520 },
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
            status: "Построение структурной карты отражающего горизонта".into(),
            drag_start: None,
            drag_cur: None,
            show_about: false,
        }
    }

    fn recalc(&mut self, ctx: &egui::Context) {
        let t = std::time::Instant::now();
        self.grid = Some(build_map(&self.profiles, &self.settings));
        self.last_ms = t.elapsed().as_secs_f64() * 1000.0;
        if let Some(g) = &self.grid {
            let mut img = egui::ColorImage::new([g.width, g.height], Color32::from_rgb(20, 40, 60));
            for (i, &d) in g.data.iter().enumerate() {
                if d.is_finite() {
                    img.pixels[i] = palette(d, g);
                }
            }
            self.texture = Some(ctx.load_texture("struct_map", img, egui::TextureOptions::LINEAR));
        }
        self.dirty = false;
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
                Pick { trace: 1.0, time: 3.465 },
                Pick { trace: 5.0, time: 3.520 },
                Pick { trace: 10.0, time: 3.500 },
            ],
        });
        self.selected = id;
        self.xg = format!("{:.0}", x1);
        self.yg = format!("{:.0}", y);
        let _ = y;
        self.dirty = true;
    }

    fn clear_table(&mut self) {
        if let Some(p) = self.profiles.get_mut(self.selected) {
            p.picks.clear();
            self.dirty = true;
        }
    }
}

fn palette(d: f32, g: &MapGrid) -> Color32 {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &v in &g.data {
        if v.is_finite() {
            min = min.min(v);
            max = max.max(v);
        }
    }
    if !min.is_finite() {
        min = 0.0;
        max = 1.0;
    }
    if (max - min).abs() < 1e-6 {
        max = min + 1.0;
    }
    let u = ((d - min) / (max - min)).clamp(0.0, 1.0);
    // оттенки оригинала: cyan -> green -> yellow -> orange
    let stops: [(f32, [u8; 3]); 4] = [
        (0.0, [82, 203, 183]),
        (0.35, [80, 203, 102]),
        (0.7, [185, 203, 62]),
        (1.0, [230, 150, 60]),
    ];
    for w in stops.windows(2) {
        if u <= w[1].0 {
            let t = (u - w[0].0) / (w[1].0 - w[0].0);
            let c = [0, 1, 2].map(|i| (w[0].1[i] as f32 + (w[1].1[i] as f32 - w[0].1[i] as f32) * t) as u8);
            return Color32::from_rgb(c[0], c[1], c[2]);
        }
    }
    Color32::from_rgb(230, 150, 60)
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
    ui.put(rect, egui::TextEdit::singleline(val).id(egui::Id::new(id)).frame(false).font(FontId::proportional(11.0))).changed()
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
            ui.painter().line_segment([Pos2::new(r.min.x, menu.max.y), Pos2::new(r.max.x, menu.max.y)], Stroke::new(1.0_f32, Color32::from_gray(180)));
            let ma = ui.interact(menu, egui::Id::new("menu"), Sense::click());
            text(ui, px(12.0), py(27.0), "Структура", 12.0, Color32::BLACK);
            text(ui, px(76.0), py(27.0), "Профили", 12.0, Color32::BLACK);
            text(ui, px(135.0), py(27.0), "Стереть таблицу", 12.0, Color32::BLACK);
            if ma.clicked() {
                if let Some(p) = ma.interact_pointer_pos() {
                    let x = (p.x - r.min.x) / sx;
                    if x > 130.0 && x < 215.0 {
                        self.clear_table();
                    }
                }
            }

            // ---- левый столбец ----
            let left = rr(0.0, 48.0, LEFT, H);
            ui.painter().rect_filled(left, 0.0, Color32::from_gray(240));
            ui.painter().line_segment([Pos2::new(left.max.x, r.min.y), Pos2::new(left.max.x, r.max.y)], Stroke::new(1.0_f32, Color32::from_gray(180)));

            text(ui, px(17.0), py(85.0), "Параметры структуры", 12.0, Color32::from_gray(30));
            text(ui, px(17.0), py(106.0), "V ср", 12.0, Color32::BLACK);
            if edit(ui, rr(70.0, 104.0, 120.0, 120.0), &mut self.v, "v_field") {
                if let Ok(x) = self.v.parse() { self.settings.velocity = x; self.dirty = true; }
            }
            text(ui, px(17.0), py(136.0), "f", 12.0, Color32::BLACK);
            edit(ui, rr(70.0, 134.0, 120.0, 150.0), &mut self.fval, "f_field");

            text(ui, px(17.0), py(169.0), "Координаты профиля", 12.0, Color32::from_gray(30));
            text(ui, px(35.0), py(190.0), format!("Профиль {}", self.selected + 1), 12.0, Color32::BLACK);
            text(ui, px(25.0), py(250.0), "X", 12.0, Color32::BLACK);
            if edit(ui, rr(70.0, 248.0, 130.0, 264.0), &mut self.xg, "xg_field") { self.dirty = true; }
            text(ui, px(25.0), py(275.0), "Y", 12.0, Color32::BLACK);
            if edit(ui, rr(70.0, 273.0, 130.0, 289.0), &mut self.yg, "yg_field") { self.dirty = true; }

            let by = 305.0;
            ui.horizontal(|ui| {
                if ui.put(rr(15.0, by, 82.0, by + 18.0), egui::Button::new("Назад")).clicked() && self.selected > 0 {
                    self.selected -= 1;
                }
                if ui.put(rr(88.0, by, 155.0, by + 18.0), egui::Button::new("Далее")).clicked() && self.selected + 1 < self.profiles.len() {
                    self.selected += 1;
                }
            });

            text(ui, px(17.0), py(330.0), "Таблица корреляции", 12.0, Color32::from_gray(30));
            if let Some(p) = self.profiles.get(self.selected) {
                for (i, pk) in p.picks.iter().take(15).enumerate() {
                    text(ui, px(39.0), py(350.0 + i as f32 * 14.0), format!("t{} = {:.3} с", i, pk.time), 11.0, Color32::from_gray(40));
                }
            }
            text(ui, px(38.0), py(612.0), "Качество изображения", 11.0, Color32::from_gray(40));

            // ---- верхняя карта ----
            centered(ui, r.min.x + r.width() * 0.5, py(48.0), "Построение структурной карты отражающего горизонта", 14.0, Color32::from_gray(25));
            text(ui, px(181.0), py(66.0), "Структурная карта", 12.0, Color32::from_gray(40));
            text(ui, px(700.0), py(62.0), "X, м", 11.0, Color32::from_gray(40));

            let map_rect = rr(180.0, MAP_TOP, 880.0, MAP_BOTTOM);
            ui.painter().rect_filled(map_rect, 0.0, Color32::from_rgb(235, 242, 245));
            if let Some(tex) = &self.texture {
                ui.painter().image(tex.id(), map_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            }
            for i in 0..=8 {
                let x = map_rect.min.x + map_rect.width() * i as f32 / 8.0;
                ui.painter().line_segment([Pos2::new(x, map_rect.min.y), Pos2::new(x, map_rect.max.y)], Stroke::new(0.7_f32, Color32::from_gray(110)));
            }
            for i in 0..=6 {
                let y = map_rect.min.y + map_rect.height() * i as f32 / 6.0;
                ui.painter().line_segment([Pos2::new(map_rect.min.x, y), Pos2::new(map_rect.max.x, y)], Stroke::new(0.7_f32, Color32::from_gray(110)));
            }
            for (i, p) in self.profiles.iter().enumerate() {
                let a = Pos2::new(px(180.0 + p.x0 as f32 / MAP_WIDTH_M as f32 * 700.0), py(MAP_TOP + p.y0 as f32 / MAP_HEIGHT_M as f32 * (MAP_BOTTOM - MAP_TOP)));
                let b = Pos2::new(px(180.0 + p.x1 as f32 / MAP_WIDTH_M as f32 * 700.0), py(MAP_TOP + p.y1 as f32 / MAP_HEIGHT_M as f32 * (MAP_BOTTOM - MAP_TOP)));
                let col = if i == self.selected { Color32::from_rgb(255, 60, 40) } else { Color32::from_rgb(30, 80, 200) };
                ui.painter().line_segment([a, b], Stroke::new(if i == self.selected { 2.5 } else { 1.5_f32 }, col));
                ui.painter().circle_filled(a, 3.0, col);
                ui.painter().circle_filled(b, 3.0, col);
            }
            if let (Some(a), Some(b)) = (self.drag_start, self.drag_cur) {
                ui.painter().line_segment([a, b], Stroke::new(2.0_f32, Color32::from_rgb(255, 255, 0)));
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

            // ---- временной разрез ----
            text(ui, px(181.0), py(438.0), "Временной разрез", 13.0, Color32::from_gray(30));
            text(ui, px(845.0), py(443.0), "трассы", 11.0, Color32::from_gray(40));
            let sec = rr(180.0, SEC_TOP, 880.0, SEC_BOTTOM);
            ui.painter().rect_filled(sec, 0.0, Color32::from_gray(252));
            ui.painter().rect_stroke(sec, 0.0, Stroke::new(1.0_f32, Color32::from_gray(150)));
            for i in 1..=10 {
                let x = sec.min.x + sec.width() * i as f32 / 11.0;
                ui.painter().line_segment([Pos2::new(x, sec.min.y), Pos2::new(x, sec.max.y)], Stroke::new(0.6_f32, Color32::from_gray(210)));
                centered(ui, x, sec.max.y + 2.0, i.to_string(), 10.0, Color32::from_gray(60));
            }
            for i in 0..=4 {
                let y = sec.min.y + sec.height() * i as f32 / 4.0;
                ui.painter().line_segment([Pos2::new(sec.min.x, y), Pos2::new(sec.max.x, y)], Stroke::new(0.6_f32, Color32::from_gray(210)));
            }
            let picks: Vec<Pick> = self.profiles.get(self.selected).map(|p| p.picks.clone()).unwrap_or_default();
            if picks.len() > 1 {
                let pts: Vec<Pos2> = picks.iter().map(|p| {
                    Pos2::new(
                        sec.min.x + (p.trace / 11.0) as f32 * sec.width(),
                        sec.max.y - ((p.time - 3.2) / 0.4).clamp(0.0, 1.0) as f32 * sec.height(),
                    )
                }).collect();
                ui.painter().add(egui::Shape::line(pts, Stroke::new(2.0_f32, Color32::from_rgb(220, 70, 30))));
            }
            for p in &picks {
                let c = Pos2::new(
                    sec.min.x + (p.trace / 11.0) as f32 * sec.width(),
                    sec.max.y - ((p.time - 3.2) / 0.4).clamp(0.0, 1.0) as f32 * sec.height(),
                );
                ui.painter().circle_filled(c, 3.0, Color32::from_rgb(220, 70, 30));
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
                    pr.picks.sort_by(|a, b| a.trace.partial_cmp(&b.trace).unwrap_or(std::cmp::Ordering::Equal));
                    self.dirty = true;
                    self.status = format!("Пик: трасса {:.1}, t {:.3} с", trace, time);
                }
            }

            text(ui, px(772.0), py(635.0), "Viktor Romanov, 2007", 11.0, Color32::from_gray(90));
            text(ui, px(17.0), py(640.0), &self.status, 9.0, Color32::from_gray(120));

            if self.show_about {
                egui::Window::new("О программе").open(&mut self.show_about).show(ctx, |ui| {
                    ui.label("Fold — структурная карта отражающего горизонта");
                    ui.label(format!("Расчёт: {:.3} мс", self.last_ms));
                });
            }
        });
    }
}
