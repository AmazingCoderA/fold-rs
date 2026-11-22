fn main() -> eframe::Result<()> {
    // Клиент формы original из DFM: 1016x694. Плюс 20 px слой меню,
    // которое в Delphi системное и не входит в клиентскую область.
    // Окно растягиваемое: gui.rs масштабирует DFM-координаты одним
    // коэффициентом для обеих осей, поэтому подпись внизу не обрезается.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Fold")
            .with_inner_size([1016.0, 714.0])
            .with_min_inner_size([640.0, 450.0])
            .with_resizable(true),
        ..Default::default()
    };
    eframe::run_native(
        "Fold",
        options,
        Box::new(|cc| Ok(Box::new(fold::gui::FoldApp::new(cc)))),
    )
}
