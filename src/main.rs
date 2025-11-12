pub mod impls;

use std::time::Duration;

use eframe::egui;
use egui_extras::install_image_loaders;
use tokio::runtime::Runtime;

use crate::impls::{app::App, async_light_thread::GreenThreadModel};

fn main() -> eframe::Result {
    let runtime = Runtime::new().unwrap();
    let _enter = runtime.enter();
    std::thread::spawn(move || {
        runtime.block_on(async {
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        })
    });
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };
    let app = Box::new(App::new());
    eframe::run_native(
        "Multithreading Model Demo",
        options,
        Box::new(|cc| {
            install_image_loaders(&cc.egui_ctx);
            Ok(app)
        }),
    )
}
