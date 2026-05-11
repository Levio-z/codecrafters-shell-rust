mod builtin_command_handler;
mod external_command_handler;
pub mod prelude;

use crate::{
    builtin_commands::BuiltinCommand,
    builtin_commands::BuiltinCommandResult,
    parse::ExecutionContext, // 添加ExecutionContext导入
};
/// 简化的命令处理器接口
pub trait CommandHandler {
    fn execute(
        &self,
        command: &str,
        args: Vec<String>,
        context: &mut ExecutionContext,
    ) -> CommandResult;
}

use builtin_command_handler::BuiltinCommandHandler;
use external_command_handler::ExternalCommandHandler;
/// 命令处理器工厂
pub struct CommandHandlerFactory;

impl CommandHandlerFactory {
    pub fn create_handler(command: &str) -> Box<dyn CommandHandler + 'static> {
        match command.parse::<BuiltinCommand>() {
            Ok(_) => Box::new(BuiltinCommandHandler),
            Err(_) => Box::new(ExternalCommandHandler),
        }
    }
}

/// 表示一个命令执行结果
#[derive(Debug)]
pub struct CommandResult {
    #[allow(dead_code)]
    pub exit_code: i32, // 退出码，0表示成功
    pub child: Option<std::process::Child>,
    pub pid: Option<u32>,
}
impl Default for CommandResult {
    fn default() -> Self {
        Self::new(0)
    }
}
impl CommandResult {
    pub fn new(exit_code: i32) -> Self {
        Self {
            exit_code,
            child: None,
            pid: None,
        }
    }
    pub fn external_with_child(child: std::process::Child) -> Self {
        Self {
            exit_code: 0,
            pid: Some(child.id()),
            child: Some(child),
        }
    }
}

impl From<BuiltinCommandResult> for CommandResult {
    fn from(value: BuiltinCommandResult) -> Self {
        Self {
            exit_code: value.exit_code,
            child: None,
            pid: None,
        }
    }
}
