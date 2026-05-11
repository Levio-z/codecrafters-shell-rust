use super::prelude::*;
use crate::GLOBAL_EDITOR;
/// Exit命令处理器
pub struct ExitCommand;

impl Builtin for ExitCommand {
    fn execute(
        &self,
        _params: Vec<String>,
        _context: &mut ExecutionContext,
    ) -> BuiltinCommandResult {
        let mut rl = GLOBAL_EDITOR.lock().unwrap();

        match crate::history::write_history_file(&mut rl) {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(_) => {
                std::process::exit(0);
            }
        }
    }
}
