use std::{fs::File, process::Stdio};

use anyhow::Context;
use rustyline::{
    Editor,
    history::{FileHistory, History},
};

use crate::{CommandResult, auto_completion::MyCompleter};
mod cd_command;
mod echo_command;
mod exit_command;
mod history_command;
mod prelude;
mod pwd_command;
mod type_command;
pub use cd_command::CdCommand;
pub use echo_command::EchoCommand;
pub use exit_command::ExitCommand;
pub use history_command::HistoryCommand;
pub use pwd_command::PwdCommand;
use strum::{AsRefStr, Display, EnumIter, EnumString};
pub use type_command::TypeCommand;
/// 内置命令接口
pub trait Builtin {
    fn execute(
        &self,
        params: Box<dyn Iterator<Item = String>>,
        rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult;
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
            _ => None,
        }
    }
}
