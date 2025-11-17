use std::{
    env::current_dir,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    thread::sleep,
    time::Duration,
};

use egui::{Button, Context, DragValue, ImageSource};
use rand::Rng;

use crate::impls::{
    DEFAULT_IMAGE, load_image,
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
    image: ImageSource<'static>,
    text_buffer: String,
    form_name: String,
    form_number: u32,
}

impl SyncForegroundTask {
    pub fn new(image: ImageSource<'static>) -> Self {
        Self {
            image,
            text_buffer: Default::default(),
            form_name: Default::default(),
            form_number: Default::default(),
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        egui::Window::new("Image Viewer").show(ctx, |ui| {
            ui.image(self.image.clone());
            if ui.add(Button::new("Load Image")).clicked() {
                let dialogue = rfd::FileDialog::new().set_directory(current_dir().unwrap());
                let result = dialogue.pick_file();
                if let Some(result) = result {
                    self.image = load_image(&result, ctx);
                }
            }
        });

        egui::Window::new("Text Editor").show(ctx, |ui| {
            ui.text_edit_multiline(&mut self.text_buffer);
        });
        egui::Window::new("Form").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");
                ui.text_edit_singleline(&mut self.form_name);
            });

            ui.horizontal(|ui| {
                ui.label("Age: ");
                ui.add(DragValue::new(&mut self.form_number));
            });
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

    fn create_foreground_task(&mut self) {
        let image = DEFAULT_IMAGE;
        self.foreground_tasks.push(SyncForegroundTask::new(image));
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
