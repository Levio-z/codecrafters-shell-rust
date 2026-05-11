use super::prelude::*;
/// Jobs命令处理器
pub struct JobsCommand;

impl Builtin for JobsCommand {
    fn execute(
        &self,
        params: Vec<String>,
        _context: &mut ExecutionContext,
    ) -> BuiltinCommandResult {
        BuiltinCommandResult::default()
    }
}
