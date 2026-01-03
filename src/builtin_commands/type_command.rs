use super::prelude::*;
/// Type命令处理器
pub struct TypeCommand;

impl Builtin for TypeCommand {
    fn execute(
        &self,
        mut params: Box<dyn Iterator<Item = String>>,
        _rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult {
        let command_type = params.next().context("type command is empty");
        let command_type = match command_type {
            Ok(command_type) => command_type,
            Err(e) => return CommandResult::new_with_stderr(e.to_string()),
        };

        match command_type.parse::<BuiltinCommand>() {
            Ok(_) => {
                CommandResult::new_with_stdout(format!("{} is a shell builtin\n", command_type))
            }
            _ => match crate::find_executable_file_in_paths(&command_type, &crate::GLOBAL_VEC) {
                Some(file_path) => CommandResult::new_with_stdout(format!(
                    "{} is {}\n",
                    command_type,
                    file_path.display()
                )),
                None => CommandResult::new_with_stderr(format!("{command_type}: not found\n")),
            },
        }
    }
}
