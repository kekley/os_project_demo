use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use egui::{Button, Context, ImageSource};
use pollster::FutureExt;
use rand::Rng;
use rfd::{AsyncFileDialog, FileHandle};
use tokio::{
    spawn,
    sync::mpsc::{Receiver, Sender, channel},
    task::JoinHandle,
    time::sleep,
};

use crate::impls::{
    IMAGE_PATH, load_image,
    thread_model::{ThreadModel, ThreadModelKind},
};

pub struct ForegroundGreenThread {
    thread_nr: usize,
    title: String,
    image: ImageSource<'static>,
    loader_thread: Option<JoinHandle<Option<FileHandle>>>,
}

impl ForegroundGreenThread {
    fn new(thread_nr: usize, image: ImageSource<'static>) -> Self {
        let title = format!("Green thread {thread_nr}");
        Self {
            thread_nr,
            title,
            image,
            loader_thread: None,
        }
    }

    async fn show(&mut self, ctx: &egui::Context) {
        let pos = egui::pos2(
            128.0 * (self.thread_nr / 7) as f32,
            128.0 * ((self.thread_nr % 7) as f32 + 1.0),
        );
        if let Some(handle) = self.loader_thread.take_if(|handle| handle.is_finished())
            && let Ok(result) = handle.await
            && let Some(path) = result
        {
            self.image = load_image(path.path(), ctx);
        }

        egui::Window::new(&self.title)
            .default_pos(pos)
            .show(ctx, |ui| {
                ui.image(self.image.clone());
                if ui
                    .add_enabled(self.loader_thread.is_none(), Button::new("Load Image"))
                    .clicked()
                {
                    let dialogue = AsyncFileDialog::new();
                    let handle = spawn(dialogue.pick_file());
                    self.loader_thread = Some(handle);
                }
            });
    }
}

pub fn foreground_green_thread(
    thread_nr: usize,
    on_done_tx: Sender<()>,
    image_path: &str,
    ctx: &Context,
) -> (JoinHandle<()>, Sender<Context>) {
    let image = load_image(Path::new(image_path), ctx);
    let (show_tx, show_rc) = channel(1);
    let handle = tokio::spawn(inner(thread_nr, image, show_rc, on_done_tx));
    (handle, show_tx)
}

async fn inner(
    thread_nr: usize,
    image: ImageSource<'static>,
    mut show_rc: Receiver<Context>,
    on_done_tx: Sender<()>,
) {
    let mut state = ForegroundGreenThread::new(thread_nr, image);
    while let Some(ctx) = show_rc.recv().await {
        state.show(&ctx).await;
        let _ = on_done_tx.send(()).await;
    }
}

pub fn background_green_thread(
    counter: Arc<AtomicU64>,
    finished: Arc<AtomicBool>,
) -> JoinHandle<()> {
    spawn(inner_background(counter, finished))
}

pub fn background_evil_thread(finished: Arc<AtomicBool>) -> JoinHandle<()> {
    spawn(inner_evil(finished))
}

async fn inner_background(counter: Arc<AtomicU64>, finished: Arc<AtomicBool>) {
    while !finished.load(Ordering::Relaxed) {
        let duration = {
            let mut rng = rand::rng();

            rng.random_range(0..1000)
        };
        counter.fetch_add(1, Ordering::Relaxed);
        sleep(Duration::from_millis(duration)).await;
    }
}

async fn inner_evil(finished: Arc<AtomicBool>) {
    while !finished.load(Ordering::Relaxed) {
        let duration = {
            let mut rng = rand::rng();

            rng.random_range(0..1000)
        };
        //This actually blocks the thread rather than cooperatively yielding execution
        //If all the kernel threads block, execution cannot continue
        std::thread::sleep(Duration::from_millis(duration));
    }
}
pub struct ManyToManyModel {
    foreground_tasks: Vec<(JoinHandle<()>, Sender<Context>)>,
    background_tasks: Vec<JoinHandle<()>>,
    on_done_tx: Sender<()>,
    on_done_rx: Receiver<()>,
    finished: Arc<AtomicBool>,
}

impl ManyToManyModel {
    pub fn new() -> Self {
        let (on_done_tx, on_done_rx) = channel(100000);
        Self {
            foreground_tasks: Vec::new(),
            background_tasks: Vec::new(),
            on_done_tx,
            on_done_rx,
            finished: Default::default(),
        }
    }
}

impl Default for ManyToManyModel {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadModel for ManyToManyModel {
    fn create_foreground_task(&mut self, ctx: &Context) {
        let thread_nr = self.foreground_tasks.len();
        self.foreground_tasks.push(foreground_green_thread(
            thread_nr,
            self.on_done_tx.clone(),
            IMAGE_PATH,
            ctx,
        ));
    }

    fn create_background_task(&mut self, counter: Arc<AtomicU64>) {
        self.background_tasks
            .push(background_green_thread(counter, self.finished.clone()));
    }

    fn num_background_tasks(&self) -> usize {
        self.background_tasks.len()
    }

    fn run_interactive(&mut self, ctx: &Context) {
        for (_, sender) in self.foreground_tasks.iter() {
            pollster::block_on(sender.send(ctx.clone())).unwrap();
        }
    }

    fn join_interactive(&mut self) {
        for _ in self.foreground_tasks.iter() {
            pollster::block_on(self.on_done_rx.recv()).unwrap();
        }
    }

    fn get_kind(&self) -> ThreadModelKind {
        ThreadModelKind::ManyToMany
    }

    fn create_evil_task(&mut self) {
        self.background_tasks
            .push(background_evil_thread(self.finished.clone()));
    }
}

impl std::ops::Drop for ManyToManyModel {
    fn drop(&mut self) {
        self.finished.store(true, Ordering::Relaxed);
        for (handle, show_tx) in self.foreground_tasks.drain(..) {
            std::mem::drop(show_tx);
            handle.block_on().unwrap();
        }
        for handle in self.background_tasks.drain(..) {
            handle.block_on().unwrap();
        }
    }
}
