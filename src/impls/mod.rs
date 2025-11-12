use std::path::Path;

use egui::{Context, ImageSource};

pub mod app;
pub mod async_light_thread;
pub mod async_os_thread;
pub mod sync;

pub fn load_image(path: &Path, ctx: &Context) -> ImageSource<'static> {
    let uri = format!("bytes://{path}", path = path.to_string_lossy());
    let file = std::fs::read(path).unwrap();
    ctx.include_bytes(uri.clone(), file);
    ImageSource::Uri(uri.into())
}

pub const PROGRESS_MAX: u64 = 12000;
