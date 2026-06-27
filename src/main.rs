#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod app;
mod command;
mod config;
mod discovery;
mod server;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "本地 AI 启动器",
        options,
        Box::new(|cc| {
            app::configure_fonts(&cc.egui_ctx);
            Ok(Box::new(app::LauncherApp::new()))
        }),
    )
}
