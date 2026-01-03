#[allow(unused_imports)]
mod auto_completion;
mod builtin_commands;
mod command_handler;
mod history;
mod lexer;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{ChildStdout, Stdio},
    sync::LazyLock,
    vec,
};

use anyhow::Context;
use auto_completion::MyCompleter;
use command_handler::CommandHandlerFactory;
use is_executable::IsExecutable;
use rustyline::{
    Editor,
    config::{CompletionType, Config, Configurer},
    error::ReadlineError,
    history::{FileHistory, History},
};

pub static GLOBAL_VEC: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    let path = std::env::var("PATH").unwrap_or("".to_string());
    std::env::split_paths(&std::ffi::OsStr::new(&path)).collect::<Vec<_>>()
});
pub static HOME_DIR: LazyLock<String> =
    LazyLock::new(|| std::env::var("HOME").unwrap_or("".to_string()));
/// 表示一个命令行参数
#[derive(Debug, Clone)]
struct Token {
    content: Vec<String>,
    redirect: Option<String>,
    redirect_err: Option<String>,
    append_out: bool,
    append_err: bool,
}
impl Token {
    fn new() -> Self {
        Self {
            content: vec![],
            redirect: None,
            redirect_err: None,
            append_out: false,
            append_err: false,
        }
    }
    fn add_content(&mut self, content: String) {
        self.content.push(content);
    }
    fn set_redirect(&mut self, redirect: String, append: bool) -> anyhow::Result<()> {
        create_or_truncate_file(&redirect, append)?;
        self.redirect = Some(redirect);
        self.append_out = append;
        Ok(())
    }
    fn set_redirect_err(&mut self, redirect_err: String, append: bool) -> anyhow::Result<()> {
        create_or_truncate_file(&redirect_err, append)?;
        self.redirect_err = Some(redirect_err);
        self.append_err = append;
        Ok(())
    }
}

fn create_or_truncate_file(path: &str, append: bool) -> anyhow::Result<()> {
    OpenOptions::new()
        .truncate(!append)
        .create(true)
        .write(true)
        .open(path)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let config = Config::builder()
        .history_ignore_dups(false)?
        .completion_type(CompletionType::List) // 多候选时列出
        .bell_style(rustyline::config::BellStyle::Audible)               // 歧义时响铃
        .build();

    let completer = MyCompleter;
    let mut rl = Editor::with_config(config)?;
    rl.set_completion_type(rustyline::CompletionType::List);
    rl.set_helper(Some(completer));
    history::read_history_file(&mut rl)?;
    loop {
        match rl.readline("$ ") {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                parse_and_handle_line(&line, &mut rl)?;
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

fn parse_and_handle_line(
    line: &str,
    rl: &mut Editor<MyCompleter, FileHistory>,
) -> anyhow::Result<()> {
    let line_trim = line.trim();
    let tokens = split_quotes(line_trim);
    let iter = tokens.into_iter();
    let len = iter.len();
    let std_in = Stdio::null();
    let mut std_out = std::io::stdout();
    let mut std_err = std::io::stderr();
    let mut child_stdins = vec![];
    child_stdins.push(std_in);
    for (i, token) in iter.enumerate() {
        let token_redirect_err = token.redirect_err.as_deref();
        let token_redirect_out = token.redirect.as_deref();

        let last_result = handle(
            child_stdins.pop(),
            Box::new(token.content.into_iter()),
            i == len - 1,
            handle_redirect_file(token.append_out, token_redirect_out)?,
            handle_redirect_file(token.append_err, token_redirect_err)?,
            rl,
        );

        if !last_result.stderr.is_empty() {
            if token_redirect_err.is_none() {
                std_err.write_all(&last_result.stderr)?;
            } else {
                handle_redirect_file(token.append_err, token_redirect_err)?
                    .as_mut()
                    .unwrap()
                    .write_all(&last_result.stderr)?;
            }
        }
        if token_redirect_out.is_none() {
            if let Some(stdout) = last_result.stdout_stdio {
                child_stdins.push(Stdio::from(stdout));
            } else if i != len - 1 {
                let (read_pipe, mut write_pipe) = os_pipe::pipe()?;
                write_pipe.write_all(&last_result.stdout)?;
                drop(write_pipe);
                child_stdins.push(Stdio::from(read_pipe));
            } else if !last_result.stdout.is_empty() {
                std_out.write_all(&last_result.stdout)?;
            }
        } else if !last_result.stdout.is_empty() {
            handle_redirect_file(token.append_out, token_redirect_out)?
                .as_mut()
                .unwrap()
                .write_all(&last_result.stdout)?;
        }
    }
    Ok(())
}

fn handle_redirect_file(append: bool, redirect_file: Option<&str>) -> anyhow::Result<Option<File>> {
    let f = if let Some(redirect_file) = redirect_file {
        Some(
            OpenOptions::new()
                .append(append)
                .write(true)
                .open(redirect_file)?,
        )
    } else {
        None
    };
    Ok(f)
}
/// 表示一个命令执行结果
#[derive(Debug, Default)]
pub struct CommandResult {
    stdout: Vec<u8>, // 标准输出
    #[allow(dead_code)]
    stderr: Vec<u8>, // 标准错误
    #[allow(dead_code)]
    exit_code: i32, // 退出码，0表示成功
    stdout_stdio: Option<ChildStdout>,
}

impl CommandResult {
    fn new_with_stdout(stdout: String) -> Self {
        Self {
            stdout: stdout.into_bytes(),
            ..Default::default()
        }
    }
    fn new_with_stderr(stderr: String) -> Self {
        Self {
            stderr: stderr.into_bytes(),
            exit_code: 1,
            ..Default::default()
        }
    }
}

fn handle(
    std_in: Option<Stdio>,
    mut params: impl Iterator<Item = String> + 'static,
    last: bool,
    redirect_out: Option<File>,
    redirect_err: Option<File>,
    rl: &mut Editor<MyCompleter, FileHistory>,
) -> CommandResult {
    let command = params.next().context("command is empty");
    let command = match command {
        Ok(command) => command,
        Err(e) => {
            return CommandResult::new_with_stderr(e.to_string());
        }
    };

    // 使用命令处理器工厂创建适当的处理器
    let handler = CommandHandlerFactory::create_handler(&command);

    // 执行命令
    handler.execute(
        &command,
        std_in,
        Box::new(params),
        last,
        redirect_out,
        redirect_err,
        rl,
    )
}

fn find_executable_file_in_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() && path.is_executable() {
        return Some(path.to_path_buf());
    }
    None
}

pub fn print_iter(history: &FileHistory) -> impl Iterator<Item = String> {
    history
        .iter()
        .enumerate()
        .map(|(i, s)| format!("    {}  {s}\n", i + 1))
}
fn find_executable_file_in_paths(executable_file: &str, paths: &Vec<PathBuf>) -> Option<PathBuf> {
    for path in paths {
        if (path.exists() || path.is_dir())
            && let Some(file_path) = find_executable_file_in_path(&path.join(executable_file))
        {
            return Some(file_path);
        }
    }
    None
}

use std::{fs, result};

fn find_all_executable_file_in_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| path.exists() && path.is_dir())
        .flat_map(|dir| {
            fs::read_dir(dir)
                .map(|rd| {
                    Box::new(
                        rd.filter_map(|entry| entry.ok())
                            .filter_map(|entry| find_executable_file_in_path(&entry.path())),
                    ) as Box<dyn Iterator<Item = PathBuf>>
                })
                .unwrap_or_else(|_| Box::new(std::iter::empty()))
        })
        .collect()
}

fn split_quotes(line: &str) -> Vec<Token> {
    // 使用新的词法分析器
    let raw_tokens = lexer::tokenize_line(line);
    lexer::raw_tokens_to_tokens(raw_tokens)
}

#[test]
fn test_split_quotes() {
    let line = "echo 'test     script' 'hello''example' shell''world";
    let params = split_quotes(line).get(0).unwrap().content.clone();
    assert_eq!(
        params,
        vec!["echo", "test     script", "helloexample", "shellworld"]
    );

    let line = "echo world     test";
    let params = split_quotes(line).get(0).unwrap().content.clone();
    assert_eq!(params, vec!["echo", "world", "test"]);
}
