use std::{
    env::current_dir,
    path::Path,
    sync::{Arc, atomic::AtomicU64},
    thread::JoinHandle,
};

use egui::{Button, Context, ImageSource};

use crate::impls::load_image;

pub struct SyncBackgroundTask {
    counter: u64,
}

impl SyncBackgroundTask {
    pub fn run(&mut self) {
        self.counter += 1;
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

pub fn sync_interactive(thread_nr: usize, image_path: &str, ctx: &Context) -> SyncForegroundTask {
    let image = load_image(Path::new(image_path), ctx);
    SyncForegroundTask::new(thread_nr, image)
}

pub trait ThreadModel {
    fn create_foreground_task(&mut self, ctx: &Context);
    fn create_background_task(&mut self, counter: Arc<AtomicU64>);
    fn num_background_tasks(&self) -> usize;
    fn run_interactive(&mut self, ctx: &Context);
    fn join_interactive(&mut self);
}
