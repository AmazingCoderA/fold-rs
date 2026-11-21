fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Fold")
            .with_inner_size([894.0, 656.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Fold",
        options,
        Box::new(|cc| Ok(Box::new(fold::gui::FoldApp::new(cc)))),
    )
}
