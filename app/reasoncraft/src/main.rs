//! ReasonCraft desktop app (stub: opens an empty window).

struct ReasonCraftApp;

impl eframe::App for ReasonCraftApp {
    fn ui(&mut self, _ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {}
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ReasonCraft")
            .with_app_id("nl.reasoncraft.app"),
        ..Default::default()
    };
    eframe::run_native(
        "ReasonCraft",
        options,
        Box::new(|_cc| Ok(Box::new(ReasonCraftApp))),
    )
}
