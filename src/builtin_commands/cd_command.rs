use super::prelude::*;
/// Cd命令处理器
pub struct CdCommand;

impl Builtin for CdCommand {
    fn execute(
        &self,
        mut params: Box<dyn Iterator<Item = String>>,
        _rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult {
        let dir = params.next().context("cd command is empty\n");
        let dir = match dir {
            Ok(dir) => dir,
            Err(_) => {
                return CommandResult::new_with_stderr("cd: missing operand\n".to_string());
            }
        };

        if params.next().is_some() {
            CommandResult::new_with_stderr("bash: cd: too many arguments\n".to_string())
        } else {
            let dir = if dir == "~" { &crate::HOME_DIR } else { &dir };
            match std::env::set_current_dir(dir).context("cd failed\n") {
                Ok(_) => CommandResult::default(),
                Err(_) => CommandResult::new_with_stderr(format!(
                    "cd: {}: No such file or directory\n",
                    dir
                )),
            }
        }
    }
}
