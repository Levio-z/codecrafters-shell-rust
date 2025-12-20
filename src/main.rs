#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::LazyLock,
};

use anyhow::Context;
use is_executable::IsExecutable;

static GLOBAL_VEC: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    let path = std::env::var("PATH").unwrap_or("".to_string());
    std::env::split_paths(&std::ffi::OsStr::new(&path)).collect::<Vec<_>>()
});

static HOME_DIR: LazyLock<String> =
    LazyLock::new(|| std::env::var("HOME").unwrap_or("".to_string()));

fn main() -> anyhow::Result<()> {
    'outer: loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .context("read line failed")?;

        let line_trim = line.trim();

        let mut params = line_trim.split_whitespace();
        let command = params.next().context("command is empty")?.trim();

        match command {
            "exit" => break 'outer,
            "echo" => println!(
                "{}",
                line_trim
                    .strip_prefix("echo ")
                    .context("echo command is empty")?
            ),
            "type" => {
                let command_type = params.next().context("type command is empty")?.trim();
                // TODO:使用 enum 优化
                match command_type {
                    "exit" | "echo" | "type" | "pwd" | "cd" => {
                        println!("{} is a shell builtin", command_type)
                    }
                    _ => match find_executable_file_in_paths(command_type, &GLOBAL_VEC) {
                        Some(file_path) => println!("{} is {}", command_type, file_path.display()),
                        None => println!("{command_type}: not found"),
                    },
                }
            }
            "pwd" => println!(
                "{}",
                std::env::current_dir().context("pwd failed")?.display()
            ),

            "cd" => {
                let dir = params.next().context("cd command is empty")?.trim();
                if params.next().is_some() {
                    println!("bash: cd: too many arguments");
                    break 'outer;
                }
                let dir = if dir == "~" { &HOME_DIR } else { dir };
                match std::env::set_current_dir(dir).context("cd failed") {
                    Ok(_) => {}
                    Err(_) => println!("cd: {}: No such file or directory", dir),
                }
            }
            _ => match find_executable_file_in_paths(command, &GLOBAL_VEC) {
                Some(file_path) => {
                    let _ = Command::new(file_path.file_name().context("file name is empty")?)
                        .args(params)
                        .status()
                        .context("failed to execute")?;
                }
                None => println!("{}: command not found", command),
            },
        }
    }
    Ok(())
}

fn find_executable_file_in_path(executable_file: &str, path: &Path) -> Option<PathBuf> {
    let file_path = path.join(executable_file);
    if file_path.is_file() && file_path.is_executable() {
        return Some(file_path);
    }
    None
}

fn find_executable_file_in_paths(executable_file: &str, paths: &Vec<PathBuf>) -> Option<PathBuf> {
    for path in paths {
        if (path.exists() || path.is_dir())
            && let Some(file_path) = find_executable_file_in_path(executable_file, path)
        {
            return Some(file_path);
        }
    }
    None
}
