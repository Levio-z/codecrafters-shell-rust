/// 外部命令处理器
use super::prelude::*;
use crate::GLOBAL_JOB;

pub struct ExternalCommandHandler;

impl CommandHandler for ExternalCommandHandler {
    fn execute(
        &self,
        command: &str,
        args: Vec<String>,
        context: &mut ExecutionContext,
    ) -> CommandResult {
        match crate::utils::find_executable_file_in_paths(command, &crate::GLOBAL_VEC) {
            Some(file_path) => {
                let file_name = file_path.file_name().context("file name is empty");
                if file_name.is_err() {
                    eprintln!("{}: file name is empty\n", command);
                    return CommandResult::new(1);
                }
                let mut cmd = std::process::Command::new(file_name.unwrap());
                cmd.args(args);
                // 应用标准输入输出重定向
                if let Some(stdin) = context.stdin.take() {
                    cmd.stdin(Stdio::from(stdin));
                }
                if let Some(stdout) = context.stdout.take() {
                    cmd.stdout(Stdio::from(stdout));
                }
                if let Some(stderr) = context.stderr.take() {
                    cmd.stderr(Stdio::from(stderr));
                }

                let child = cmd.spawn().context("spawn command failed");

                match child {
                    Ok(child) => {
                        if let Some(id) = &context.job {
                            println!("[{}] {}", id, child.id());
                            let mut joblist = GLOBAL_JOB.lock().unwrap();
                            joblist.update_pid(*id, child.id(), child);
                            return CommandResult::new(1);
                        }
                        CommandResult::external_with_child(child)
                    }
                    Err(e) => {
                        eprintln!("{}: spawn command failed: {:?}\n", command, e);
                        CommandResult::new(1)
                    }
                }
            }
            None => {
                eprintln!("{}: command not found", command);
                CommandResult::new(1)
            }
        }
    }
}
