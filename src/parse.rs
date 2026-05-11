use std::fs::File;

use regex::Regex;

use crate::{
    builtin_commands::{GLOBAL_COMPLETION_DECLARE, Job},
    executor::CommandResult,
    lexer::{RawToken, RedirectOp},
};
#[derive(Debug, Clone)]
pub struct Command {
    pub argv: Vec<String>,
    pub redirections: Vec<Redirection>, // 有序，决定语义
}

#[derive(Debug, Clone)]
pub struct Redirection {
    pub src_fd: Option<u8>, // None = 默认 fd（>, <）
    pub op: RedirectOp,
    pub target: RedirectTarget,
}

#[derive(Debug, Clone)]
pub enum RedirectTarget {
    File(String), // > file
    Fd(u8),       // 2>&1
    Close,        // 2>&-
    #[allow(dead_code)]
    Heredoc(String),
}

#[derive(Debug, Clone)]
pub struct CommandGroup {
    pub commands: Vec<Command>,
    pub background: bool,
}

pub fn parse_command(tokens: &[RawToken]) -> Vec<CommandGroup> {
    let mut command_groups = Vec::new();
    let mut commands = Vec::new();
    let mut current_tokens = Vec::new();

    for token in tokens {
        match token {
            RawToken::Pipe => {
                if !current_tokens.is_empty() {
                    commands.push(parse_simple_command(&current_tokens));
                    current_tokens.clear();
                }
            }
            RawToken::Background => {
                if !current_tokens.is_empty() {
                    commands.push(parse_simple_command(&current_tokens));
                    current_tokens.clear();
                }
                command_groups.push(CommandGroup {
                    commands: std::mem::take(&mut commands),
                    background: true,
                });
            }
            _ => {
                current_tokens.push(token.clone());
            }
        }
    }

    // 处理最后一个命令
    if !current_tokens.is_empty() {
        commands.push(parse_simple_command(&current_tokens));
        command_groups.push(CommandGroup {
            commands,
            background: false,
        });
    }

    command_groups
}

pub fn parse_simple_command(tokens: &[RawToken]) -> Command {
    let mut argv = Vec::new();
    let mut redirections = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            RawToken::Word(w) => {
                let expanded = expand(w);
                if !expanded.is_empty() {
                    argv.push(expanded);
                }

                i += 1;
            }

            RawToken::IoNumber(fd) => {
                let src_fd = Some(*fd);

                match tokens.get(i + 1) {
                    Some(RawToken::Redirect(op)) => {
                        let target = parse_redirect_target(&tokens[i + 2]);
                        redirections.push(Redirection {
                            src_fd,
                            op: *op,
                            target,
                        });
                        i += 3;
                    }
                    _ => panic!("io number not followed by redirect"),
                }
            }

            RawToken::Redirect(op) => {
                let src_fd = None;
                let target = parse_redirect_target(&tokens[i + 1]);

                redirections.push(Redirection {
                    src_fd,
                    op: *op,
                    target,
                });
                i += 2;
            }

            _ => panic!("unexpected token"),
        }
    }

    Command { argv, redirections }
}

fn parse_redirect_target(token: &RawToken) -> RedirectTarget {
    match token {
        RawToken::Word(w) if w == "-" => RedirectTarget::Close,
        RawToken::Word(w) => {
            if let Ok(fd) = w.parse::<u8>() {
                RedirectTarget::Fd(fd)
            } else {
                RedirectTarget::File(w.clone())
            }
        }
        _ => panic!("invalid redirect target"),
    }
}

use std::os::unix::io::FromRawFd;
/// 命令执行上下文
#[derive(Debug)]
pub struct ExecutionContext {
    pub stdin: Option<File>,
    pub stdout: Option<File>,
    pub stderr: Option<File>,
    pub background: bool,
    pub job: Option<usize>,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self {
            stdin: Some(unsafe { File::from_raw_fd(libc::dup(0)) }),
            stdout: Some(unsafe { File::from_raw_fd(libc::dup(1)) }),
            stderr: Some(unsafe { File::from_raw_fd(libc::dup(2)) }),
            background: false,
            job: None,
        }
    }
}
fn exit_code_by_child(child: Option<std::process::Child>) -> i32 {
    child.map_or(0, |mut c| c.wait().ok().and_then(|e| e.code()).unwrap_or(1))
}
pub fn excuete_single_command(
    command: &Command,
    context: &mut ExecutionContext,
) -> anyhow::Result<CommandResult> {
    let mut res = execute_command(command, context)?;
    if context.background {
        return Ok(CommandResult::default());
    }
    Ok(CommandResult::new(exit_code_by_child(res.child.take())))
}
/// 执行命令
pub fn execute_command(
    command: &Command,
    context: &mut ExecutionContext,
) -> anyhow::Result<CommandResult> {
    // 处理重定向
    apply_redirections(command, context)?;

    // 执行命令
    if command.argv.is_empty() {
        return Ok(CommandResult::default());
    }

    let command_name = &command.argv[0];
    let args = command.argv[1..].to_vec();

    // 使用简化的命令处理器
    let handler = crate::CommandHandlerFactory::create_handler(command_name);

    let job = if context.background {
        let mut arg = command.argv.clone().join(" ");
        arg.push(' ');
        Some(Job::run(arg))
    } else {
        None
    };
    context.job = job;

    let result = handler.execute(command_name, args.clone(), context);
    Ok(result)
}

/// 应用重定向
fn apply_redirections(command: &Command, context: &mut ExecutionContext) -> anyhow::Result<()> {
    for redirection in &command.redirections {
        match redirection.op {
            RedirectOp::Out | RedirectOp::OutAppend => {
                let fd = redirection.src_fd.unwrap_or(1); // 默认stdout
                if fd == 1 {
                    if let RedirectTarget::File(filename) = &redirection.target {
                        let file = File::options()
                            .write(true)
                            .create(true)
                            .append(redirection.op == RedirectOp::OutAppend)
                            .open(filename)?;
                        context.stdout = Some(file);
                    }
                } else if fd == 2
                    && let RedirectTarget::File(filename) = &redirection.target
                {
                    let file = File::options()
                        .write(true)
                        .create(true)
                        .append(redirection.op == RedirectOp::OutAppend)
                        .open(filename)?;
                    context.stderr = Some(file);
                }
            }
            RedirectOp::In => {
                let fd = redirection.src_fd.unwrap_or(0); // 默认stdin
                if fd == 0
                    && let RedirectTarget::File(filename) = &redirection.target
                {
                    let file = File::open(filename)?;
                    context.stdin = Some(file);
                }
            }
            RedirectOp::DupOut => {
                // 处理文件描述符复制：2>&1
                if let (Some(src_fd), RedirectTarget::Fd(target_fd)) =
                    (redirection.src_fd, &redirection.target)
                {
                    // 这里需要更复杂的文件描述符复制逻辑
                    // 简化实现：如果是stdout重定向到stderr或反之
                    if src_fd == 2 && *target_fd == 1 {
                        // stderr重定向到stdout
                        context.stderr = context.stdout.take();
                    } else if src_fd == 1 && *target_fd == 2 {
                        // stdout重定向到stderr
                        context.stdout = context.stderr.take();
                    }
                }
            }
            RedirectOp::Heredoc => {
                // 处理heredoc重定向
                if let RedirectTarget::Heredoc(_content) = &redirection.target {
                    // // 创建临时文件或管道来传递heredoc内容
                    // // 简化实现：使用临时文件
                    // let temp_file = tempfile::NamedTempFile::new()?;
                    // std::fs::write(temp_file.path(), content)?;
                    // let file = File::open(temp_file.path())?;
                    // context.stdin = Some(file.as_raw_fd());
                }
            }
            _ => {
                // 其他重定向操作符的实现在此省略，可根据需要添加
            }
        }
    }
    Ok(())
}

/// 执行管道命令
pub fn execute_pipeline(
    commands: &[Command],
    context: &mut ExecutionContext,
) -> anyhow::Result<CommandResult> {
    if commands.is_empty() {
        return Ok(CommandResult::default());
    }

    let mut vec = vec![];

    for (i, command) in commands.iter().enumerate() {
        let is_last = i == commands.len() - 1;
        // 设置管道
        if !is_last {
            let mut fds = [0; 2];
            unsafe { libc::pipe(fds.as_mut_ptr()) };

            let reader = unsafe { File::from_raw_fd(fds[0]) };
            let writer = unsafe { File::from_raw_fd(fds[1]) };

            context.stdout = Some(writer);

            // 执行当前命令
            let mut command_context = ExecutionContext {
                stdin: context.stdin.take(),
                stdout: context.stdout.take(),
                stderr: context.stderr.take(),
                background: false,
                job: None,
            };
            let result = execute_command(command, &mut command_context)?;

            context.stdin = Some(reader);
            context.stdout = Some(unsafe { File::from_raw_fd(libc::dup(1)) });
            context.stderr = Some(unsafe { File::from_raw_fd(libc::dup(2)) });

            vec.push(result);
        } else {
            // 最后一个命令
            let result = execute_command(command, context)?;

            vec.push(result);
        }
    }
    let mut last_exit_code = 0;
    for result in vec {
        if let Some(mut child) = result.child {
            let status = child.wait()?;
            last_exit_code = status.code().unwrap_or(1);
        } else {
            last_exit_code = 0;
        }
    }
    Ok(CommandResult::new(last_exit_code))
}

fn expand(input: &str) -> String {
    let env = &GLOBAL_COMPLETION_DECLARE.lock().unwrap().completions;

    let re = Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}|\$([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

    re.replace_all(input, |caps: &regex::Captures| {
        // ${VAR}
        let name = caps
            .get(1)
            .or_else(|| caps.get(2))
            .map(|m| m.as_str())
            .unwrap_or("");

        env.get(name).cloned().unwrap_or_default()
    })
    .to_string()
}
