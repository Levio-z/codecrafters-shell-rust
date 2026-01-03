use std::{fs::File, process::Stdio};

use anyhow::Context;
use rustyline::{Editor, history::FileHistory};

use crate::{
    CommandResult,
    auto_completion::MyCompleter,
    builtin_commands::{BuiltinCommand, BuiltinFactory},
};
/// 命令处理器接口
pub trait CommandHandler {
    fn execute(
        &self,
        command: &str,
        std_in: Option<Stdio>,
        params: Box<dyn Iterator<Item = String>>,
        last: bool,
        redirect_out: Option<File>,
        redirect_err: Option<File>,
        rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult;
}

/// 内置命令处理器
pub struct BuiltinCommandHandler;

impl CommandHandler for BuiltinCommandHandler {
    fn execute(
        &self,
        command: &str,
        _std_in: Option<Stdio>,
        mut params: Box<dyn Iterator<Item = String>>,
        _last: bool,
        _redirect_out: Option<File>,
        _redirect_err: Option<File>,
        rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult {
        if let Some(cmd) = BuiltinFactory::create_command(command) {
            cmd.execute(params, rl)
        } else {
            CommandResult::new_with_stderr(format!("{}: command not found\n", command))
        }
    }
}

/// 外部命令处理器
pub struct ExternalCommandHandler;

impl CommandHandler for ExternalCommandHandler {
    fn execute(
        &self,
        command: &str,
        std_in: Option<Stdio>,
        mut params: Box<dyn Iterator<Item = String> + 'static>,
        last: bool,
        redirect_out: Option<File>,
        redirect_err: Option<File>,
        _rl: &mut Editor<MyCompleter, FileHistory>,
    ) -> CommandResult {
        match crate::find_executable_file_in_paths(command, &crate::GLOBAL_VEC) {
            Some(file_path) => {
                let file_name = file_path.file_name().context("file name is empty");
                if file_name.is_err() {
                    return CommandResult::new_with_stderr(format!(
                        "{}: file name is empty\n",
                        command
                    ));
                }
                if last {
                    std::process::Command::new(file_name.as_ref().unwrap())
                        .args(params)
                        .stdin(std_in.unwrap_or(Stdio::null()))
                        .stdout(if let Some(redirect_out) = redirect_out {
                            Stdio::from(redirect_out)
                        } else {
                            Stdio::inherit()
                        })
                        .stderr(if let Some(redirect_err) = redirect_err {
                            Stdio::from(redirect_err)
                        } else {
                            Stdio::inherit()
                        })
                        .output()
                        .map(|output| CommandResult {
                            stdout: Vec::new(),
                            stderr: Vec::new(),
                            exit_code: output.status.code().unwrap_or(1),
                            stdout_stdio: None,
                        })
                        .unwrap_or_else(|_| {
                            CommandResult::new_with_stderr(format!(
                                "{}: failed to execute\n",
                                command
                            ))
                        })
                } else {
                    let is_redirect_out = redirect_out.is_some();
                    std::process::Command::new(file_name.as_ref().unwrap())
                        .args(params)
                        .stdin(std_in.unwrap_or(Stdio::null()))
                        .stdout(if let Some(redirect_out) = redirect_out {
                            Stdio::from(redirect_out)
                        } else {
                            Stdio::piped()
                        })
                        .stderr(if let Some(redirect_err) = redirect_err {
                            Stdio::from(redirect_err)
                        } else {
                            Stdio::inherit()
                        })
                        .spawn()
                        .map(|mut child| {
                            let child_stdin = child
                                .stdout
                                .take()
                                .expect("Failed to open child stdin pipe");
                            CommandResult {
                                stdout: Vec::new(),
                                stderr: Vec::new(),
                                exit_code: 0,
                                stdout_stdio: if is_redirect_out {
                                    None
                                } else {
                                    Some(child_stdin)
                                },
                            }
                        })
                        .unwrap_or_else(|_| {
                            CommandResult::new_with_stderr(format!(
                                "{}: failed to execute\n",
                                command
                            ))
                        })
                }
            }
            None => CommandResult::new_with_stderr(format!("{}: command not found\n", command)),
        }
    }
}

/// 命令处理器工厂
pub struct CommandHandlerFactory;

impl CommandHandlerFactory {
    pub fn create_handler(command: &str) -> Box<dyn CommandHandler> {
        match command.parse::<BuiltinCommand>() {
            Ok(_) => Box::new(BuiltinCommandHandler),
            Err(_) => Box::new(ExternalCommandHandler),
        }
    }
}
