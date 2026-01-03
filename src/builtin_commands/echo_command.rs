use super::prelude::*;
/// Echo命令处理器
pub struct EchoCommand;

impl Builtin for EchoCommand {
    fn execute(
        &self,
        params: Box<dyn Iterator<Item = String>>,
        _rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult {
        CommandResult::new_with_stdout(format!("{}\n", params.collect::<Vec<_>>().join(" ")))
    }
}
