use super::prelude::*;
/// Exit命令处理器
pub struct ExitCommand;

impl Builtin for ExitCommand {
    fn execute(
        &self,
        _params: Box<dyn Iterator<Item = String>>,
        rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult {
        match crate::history::write_history_file(rl) {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(_) => {
                std::process::exit(0);
            }
        }
    }
}
