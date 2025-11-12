use std::{
    env::current_dir,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread::sleep,
    time::Duration,
};

use egui::{Button, Context, ImageSource};
use rand::Rng;

use crate::impls::{
    IMAGE_PATH, load_image,
    thread_model::{ThreadModel, ThreadModelKind},
};

pub struct SyncBackgroundTask {
    counter: Arc<AtomicU64>,
}

impl SyncBackgroundTask {
    pub fn run(&mut self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
        let duration = {
            let mut rng = rand::rng();

            rng.random_range(0..1000)
        };

        sleep(Duration::from_millis(duration));
    }
}

pub struct SyncForegroundTask {
    task_nr: usize,
    title: String,
    image: ImageSource<'static>,
}

impl SyncForegroundTask {
    pub fn new(task_nr: usize, image: ImageSource<'static>) -> Self {
        let title = format!("Window {task_nr}");
        Self {
            task_nr,
            title,
            image,
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let pos = egui::pos2(
            128.0 * (self.task_nr / 7) as f32,
            128.0 * ((self.task_nr % 7) as f32 + 1.0),
        );
        egui::Window::new(&self.title)
            .default_pos(pos)
            .show(ctx, |ui| {
                ui.image(self.image.clone());
                if ui.add(Button::new("Load Image")).clicked() {
                    let dialogue = rfd::FileDialog::new().set_directory(current_dir().unwrap());
                    let result = dialogue.pick_file();
                    if let Some(result) = result {
                        self.image = load_image(&result, ctx);
                    }
                }
            });
    }
}

#[derive(Default)]
pub struct ManyToOneModel {
    foreground_tasks: Vec<SyncForegroundTask>,
    background_tasks: Vec<SyncBackgroundTask>,
}

impl ThreadModel for ManyToOneModel {
    fn get_kind(&self) -> ThreadModelKind {
        ThreadModelKind::ManyToOne
    }

    fn create_foreground_task(&mut self, ctx: &Context) {
        let image = load_image(Path::new(IMAGE_PATH), ctx);
        self.foreground_tasks
            .push(SyncForegroundTask::new(self.foreground_tasks.len(), image));
    }

    fn create_background_task(&mut self, counter: Arc<AtomicU64>) {
        self.background_tasks.push(SyncBackgroundTask { counter });
    }

    fn create_evil_task(&mut self) {}

    fn num_background_tasks(&self) -> usize {
        self.background_tasks.len()
    }

    fn run_interactive(&mut self, ctx: &Context) {
        for task in self.foreground_tasks.iter_mut() {
            task.show(ctx);
        }
    }

    fn join_interactive(&mut self) {
        //We don't need to join any threads in this model, so use this function to run
        //background tasks
        for task in self.background_tasks.iter_mut() {
            task.run();
        }
    }
}
