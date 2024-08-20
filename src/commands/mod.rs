mod config;
mod net;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::io::Write;
use rustyline_async::{Readline, SharedWriter, ReadlineEvent};
use tracing_appender::non_blocking::WorkerGuard;
use crate::ServerContext;

type ArgSlice<'a> = &'a [&'a str];

macro_rules! commands {
    ($($category:ident::$action:ident $usage:tt $desc:tt;)*) => {
        async fn exec(state: &ServerContext, cmd: &str) -> String {
            let input = cmd.split(" ").collect::<Vec<&str>>();

            if input.len() == 1 && *input.get(0).unwrap() == "help" {
                return Self::help_message();
            }

            let (Some(category), Some(action)) = (input.get(0), input.get(1)) else {
                return String::from("Unrecognized command, enter `help` to view supported commands");
            };

            let args = &input[2..];
            match match (*category, *action) {
                $(
                    (stringify!($category), stringify!($action)) => {
                        $category::$action(args, state).await
                    }
                )*,
                _ => {
                    return String::from("Unrecognized command, enter `help` to view supported commands");
                }
            } {
                Ok(s) => s,
                Err(err) => format!("failed to execute command: {err}"),
            }
        }

        fn help_message() -> String {
            concat!("available commands:\n",
                $(stringify!($category), " ", stringify!($action), " ", $usage, " - ", $desc, "\n",)*
                "help - shows this message"
            ).to_string()
        }
    };
}

pub struct CommandManager {
    context: Arc<Mutex<Option<Arc<ServerContext>>>>,
}

impl CommandManager {
    pub fn new() -> Self {
        Self { context: Arc::new(Mutex::new(None)) }
    }

    pub async fn set_context(&mut self, new_context: Arc<ServerContext>) {
        let mut context = self.context.lock().await;
        *context = Some(new_context);
    }

    pub fn run(&self, mut rl: Readline, mut out: SharedWriter, guard: WorkerGuard) {
        let context = Arc::clone(&self.context);
        tokio::spawn(async move {
            loop {
                match rl.readline().await {
                    Ok(ReadlineEvent::Line(line)) => {
                        let context = context.lock().await;
                        if let Some(context) = &*context {
                            let str = Self::exec(context, &line).await;
                            writeln!(&mut out, "{str}").unwrap();
                            rl.add_history_entry(line);
                        } else {
                            tracing::error!("Using commands before init Server Context");
                        }
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

    commands! {
        config::timeout "[websocket_proxy|tcp_proxy] [timeout]" "Set the service timeout";
        config::save "" "Save the current configuration to file";
        config::show "" "Show the current configuration";
        net::reconnect "[websocket_proxy|tcp_proxy]" "Reconnect service";
    }
}
