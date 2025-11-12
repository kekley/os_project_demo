use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use egui::ProgressBar;
use memory_stats::memory_stats;

use crate::impls::{PROGRESS_MAX, sync::ThreadModel};

pub struct App {
    model: Box<dyn ThreadModel>,
    counter: Arc<AtomicU64>,
}

impl App {}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::Window::new("One to One").show(ctx, |ui| {
            if ui.button("Spawn a foreground task").clicked() {
                self.model.create_foreground_task(ctx)
            }
            if ui.button("Spawn 1000 background tasks").clicked() {
                for _ in 0..1000 {
                    self.model.create_background_task(self.counter.clone());
                }
            }
            ui.label(format!(
                "Background tasks: {num}",
                num = self.model.num_background_tasks()
            ));

            let mem_usage = memory_stats().unwrap().physical_mem / 1000000;
            ui.label(format!("Memory usage: {mem_usage}MB"));

            let counter = self.counter.load(Ordering::Relaxed);

            let progress = (counter % PROGRESS_MAX) as f32 / PROGRESS_MAX as f32;

            ui.add(ProgressBar::new(progress));
        });

        self.model.run_interactive(ctx);
        self.model.join_interactive();
    }
}
