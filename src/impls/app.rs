use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use egui::{Button, DragValue, Pos2, ProgressBar};
use memory_stats::memory_stats;

use crate::impls::{
    PROGRESS_MAX,
    async_light_thread::ManyToManyModel,
    async_os_thread::OneToOneModel,
    sync::ManyToOneModel,
    thread_model::{ThreadModel, ThreadModelKind},
};

pub struct App {
    model: Box<dyn ThreadModel>,
    foreground_tasks_started: bool,
    background_task_spawn_num: u32,
    counter: Arc<AtomicU64>,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            model: Box::new(ManyToOneModel::default()),
            counter: Default::default(),
            background_task_spawn_num: 1,
            foreground_tasks_started: false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.3);
        let mut current_model = self.model.get_kind();
        egui::Window::new(current_model.to_string())
            .default_pos(Pos2::new(20.0, 20.0))
            .show(ctx, |ui| {
                if ui
                    .radio_value(
                        &mut current_model,
                        ThreadModelKind::ManyToOne,
                        ThreadModelKind::ManyToOne.to_string(),
                    )
                    .changed()
                {
                    self.model = Box::new(ManyToOneModel::default());
                    self.foreground_tasks_started = false;
                    return;
                };
                if ui
                    .radio_value(
                        &mut current_model,
                        ThreadModelKind::OneToOne,
                        ThreadModelKind::OneToOne.to_string(),
                    )
                    .changed()
                {
                    self.model = Box::new(OneToOneModel::default());

                    self.foreground_tasks_started = false;
                    return;
                }
                if ui
                    .radio_value(
                        &mut current_model,
                        ThreadModelKind::ManyToMany,
                        ThreadModelKind::ManyToMany.to_string(),
                    )
                    .changed()
                {
                    self.model = Box::new(ManyToManyModel::default());

                    self.foreground_tasks_started = false;
                    return;
                }

                if !self.foreground_tasks_started {
                    self.model.create_foreground_task(ctx);
                    self.foreground_tasks_started = true;
                }
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut self.background_task_spawn_num));
                    if ui.button("Spawn n background tasks").clicked() {
                        for _ in 0..self.background_task_spawn_num {
                            self.model.create_background_task(self.counter.clone());
                        }
                    }
                });
                if self.model.get_kind() == ThreadModelKind::ManyToMany
                    && ui.add(Button::new("Spawn evil task")).clicked()
                {
                    self.model.create_evil_task();
                }
                ui.label(format!(
                    "Background tasks: {num}",
                    num = self.model.num_background_tasks()
                ));

                let mem_usage = memory_stats().unwrap().physical_mem / 1000000;
                ui.label(format!("Memory usage: {mem_usage}MB"));

                let counter = self.counter.load(Ordering::Relaxed);

                let progress = (counter % PROGRESS_MAX) as f32 / PROGRESS_MAX as f32;

                ui.label("Background Task Progress:");
                ui.add(ProgressBar::new(progress));
            });

        self.model.run_interactive(ctx);
        self.model.join_interactive();
    }
}
