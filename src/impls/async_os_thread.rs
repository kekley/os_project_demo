use crate::impls::PROGRESS_MAX;
use egui::ProgressBar;
use memory_stats::memory_stats;
use std::{
    env::current_dir,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, sync_channel},
    },
    thread::{JoinHandle, sleep, spawn},
    time::Duration,
};

use egui::{Button, Context, ImageSource};

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

pub fn os_background(counter: Arc<AtomicU64>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            counter.fetch_add(1, Ordering::Relaxed);
            sleep(Duration::from_secs(1));
        }
    })
}

pub struct OsThreadApp {
    interactive_threads: Vec<(JoinHandle<()>, SyncSender<egui::Context>)>,
    background_threads: Vec<JoinHandle<()>>,
    counter: Arc<AtomicU64>,
    on_done_tx: SyncSender<()>,
    on_done_rc: Receiver<()>,
}

impl OsThreadApp {
    pub fn new() -> Self {
        let (on_done_tx, on_done_rc) = sync_channel(0);

        Self {
            interactive_threads: Vec::new(),
            on_done_tx,
            on_done_rc,
            background_threads: Vec::new(),
            counter: Default::default(),
        }
    }

    fn create_interactive_task(&mut self, ctx: &Context) {
        let thread_nr = self.interactive_threads.len();
        self.interactive_threads.push(os_foreground(
            thread_nr,
            self.on_done_tx.clone(),
            "assets/shocked.gif",
            ctx,
        ));
    }
    fn create_background_task(&mut self) {
        self.background_threads
            .push(os_background(self.counter.clone()));
    }
}

impl Default for OsThreadApp {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Drop for OsThreadApp {
    fn drop(&mut self) {
        for (handle, show_tx) in self.interactive_threads.drain(..) {
            std::mem::drop(show_tx);
            handle.join().unwrap();
        }
    }
}

impl eframe::App for OsThreadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {}
}
