use super::prelude::*;
/// History命令处理器
pub struct HistoryCommand;

impl Builtin for HistoryCommand {
    fn execute(
        &self,
        mut params: Box<dyn Iterator<Item = String>>,
        rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult {
        match params.next() {
            Some(dir) => {
                if dir == "-r" || dir == "-w" || dir == "-a" {
                    let file: Option<String> = params.next();
                    return match crate::history::handle_history_options(&dir, file, rl) {
                        Ok(_) => CommandResult::default(),
                        Err(e) => CommandResult::new_with_stderr(format!("{}\n", e)),
                    };
                }

                let num = dir
                    .parse::<usize>()
                    .context("history number is not a number");
                let history = rl.history();
                let len = history.len();
                match num {
                    Ok(num) => {
                        if num > len {
                            CommandResult::new_with_stdout(crate::print_iter(history).collect())
                        } else {
                            CommandResult::new_with_stdout(
                                crate::print_iter(history).skip(len - num).collect(),
                            )
                        }
                    }
                    Err(_) => CommandResult::new_with_stderr(format!(
                        "history: {}: event not found\n",
                        dir
                    )),
                }
            }
            None => CommandResult::new_with_stdout(crate::print_iter(rl.history()).collect()),
        }
    }
}
