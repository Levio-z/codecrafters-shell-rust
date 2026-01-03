use super::prelude::*;
/// Pwd命令处理器
pub struct PwdCommand;

impl Builtin for PwdCommand {
    fn execute(
        &self,
        _params: Box<dyn Iterator<Item = String>>,
        _rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult {
        CommandResult::new_with_stdout(
            std::env::current_dir()
                .context("pwd failed\n")
                .map(|dir| format!("{}\n", dir.display()))
                .unwrap_or("".to_string()),
        )
    }
}
