use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use super::prelude::*;
/// Complete命令处理器
pub struct CompleteCommand;

pub static GLOBAL_COMPLETION_MANAGER: LazyLock<Mutex<CompletionManager>> =
    LazyLock::new(|| Mutex::new(CompletionManager::new()));

impl Builtin for CompleteCommand {
    fn execute(
        &self,
        params: Vec<String>,
        _context: &mut ExecutionContext,
    ) -> BuiltinCommandResult {
        let mut args = params.into_iter();
        match args.next().as_deref() {
            Some("-p") => match args.next() {
                Some(cmd) => {
                    let completion_manager = GLOBAL_COMPLETION_MANAGER.lock().unwrap();
                    if let Some(path) = completion_manager.completions.get(&cmd) {
                        // complete -C '/path/to/completer/script' git
                        BuiltinCommandResult::new_with_stdout(format!(
                            "complete -C '{}' {}\n",
                            path, cmd
                        ))
                    } else {
                        BuiltinCommandResult::new_with_stderr(
                            format!("complete: {}: no completion specification\n", cmd).to_string(),
                        )
                    }
                }

                None => BuiltinCommandResult::new_with_stderr(
                    "complete: option requires an argument -- p\n".to_string(),
                ),
            },
            Some("-C") => match (args.next(), args.next()) {
                (Some(path), Some(cmd)) => {
                    let mut completion_manager = GLOBAL_COMPLETION_MANAGER.lock().unwrap();
                    completion_manager
                        .completions
                        .insert(cmd.to_string(), path.to_string());
                    BuiltinCommandResult::default()
                }

                _ => BuiltinCommandResult::new_with_stderr(
                    "complete: option requires an argument -- n\n".to_string(),
                ),
            },
            Some("-r") => match args.next().as_deref() {
                Some(cmd) => {
                    let mut completion_manager = GLOBAL_COMPLETION_MANAGER.lock().unwrap();
                    completion_manager.completions.remove(cmd);
                    BuiltinCommandResult::default()
                }
                None => BuiltinCommandResult::default(),
            },

            _ => BuiltinCommandResult::new_with_stderr(
                "complete: option requires an argument -- p\n".to_string(),
            ),
        }
    }
}

pub struct CompletionManager {
    pub completions: HashMap<String, String>,
}
impl CompletionManager {
    pub fn new() -> Self {
        Self {
            completions: HashMap::new(),
        }
    }
}
