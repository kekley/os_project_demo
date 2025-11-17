use crate::impls::{
    DEFAULT_IMAGE,
    app::{DEFAULT_AGE, DEFAULT_NAME, DEFAULT_TEXT},
    thread_model::{ThreadModel, ThreadModelKind},
};
use std::{
    env::current_dir,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread::{JoinHandle, sleep, spawn},
    time::Duration,
};

use egui::{Button, Context, DragValue, ImageSource};
use rand::Rng;

use crate::impls::load_image;

pub struct OsForegroundTask {
    image: ImageSource<'static>,
    loader_thread: Option<JoinHandle<Option<PathBuf>>>,
    text_buffer: String,
    form_name: String,
    form_number: u32,
}

impl OsForegroundTask {
    fn new(image: ImageSource<'static>) -> Self {
        Self {
            image,
            loader_thread: None,
            text_buffer: DEFAULT_TEXT.to_string(),
            form_name: DEFAULT_NAME.to_string(),
            form_number: DEFAULT_AGE,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        egui::Window::new("Image viewer").show(ctx, |ui| {
            ui.image(self.image.clone());
            if let Some(handle) = self.loader_thread.take_if(|handle| handle.is_finished())
                && let Ok(result) = handle.join()
                && let Some(path) = result
            {
                self.image = load_image(&path, ctx);
            }

            if ui
                .add_enabled(self.loader_thread.is_none(), Button::new("Load Image"))
                .clicked()
            {
                let dialogue = rfd::FileDialog::new().set_directory(current_dir().unwrap());
                let handle = spawn(|| dialogue.pick_file());
                self.loader_thread = Some(handle);
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

pub fn os_foreground(
    thread_nr: usize,
    on_done_tx: SyncSender<()>,
) -> (JoinHandle<()>, SyncSender<Context>) {
    let image = DEFAULT_IMAGE;
    let (show_tx, show_rc) = sync_channel(0);
    let handle = std::thread::Builder::new()
        .name(format!("Worker {thread_nr}"))
        .spawn(move || {
            let mut state = OsForegroundTask::new(image);
            while let Ok(ctx) = show_rc.recv() {
                state.show(&ctx);
                let _ = on_done_tx.send(());
            }
        })
        .expect("failed to spawn thread");
    (handle, show_tx)
}

pub fn os_background(counter: Arc<AtomicU64>, finished: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while !finished.load(Ordering::Relaxed) {
            let duration = {
                let mut rng = rand::rng();

                rng.random_range(0..1000)
            };
            counter.fetch_add(1, Ordering::Relaxed);
            sleep(Duration::from_millis(duration))
        }
    })
}

pub struct OneToOneModel {
    foreground_tasks: Vec<(JoinHandle<()>, SyncSender<egui::Context>)>,
    background_tasks: Vec<JoinHandle<()>>,
    on_done_tx: SyncSender<()>,
    on_done_rx: Receiver<()>,
    finished: Arc<AtomicBool>,
}

impl OneToOneModel {
    pub fn new() -> Self {
        let (on_done_tx, on_done_rc) = sync_channel(0);

        Self {
            foreground_tasks: Vec::new(),
            on_done_tx,
            on_done_rx: on_done_rc,
            background_tasks: Vec::new(),
            finished: Default::default(),
        }
    }
}

impl ThreadModel for OneToOneModel {
    fn get_kind(&self) -> ThreadModelKind {
        ThreadModelKind::OneToOne
    }

    fn create_foreground_task(&mut self) {
        let thread_nr = self.foreground_tasks.len();
        self.foreground_tasks
            .push(os_foreground(thread_nr, self.on_done_tx.clone()));
    }

    fn create_background_task(&mut self, counter: Arc<AtomicU64>) {
        self.background_tasks
            .push(os_background(counter.clone(), self.finished.clone()));
    }

    fn num_background_tasks(&self) -> usize {
        self.background_tasks.len()
    }

    fn run_interactive(&mut self, ctx: &Context) {
        for (_, sender) in self.foreground_tasks.iter_mut() {
            let _ = sender.send(ctx.clone());
        }
    }

    fn join_interactive(&mut self) {
        for _ in self.foreground_tasks.iter_mut() {
            let _ = self.on_done_rx.recv();
        }
    }

    fn create_evil_task(&mut self) {}
}

impl Default for OneToOneModel {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Drop for OneToOneModel {
    fn drop(&mut self) {
        self.finished.store(true, Ordering::Relaxed);
        for (handle, show_tx) in self.foreground_tasks.drain(..) {
            std::mem::drop(show_tx);
            handle.join().unwrap()
        }
        for handle in self.background_tasks.drain(..) {
            handle.join().unwrap();
        }
    }
}
