#[allow(unused_imports)]
mod auto_completion;
mod builtin_commands;
mod executor;
mod history;
mod lexer;
mod parse;
mod utils;
use std::{
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex},
    thread,
};

use anyhow::Context;
use auto_completion::MyCompleter;
use executor::CommandHandlerFactory;
use rustyline::{
    DefaultEditor, Editor,
    config::{CompletionType, Config, Configurer},
    error::ReadlineError,
    history::FileHistory,
};

use crate::parse::{
    CommandGroup, ExecutionContext, excuete_single_command, execute_pipeline, parse_command,
};

pub static GLOBAL_VEC: LazyLock<Vec<PathBuf>> = LazyLock::new(|| {
    let path = std::env::var("PATH").unwrap_or("".to_string());
    std::env::split_paths(&std::ffi::OsStr::new(&path)).collect::<Vec<_>>()
});
pub static HOME_DIR: LazyLock<String> =
    LazyLock::new(|| std::env::var("HOME").unwrap_or("".to_string()));

static GLOBAL_EDITOR: LazyLock<Mutex<Editor<MyCompleter, FileHistory>>> = LazyLock::new(|| {
    let config = Config::builder()
        .history_ignore_dups(false)
        .unwrap()
        .completion_type(CompletionType::List)
        .bell_style(rustyline::config::BellStyle::Audible)
        .build();

    let completer = MyCompleter;
    let mut rl = Editor::with_config(config).unwrap();

    rl.set_helper(Some(completer));
    history::read_history_file(&mut rl);

    Mutex::new(rl)
});
fn main() -> anyhow::Result<()> {
    loop {
        let line = {
            let mut rl = GLOBAL_EDITOR.lock().unwrap();
            rl.readline("$ ")
        };

        match line {
            Ok(line) => {
                {
                    let mut rl = GLOBAL_EDITOR.lock().unwrap();
                    let _ = rl.add_history_entry(line.as_str());
                }
                let _ = parse_and_handle_line(&line);
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
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

    // 空行处理
    if line_trim.is_empty() {
        return Ok(());
    }

    // 词法分析
    let raw_tokens = crate::lexer::tokenize_line(line_trim)?;

    // 语法分析
    let command_type = parse_command(&raw_tokens);
    // 创建执行上下文
    let mut context = ExecutionContext::new();

    let execute_command = move |command_groups: &mut CommandGroup,
                                context: &mut ExecutionContext|
          -> anyhow::Result<()> {
        if command_groups.commands.len() > 1 {
            let _ = execute_pipeline(&command_groups.commands, context)?;
        } else {
            let _ = excuete_single_command(&command_groups.commands.remove(0), context)?;
        };
        Ok(())
    };

    for mut command_groups in command_type.into_iter() {
        // 将下面内容抽取成闭包
        if command_groups.background {
            context.background = true;
            execute_command(&mut command_groups, &mut context)?;
            context.background = false;
        } else {
            execute_command(&mut command_groups, &mut context)?;
        }
    }

    // // 执行命令
    //
    //     if command_groups.background {
    //         continue;
    //     }
    //     thread::spawn(move || {
    //         // 需要将下面这个if else 抽成闭包
    //         i
    //     });
    // }

    Ok(())
}
