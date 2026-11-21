fn main() -> eframe::Result<()> {
    // Клиент формы original из DFM: 1016x694. Плюс 20 px слой меню,
    // которое в Delphi системное и не входит в клиентскую область.
    // Размер ЗАФИКСИРОВАН: масштабирование окна даёт «кривоту» линий и текста.
    let size = [1016.0, 714.0];
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Fold")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "Fold",
        options,
        Box::new(|cc| Ok(Box::new(fold::gui::FoldApp::new(cc)))),
    )
}
