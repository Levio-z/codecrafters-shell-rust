#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
    vec,
};
mod auto_completion;
use anyhow::Context;
use auto_completion::MyCompleter;
use is_executable::IsExecutable;
use rustyline::{Editor, config::Configurer, error::ReadlineError,config::{Config, CompletionType}};
use strum::{AsRefStr, Display, EnumIter, EnumString};
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
        Ok(())
    }
    fn set_redirect_err(&mut self, redirect_err: String, append: bool) -> anyhow::Result<()> {
        create_or_truncate_file(&redirect_err, append)?;
        self.redirect_err = Some(redirect_err);
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
    .completion_type(CompletionType::List) // 多候选时列出
    .bell_style(rustyline::config::BellStyle::Audible)               // 歧义时响铃
    .build();

    let completer = MyCompleter;
    let mut rl = Editor::with_config(config)?;
    rl.set_completion_type(rustyline::CompletionType::List);
    rl.set_helper(Some(completer));
    loop {
        match rl.readline("$ ") {
            Ok(line) => {
                parse_and_handle_line(&line)?;
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

fn parse_and_handle_line(line: &str) -> anyhow::Result<()> {
    let line_trim = line.trim();
    let tokens = split_quotes(line_trim);
    let iter = tokens.into_iter();
    for token in iter {
        let last_result = handle(token.content.into_iter());

        match token.redirect_err {
            None => {
                if !last_result.stderr.is_empty() {
                    eprint!("{}", last_result.stderr);
                }
            }
            Some(redirect_err) => {
                let mut f = OpenOptions::new()
                    .append(token.append_err)
                    .write(true)
                    .open(redirect_err)?;
                std::io::Write::write_all(&mut f, last_result.stderr.as_bytes())
                    .context("write file failed")?;
            }
        }
        match token.redirect {
            None => {
                if !last_result.stdout.is_empty() {
                    print!("{}", last_result.stdout);
                }
            }
            Some(redirect) => {
                let mut f = OpenOptions::new()
                    .append(token.append_out)
                    .write(true)
                    .open(redirect)?;
                std::io::Write::write_all(&mut f, last_result.stdout.as_bytes())
                    .context("write file failed")?;
            }
        }
    }
    Ok(())
}
/// 表示一个命令执行结果
#[derive(Debug, Clone)]
struct CommandResult {
    stdout: String, // 标准输出
    #[allow(dead_code)]
    stderr: String, // 标准错误
    #[allow(dead_code)]
    exit_code: i32, // 退出码，0表示成功
}
impl Default for CommandResult {
    fn default() -> Self {
        Self {
            stdout: "".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
        }
    }
}
impl CommandResult {
    fn new_with_stdout(stdout: String) -> Self {
        Self {
            stdout,
            ..Default::default()
        }
    }
    fn new_with_stderr(stderr: String) -> Self {
        Self {
            stderr,
            exit_code: 1,
            ..Default::default()
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Display, EnumString, AsRefStr, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum BuildinCommand {
    Exit,
    Pwd,
    Cd,
    Echo,
    Type,
}

fn handle(mut params: impl Iterator<Item = String>) -> CommandResult {
    let command = params.next().context("command is empty");
    let command = match command {
        Ok(command) => command,
        Err(e) => {
            return CommandResult::new_with_stderr(e.to_string());
        }
    };

    match command.parse::<BuildinCommand>() {
        Ok(BuildinCommand::Exit) => std::process::exit(0),
        Ok(BuildinCommand::Echo) => {
            CommandResult::new_with_stdout(format!("{}\n", params.collect::<Vec<_>>().join(" ")))
        }
        Ok(BuildinCommand::Type) => {
            let command_type = params.next().context("type command is empty");
            let command_type = match command_type {
                Ok(command_type) => command_type,
                Err(e) => return CommandResult::new_with_stderr(e.to_string()),
            };
            // TODO:使用 enum 优化
            match command_type.parse::<BuildinCommand>() {
                Ok(_) => {
                    CommandResult::new_with_stdout(format!("{} is a shell builtin\n", command_type))
                }
                _ => match find_executable_file_in_paths(&command_type, &GLOBAL_VEC) {
                    Some(file_path) => CommandResult::new_with_stdout(format!(
                        "{} is {}\n",
                        command_type,
                        file_path.display()
                    )),
                    None => CommandResult::new_with_stderr(format!("{command_type}: not found\n")),
                },
            }
        }
        Ok(BuildinCommand::Pwd) => CommandResult::new_with_stdout(
            std::env::current_dir()
                .context("pwd failed\n")
                .map(|dir| format!("{}\n", dir.display()))
                .unwrap_or("".to_string()),
        ),
        Ok(BuildinCommand::Cd) => {
            let dir = params.next().context("cd command is empty\n");
            let dir = match dir {
                Ok(dir) => dir,
                Err(_) => {
                    return CommandResult::new_with_stderr("cd: missing operand\n".to_string());
                }
            };

            if params.next().is_some() {
                CommandResult::new_with_stderr("bash: cd: too many arguments\n".to_string())
            } else {
                let dir = if dir == "~" { &HOME_DIR } else { &dir };
                match std::env::set_current_dir(dir).context("cd failed\n") {
                    Ok(_) => CommandResult::default(),

                    Err(_) => CommandResult::new_with_stderr(format!(
                        "cd: {}: No such file or directory\n",
                        dir
                    )),
                }
            }
        }
        _ => match find_executable_file_in_paths(&command, &GLOBAL_VEC) {
            Some(file_path) => {
                let file_name = file_path.file_name().context("file name is empty");
                if file_name.is_err() {
                    return CommandResult::new_with_stderr(format!(
                        "{}: file name is empty\n",
                        command
                    ));
                }
                Command::new(file_name.as_ref().unwrap())
                    .args(params)
                    .output()
                    .map(|output| CommandResult {
                        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                        exit_code: output.status.code().unwrap_or(1),
                    })
                    .unwrap_or_else(|_| {
                        CommandResult::new_with_stderr(format!("{}: failed to execute\n", command))
                    })
            }
            None => CommandResult::new_with_stderr(format!("{}: command not found\n", command)),
        },
    }
}

fn find_executable_file_in_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() && path.is_executable() {
        return Some(path.to_path_buf());
    }
    None
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

use std::fs;

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

enum MatchType {
    Default,
    DoubleQuote,
    SingleQuote,
    Escaping,
    DoubleQuoteEscaping,
}

fn split_quotes(line: &str) -> Vec<Token> {
    let mut token = Token::new();
    let mut match_type = MatchType::Default;
    let mut string = String::new();
    let mut res = Vec::new();
    let mut redirect = false;
    let mut redirect_err = false;
    for ch in line.chars() {
        match match_type {
            MatchType::Default => match ch {
                ch if ch.is_whitespace() => {
                    if !string.is_empty() {
                        if redirect {
                            let _ = token.set_redirect(string.clone(), token.append_out);
                            redirect = false;
                        } else if redirect_err {
                            let _ = token.set_redirect_err(string.clone(), token.append_err);
                            redirect_err = false;
                        } else {
                            token.add_content(string.clone());
                        }
                        string = String::new();
                    }
                    continue;
                }
                '\'' => match_type = MatchType::SingleQuote,
                '"' => match_type = MatchType::DoubleQuote,
                '\\' => match_type = MatchType::Escaping,
                '>' => {
                    if redirect {
                        token.append_out = true;
                        continue;
                    } else if redirect_err {
                        token.append_err = true;
                        continue;
                    }

                    if !string.is_empty() {
                        if string.ends_with("1") {
                            string.pop();
                            redirect = true;
                            token.append_out = false;
                        } else if string.ends_with("2") {
                            string.pop();
                            redirect_err = true;
                            token.append_err = false;
                        }
                    } else {
                        redirect = true;
                        token.append_out = false;
                    }
                    if !string.is_empty() {
                        token.add_content(string.clone());
                    }
                    string = String::new();
                }
                _ => string.push(ch),
            },
            MatchType::SingleQuote => match ch {
                '\'' => match_type = MatchType::Default,
                _ => string.push(ch),
            },
            MatchType::DoubleQuote => match ch {
                '"' => match_type = MatchType::Default,
                '\\' => match_type = MatchType::DoubleQuoteEscaping,
                _ => string.push(ch),
            },
            MatchType::DoubleQuoteEscaping => match ch {
                '"' => {
                    string.push(ch);
                    match_type = MatchType::DoubleQuote;
                }
                '\\' => {
                    string.push(ch);
                    match_type = MatchType::DoubleQuote;
                }
                _ => {
                    string.push('\\');
                    string.push(ch);
                    match_type = MatchType::DoubleQuote;
                }
            },
            MatchType::Escaping => {
                string.push(ch);
                match_type = MatchType::Default;
            }
        }
    }

    if redirect {
        let _ = token.set_redirect(string.clone(), token.append_out);
    } else if redirect_err {
        let _ = token.set_redirect_err(string.clone(), token.append_err);
    } else {
        token.add_content(string.clone());
    }
    res.push(token);
    res
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
