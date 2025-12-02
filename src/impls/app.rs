use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use egui::{Button, CentralPanel, DragValue, Pos2, ProgressBar};
use memory_stats::memory_stats;

use crate::impls::{
    PROGRESS_MAX,
    async_light_thread::ManyToManyModel,
    async_os_thread::OneToOneModel,
    sync::ManyToOneModel,
    thread_model::{ThreadModel, ThreadModelKind},
};

pub const DEFAULT_TEXT: &str = "Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and scrambled it to make a type specimen book. It has survived not only five centuries, but also the leap into electronic typesetting, remaining essentially unchanged. It was popularised in the 1960s with the release of Letraset sheets containing Lorem Ipsum passages, and more recently with desktop publishing software like Aldus PageMaker including versions of Lorem Ipsum.";
pub const DEFAULT_NAME: &str = "First Last";
pub const DEFAULT_AGE: u32 = 42;

pub struct App {
    model: Box<dyn ThreadModel>,
    foreground_tasks_started: bool,
    background_task_spawn_num: u32,
    counter: Arc<AtomicU64>,
    bench_result: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    bench_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            model: Box::new(ManyToOneModel::default()),
            counter: Default::default(),
            background_task_spawn_num: 1,
            foreground_tasks_started: false,
            bench_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
            bench_running: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_pixels_per_point(1.3);
        let mut current_model = self.model.get_kind();
        CentralPanel::default().show(ctx, |_| {
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
                        self.model.create_foreground_task();
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

                    ui.separator();
                    if ui.button("Run overhead benchmark").clicked() {
                        let n = 1000;
                        let iterations = 5000usize;
                        let bench_result = self.bench_result.clone();
                        let bench_running = self.bench_running.clone();
                        bench_running.store(true, Ordering::Relaxed);
                        std::thread::spawn(move || {
                            let out = crate::impls::bench::run_benchmarks(n, iterations);
                            *bench_result.lock().unwrap() = Some(out);
                            bench_running.store(false, Ordering::Relaxed);
                        });
                    }

                    if self.bench_running.load(Ordering::Relaxed) {
                        ui.label("Benchmark running...");
                    } else if let Some(res) = self.bench_result.lock().unwrap().as_ref() {
                        ui.label("Benchmark result:");
                        ui.collapsing("Details", |ui| {
                            ui.label(res.clone());
                        });
                    }

                    let mem_usage = memory_stats().unwrap().physical_mem / 1000000;
                    ui.label(format!("Memory usage: {mem_usage}MB"));

                    let counter = self.counter.load(Ordering::Relaxed);

                    let progress = (counter % PROGRESS_MAX) as f32 / PROGRESS_MAX as f32;

                    ui.label("Background Task Progress:");
                    ui.add(ProgressBar::new(progress));
                });

            self.model.run_interactive(ctx);
            self.model.join_interactive();
        });
    }
}
