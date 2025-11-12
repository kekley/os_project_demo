use egui::Context;
use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

#[derive(Debug, PartialEq, Eq)]
pub enum ThreadModelKind {
    ManyToMany,
    ManyToOne,
    OneToOne,
}

impl Display for ThreadModelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ThreadModelKind::ManyToMany => "Many to Many",
            ThreadModelKind::ManyToOne => "Many to One",
            ThreadModelKind::OneToOne => "One to One",
        };
        f.write_str(str)
    }
}

pub trait ThreadModel {
    fn get_kind(&self) -> ThreadModelKind;
    fn create_foreground_task(&mut self, ctx: &Context);
    fn create_background_task(&mut self, counter: Arc<AtomicU64>);
    fn create_evil_task(&mut self);
    fn num_background_tasks(&self) -> usize;
    fn run_interactive(&mut self, ctx: &Context);
    fn join_interactive(&mut self);
}
