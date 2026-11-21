//! GUI-клон оригинальной формы Fold (Delphi/VCL → egui).
//!
//! Сохранены: заголовок, вкладки, панели «Координаты профиля»,
//! «Параметры структуры», «Таблица корреляции», временной разрез,
//! кнопки «Ввести вручную / Удалить все / Стереть таблицу / Назад / Далее»
//! и значения по умолчанию из DFM (3, 1500, 95, 0, 0).

use crate::grid::{build_map, MapGrid};
use crate::model::{Pick, Profile, Settings, MAP_HEIGHT_M, MAP_WIDTH_M};
use egui::{Color32, Pos2, Rect, Sense, Stroke, TextureHandle, Vec2};

pub struct FoldApp {
    profiles: Vec<Profile>,
    settings: Settings,
    selected: usize,
    grid: Option<MapGrid>,
    texture: Option<TextureHandle>,
    last_ms: f64,

    // поля ввода «Координаты профиля»
    x0: String,
    y0: String,
    x1: String,
    y1: String,
    v: String,
    q: String,
    f: String,

    drag_start: Option<Pos2>,
    drag_cur: Option<Pos2>,
    status: String,
    dirty: bool,
    show_about: bool,
}

impl FoldApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            profiles: Vec::new(),
            settings: Settings::default(),
            selected: 0,
            grid: None,
            texture: None,
            last_ms: 0.0,
            x0: "0".into(),
            y0: "0".into(),
            x1: "8000".into(),
            y1: "4000".into(),
            v: "1500".into(),
            q: "95".into(),
            f: "3".into(),
            drag_start: None,
            drag_cur: None,
            status: "Готово. Нарисуйте профиль на структурной карте.".into(),
            dirty: true,
            show_about: false,
        }
    }

    fn recalc(&mut self, ctx: &egui::Context) {
        let t = std::time::Instant::now();
        self.grid = Some(build_map(&self.profiles, &self.settings));
        self.last_ms = t.elapsed().as_secs_f64() * 1000.0;
        if let Some(g) = &self.grid {
            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            for &x in &g.data {
                if x.is_finite() {
                    min = min.min(x);
                    max = max.max(x);
                }
            }
            if !min.is_finite() {
                min = 0.0;
                max = 1.0;
            }
            if (max - min).abs() < 1e-6 {
                max = min + 1.0;
            }
            let mut img = egui::ColorImage::new([g.width, g.height], Color32::from_gray(30));
            for (i, &d) in g.data.iter().enumerate() {
                if d.is_finite() {
                    let u = ((d - min) / (max - min)).clamp(0.0, 1.0);
                    let c = (37.0 + 200.0 * u) as u8;
                    img.pixels[i] = Color32::from_rgb(c, c, (c as u16 + 30).min(255) as u8);
                }
            }
            self.texture = Some(ctx.load_texture(
                "struct_map",
                img,
                egui::TextureOptions::NEAREST,
            ));
        }
        self.dirty = false;
    }

    fn map_to_screen(p: Pos2, rect: Rect) -> Pos2 {
        Pos2::new(
            rect.min.x + (p.x / MAP_WIDTH_M as f32) * rect.width(),
            rect.min.y + (p.y / MAP_HEIGHT_M as f32) * rect.height(),
        )
    }

    fn screen_to_map(p: Pos2, rect: Rect) -> Pos2 {
        Pos2::new(
            ((p.x - rect.min.x) / rect.width() * MAP_WIDTH_M as f32).clamp(0.0, MAP_WIDTH_M as f32),
            ((p.y - rect.min.y) / rect.height() * MAP_HEIGHT_M as f32).clamp(0.0, MAP_HEIGHT_M as f32),
        )
    }

    fn add_profile(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let id = self.profiles.len();
        self.profiles.push(Profile {
            id,
            x0,
            y0,
            x1,
            y1,
            picks: vec![
                Pick { trace: 0.0, time: 1.0 },
                Pick { trace: 60.0, time: 1.1 },
                Pick { trace: 120.0, time: 0.95 },
            ],
        });
        self.selected = id;
        self.dirty = true;
        self.status = format!("Профиль №{} добавлен", id + 1);
    }

    fn save_pgm(&self) {
        if let Some(g) = &self.grid {
            let _ = crate::io::to_pgm(g, "map.pgm");
        }
    }

    fn menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::menu::menu_button(ui, "Файл", |ui| {
                    if ui.button("Открыть...").clicked() {
                        self.status = "Откройте файл через CLI: fold <file.txt>".into();
                    }
                    if ui.button("Сохранить карту (map.pgm)").clicked() {
                        self.save_pgm();
                    }
                    ui.separator();
                    if ui.button("Выход").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                egui::menu::menu_button(ui, "Профили", |ui| {
                    if ui.button("Ввести вручную").clicked() {
                        self.manual_add();
                    }
                    if ui.button("Удалить все").clicked() {
                        self.profiles.clear();
                        self.selected = 0;
                        self.dirty = true;
                        self.status = "Все профили удалены".into();
                    }
                    if ui.button("Стереть таблицу").clicked() {
                        if let Some(p) = self.profiles.get_mut(self.selected) {
                            p.picks.clear();
                            self.dirty = true;
                            self.status = "Таблица корреляции очищена".into();
                        }
                    }
                });
                egui::menu::menu_button(ui, "Справка", |ui| {
                    if ui.button("О программе").clicked() {
                        self.show_about = true;
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Fold — по Viktor Romanov, 2007 (Rust port)");
                });
            });
        });
    }

    fn manual_add(&mut self) {
        let nums: Vec<Option<f64>> = [&self.x0, &self.y0, &self.x1, &self.y1]
            .iter()
            .map(|s| s.parse().ok())
            .collect();
        if nums.iter().any(Option::is_none) {
            self.status = "Ошибка в координатах профиля".into();
            return;
        }
        let (x0, y0, x1, y1) = (
            nums[0].unwrap(),
            nums[1].unwrap(),
            nums[2].unwrap(),
            nums[3].unwrap(),
        );
        self.add_profile(x0, y0, x1, y1);
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("coords").min_width(210.0).show(ctx, |ui| {
            ui.heading("Координаты профиля");
            ui.separator();
            ui.label("X, м");
            ui.text_edit_singleline(&mut self.x0);
            ui.label("Y, м");
            ui.text_edit_singleline(&mut self.y0);
            ui.label("X1, м");
            ui.text_edit_singleline(&mut self.x1);
            ui.label("Y1, м");
            ui.text_edit_singleline(&mut self.y1);
            ui.separator();
            if ui.button("Ввести вручную").clicked() {
                self.manual_add();
            }
            if ui.button("Удалить все").clicked() {
                self.profiles.clear();
                self.selected = 0;
                self.dirty = true;
                self.status = "Все профили удалены".into();
            }
            if ui.button("Стереть таблицу").clicked() {
                if let Some(p) = self.profiles.get_mut(self.selected) {
                    p.picks.clear();
                    self.dirty = true;
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Назад").clicked() && self.selected > 0 {
                    self.selected -= 1;
                }
                ui.label(format!("Профиль №{}", self.selected + 1));
                if ui.button("Далее").clicked() && self.selected + 1 < self.profiles.len() {
                    self.selected += 1;
                }
            });
            ui.separator();
            ui.label("V ср");
            if ui.text_edit_singleline(&mut self.v).changed() {
                if let Ok(x) = self.v.parse() {
                    self.settings.velocity = x;
                    self.dirty = true;
                }
            }
            ui.label("Качество изображения");
            if ui.text_edit_singleline(&mut self.q).changed() {
                if let Ok(x) = self.q.parse() {
                    self.settings.quality = x;
                    self.dirty = true;
                }
            }
            ui.label("f");
            ui.text_edit_singleline(&mut self.f);
        });
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("params").min_width(230.0).show(ctx, |ui| {
            ui.heading("Параметры структуры");
            ui.separator();
            ui.label(format!("V ср: {} м/с", self.settings.velocity));
            ui.label(format!("Качество: {} %", self.settings.quality));
            ui.label(format!("Шаг: {} м", self.settings.step));
            ui.label(format!("Карта: {} × {} м", MAP_WIDTH_M as i64, MAP_HEIGHT_M as i64));
            ui.label(format!("Узлы: {} × {}", self.settings.width, self.settings.height));
            ui.separator();
            if ui.button("Построить карту").clicked() {
                self.dirty = true;
            }
            ui.label(format!("Время: {:.3} мс", self.last_ms));
            ui.separator();
            ui.heading("Таблица корреляции");
            ui.separator();
            egui::Grid::new("corr").striped(true).show(ui, |ui| {
                ui.label("№");
                ui.label("X0");
                ui.label("Y0");
                ui.label("t min");
                ui.label("t max");
                ui.end_row();
                for p in &self.profiles {
                    ui.label((p.id + 1).to_string());
                    ui.label(format!("{:.0}", p.x0));
                    ui.label(format!("{:.0}", p.y0));
                    let (a, b) = p
                        .picks
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
                            (a.min(p.time), b.max(p.time))
                        });
                    if a.is_finite() {
                        ui.label(format!("{:.2}", a));
                        ui.label(format!("{:.2}", b));
                    } else {
                        ui.label("—");
                        ui.label("—");
                    }
                    ui.end_row();
                }
            });
        });
    }

    fn central(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Структурная карта");
            let size = ui.available_size();
            let mh = (size.y * 0.55).max(180.0);
            let (mrect, mresp) = ui.allocate_exact_size(Vec2::new(size.x, mh), Sense::click_and_drag());
            self.draw_map(ui, mrect, &mresp);

            ui.separator();
            ui.heading("Временной разрез");
            let sh = (ui.available_size().y - 24.0).max(120.0);
            let (srect, sresp) = ui.allocate_exact_size(Vec2::new(size.x, sh), Sense::click());
            self.draw_section(ui, srect, &sresp);

            ui.separator();
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Построить").clicked() {
                        self.dirty = true;
                    }
                });
            });
        });
    }

    fn draw_map(&mut self, ui: &mut egui::Ui, rect: Rect, resp: &egui::Response) {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, Color32::from_gray(24));
        if let Some(tex) = &self.texture {
            painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        }
        // сетка карты
        for i in 0..=8 {
            let x = rect.min.x + rect.width() * i as f32 / 8.0;
            painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], Stroke::new(0.5_f32, Color32::from_gray(60)));
        }
        for i in 0..=4 {
            let y = rect.min.y + rect.height() * i as f32 / 4.0;
            painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], Stroke::new(0.5_f32, Color32::from_gray(60)));
        }
        for (i, p) in self.profiles.iter().enumerate() {
            let a = Self::map_to_screen(Pos2::new(p.x0 as f32, p.y0 as f32), rect);
            let b = Self::map_to_screen(Pos2::new(p.x1 as f32, p.y1 as f32), rect);
            let col = if i == self.selected { Color32::YELLOW } else { Color32::from_rgb(90, 200, 255) };
            painter.line_segment([a, b], Stroke::new(if i == self.selected { 2.5_f32 } else { 1.5 }, col));
            painter.circle_filled(a, 3.0, col);
            painter.circle_filled(b, 3.0, col);
        }
        if let (Some(s), Some(c)) = (self.drag_start, self.drag_cur) {
            painter.line_segment([s, c], Stroke::new(2.0_f32, Color32::LIGHT_GREEN));
        }
        if resp.clicked() {
            self.drag_start = Some(resp.interact_pointer_pos().unwrap_or(resp.hover_pos().unwrap_or(rect.min)));
            self.drag_cur = self.drag_start;
        }
        if resp.dragged() {
            self.drag_cur = resp.interact_pointer_pos().or(self.drag_cur);
        }
        if resp.drag_stopped() {
            if let (Some(a), Some(b)) = (self.drag_start, self.drag_cur) {
                if (b - a).length() > 5.0 {
                    let p0 = Self::screen_to_map(a, rect);
                    let p1 = Self::screen_to_map(b, rect);
                    let id = self.profiles.len();
                    self.add_profile(p0.x as f64, p0.y as f64, p1.x as f64, p1.y as f64);
                    self.status = format!("Профиль №{} нарисован", id + 1);
                }
            }
            self.drag_start = None;
            self.drag_cur = None;
        }
    }

    fn draw_section(&mut self, ui: &mut egui::Ui, rect: Rect, resp: &egui::Response) {
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, Color32::from_gray(20));
        for i in 0..=6 {
            let x = rect.min.x + rect.width() * i as f32 / 6.0;
            painter.line_segment([Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)], Stroke::new(0.5_f32, Color32::from_gray(50)));
            painter.text(Pos2::new(x + 2.0, rect.min.y + 2.0), egui::Align2::LEFT_TOP, format!("{}", i * 20), egui::FontId::default(), Color32::GRAY);
        }
        for i in 0..=4 {
            let y = rect.min.y + rect.height() * i as f32 / 4.0;
            painter.line_segment([Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)], Stroke::new(0.5_f32, Color32::from_gray(50)));
            painter.text(Pos2::new(rect.min.x + 2.0, y + 1.0), egui::Align2::LEFT_TOP, format!("{:.1}", i as f32 * 0.5), egui::FontId::default(), Color32::GRAY);
        }
        let picks: Vec<Pick> = self
            .profiles
            .get(self.selected)
            .map(|p| p.picks.clone())
            .unwrap_or_default();
        if picks.len() > 1 {
            let mut pts = Vec::with_capacity(picks.len());
            for p in &picks {
                let x = rect.min.x + (p.trace / 120.0) as f32 * rect.width();
                let y = rect.max.y - (p.time / 2.0) as f32 * rect.height();
                pts.push(Pos2::new(x, y));
            }
            painter.add(egui::Shape::line(pts, Stroke::new(1.5_f32, Color32::from_rgb(255, 170, 60))));
        }
        for p in &picks {
            let x = rect.min.x + (p.trace / 120.0) as f32 * rect.width();
            let y = rect.max.y - (p.time / 2.0) as f32 * rect.height();
            painter.circle_filled(Pos2::new(x, y), 3.0, Color32::from_rgb(255, 220, 80));
        }
        if resp.clicked() {
            let pos = resp.interact_pointer_pos().unwrap_or(rect.min);
            let trace = ((pos.x - rect.min.x) / rect.width() * 120.0) as f64;
            let time = ((rect.max.y - pos.y) / rect.height() * 2.0) as f64;
            if self.profiles.is_empty() {
                self.add_profile(0.0, 2000.0, 8000.0, 2000.0);
            }
            if let Some(p) = self.profiles.get_mut(self.selected) {
                p.picks.push(Pick { trace, time });
                p.picks.sort_by(|a, b| a.trace.partial_cmp(&b.trace).unwrap_or(std::cmp::Ordering::Equal));
                self.dirty = true;
                self.status = format!("Пик: трасса {:.1}, t {:.3} с", trace, time);
            }
        }
    }
}

impl eframe::App for FoldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.dirty {
            self.recalc(ctx);
        }
        self.menu(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        self.central(ctx);

        if self.show_about {
            egui::Window::new("О программе").open(&mut self.show_about).show(ctx, |ui| {
                ui.label("Fold — построение структурной карты отражающего горизонта");
                ui.label("Оригинал: Viktor Romanov, 2007 (Delphi/VCL)");
                ui.label("Rust-порт: LGPL-совместимый独立 reimplementation");
                ui.label("Константы восстановлены из бинарника.");
            });
        }
    }
}
