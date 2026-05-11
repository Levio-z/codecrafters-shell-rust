use super::prelude::*;
use crate::GLOBAL_EDITOR;
/// History命令处理器
pub struct HistoryCommand;

impl Builtin for HistoryCommand {
    fn execute(&self, params: Vec<String>, _: &mut ExecutionContext) -> BuiltinCommandResult {
        let mut params = params.iter();
        match params.next() {
            Some(dir) => {
                if dir == "-r" || dir == "-w" || dir == "-a" {
                    let file: Option<&String> = params.next();
                    let mut rl = GLOBAL_EDITOR.lock().unwrap();
                    return match crate::history::handle_history_options(dir, file, &mut rl) {
                        Ok(_) => BuiltinCommandResult::default(),
                        Err(e) => BuiltinCommandResult::new_with_stderr(format!("{}\n", e)),
                    };
                }

                let num = dir
                    .parse::<usize>()
                    .context("history number is not a number");
                let rl = GLOBAL_EDITOR.lock().unwrap();
                let history = rl.history();
                let len = history.len();
                match num {
                    Ok(num) => {
                        if num > len {
                            BuiltinCommandResult::new_with_stdout(
                                crate::history::print_iter(history).collect(),
                            )
                        } else {
                            BuiltinCommandResult::new_with_stdout(
                                crate::history::print_iter(history)
                                    .skip(len - num)
                                    .collect(),
                            )
                        }
                    }
                    Err(_) => BuiltinCommandResult::new_with_stderr(format!(
                        "history: {}: event not found\n",
                        dir
                    )),
                }
            }
            None => BuiltinCommandResult::new_with_stdout({
                let rl = GLOBAL_EDITOR.lock().unwrap();
                crate::history::print_iter(rl.history()).collect()
            }),
        }
    }
}
