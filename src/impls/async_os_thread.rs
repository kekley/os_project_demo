use crate::impls::{
    IMAGE_PATH,
    thread_model::{ThreadModel, ThreadModelKind},
};
use std::{
    env::current_dir,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread::{JoinHandle, sleep, spawn},
    time::Duration,
};

use egui::{Button, Context, ImageSource};
use rand::Rng;

use crate::impls::load_image;

pub struct OsForegroundTask {
    thread_nr: usize,
    title: String,
    image: ImageSource<'static>,
    loader_thread: Option<JoinHandle<Option<PathBuf>>>,
}

impl OsForegroundTask {
    fn new(thread_nr: usize, image: ImageSource<'static>) -> Self {
        let title = format!("OS thread {thread_nr}");
        Self {
            thread_nr,
            title,
            image,
            loader_thread: None,
        }
    }

    fn show(&mut self, ctx: &egui::Context) {
        let pos = egui::pos2(
            128.0 * (self.thread_nr / 7) as f32,
            128.0 * ((self.thread_nr % 7) as f32 + 1.0),
        );
        egui::Window::new(&self.title)
            .default_pos(pos)
            .show(ctx, |ui| {
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
    }
}

pub fn os_foreground(
    thread_nr: usize,
    on_done_tx: SyncSender<()>,
    image_path: &str,
    ctx: &Context,
) -> (JoinHandle<()>, SyncSender<Context>) {
    let image = load_image(Path::new(image_path), ctx);
    let (show_tx, show_rc) = sync_channel(0);
    let handle = std::thread::Builder::new()
        .name(format!("Worker {thread_nr}"))
        .spawn(move || {
            let mut state = OsForegroundTask::new(thread_nr, image);
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

    fn create_foreground_task(&mut self, ctx: &Context) {
        let thread_nr = self.foreground_tasks.len();
        self.foreground_tasks.push(os_foreground(
            thread_nr,
            self.on_done_tx.clone(),
            IMAGE_PATH,
            ctx,
        ));
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
