use crate::parse::ExecutionContext;
mod cd_command;
mod complete_command;
mod echo_command;
mod exit_command;
mod history_command;
mod jobs_command;
mod prelude;
mod pwd_command;
mod type_command;
pub use cd_command::CdCommand;
pub use complete_command::{CompleteCommand, GLOBAL_COMPLETION_MANAGER};
pub use echo_command::EchoCommand;
pub use exit_command::ExitCommand;
pub use history_command::HistoryCommand;
pub use jobs_command::{Job, JobList, JobsCommand};
pub use pwd_command::PwdCommand;
use strum::{AsRefStr, Display, EnumIter, EnumString};
pub use type_command::TypeCommand;
/// 内置命令接口
pub trait Builtin {
    fn execute(&self, params: Vec<String>, context: &mut ExecutionContext) -> BuiltinCommandResult;
}
#[derive(Debug, Clone, Copy, PartialEq, Display, EnumString, AsRefStr, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum BuiltinCommand {
    Exit,
    Pwd,
    Cd,
    Echo,
    Type,
    History,
    Jobs,
    Complete,
}

/// 表示一个命令执行结果
#[derive(Debug, Default)]
pub struct BuiltinCommandResult {
    pub stdout: Vec<u8>, // 标准输出
    #[allow(dead_code)]
    pub stderr: Vec<u8>, // 标准错误
    #[allow(dead_code)]
    pub exit_code: i32, // 退出码，0表示成功
}
impl BuiltinCommandResult {
    pub fn new_with_stdout(stdout: String) -> Self {
        Self {
            stdout: stdout.into_bytes(),
            ..Default::default()
        }
    }
    pub fn new_with_stderr(stderr: String) -> Self {
        Self {
            stderr: stderr.into_bytes(),
            exit_code: 1,
            ..Default::default()
        }
    }
}

/// 内置命令工厂
pub struct BuiltinFactory;

impl BuiltinFactory {
    pub fn create_command(command: &str) -> Option<Box<dyn Builtin>> {
        match command.parse::<BuiltinCommand>() {
            Ok(BuiltinCommand::Exit) => Some(Box::new(ExitCommand)),
            Ok(BuiltinCommand::Echo) => Some(Box::new(EchoCommand)),
            Ok(BuiltinCommand::Type) => Some(Box::new(TypeCommand)),
            Ok(BuiltinCommand::Pwd) => Some(Box::new(PwdCommand)),
            Ok(BuiltinCommand::Cd) => Some(Box::new(CdCommand)),
            Ok(BuiltinCommand::History) => Some(Box::new(HistoryCommand)),
            Ok(BuiltinCommand::Jobs) => Some(Box::new(JobsCommand)),
            Ok(BuiltinCommand::Complete) => Some(Box::new(CompleteCommand)),
            _ => None,
        }
    }
}
