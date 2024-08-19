mod config;
mod net;

use std::sync::Arc;
use std::io::Write;
use rustyline_async::{Readline, SharedWriter, ReadlineEvent};
use tracing_appender::non_blocking::WorkerGuard;
use crate::ServerContext;

type ArgSlice<'a> = &'a [&'a str];

pub struct CommandManager {
    context: Option<Arc<ServerContext>>,
}

impl CommandManager {
    pub fn new() -> Self {
        Self { context: None }
    }

    pub fn set_context(&mut self, context: Arc<ServerContext>) {
        self.context = Some(context);
    }

    pub fn run(&self, mut rl: Readline, mut out: SharedWriter, guard: WorkerGuard) {
        tokio::spawn(async move {
            loop {
                match rl.readline().await {
                    Ok(ReadlineEvent::Line(line)) => {
                        writeln!(&mut out, "Some Output").unwrap();
                        rl.add_history_entry(line);
                    }
                    Ok(ReadlineEvent::Eof) | Ok(ReadlineEvent::Interrupted) => {
                        tracing::info!("Received Ctrl+C, shutting down...");
                        drop(guard);
                        tracing::info!("All logs should be flushed now.");
                        rl.flush().unwrap();
                        drop(rl);
                        std::process::exit(0);
                    }
                    _ => continue,
                }
            }
        });
    }
}
