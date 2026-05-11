use std::{borrow::Cow, sync::LazyLock};

use anyhow::Context;
use radix_trie::{Trie, TrieCommon};
use rustyline::{
    Changeset, Helper,
    completion::{Completer, Pair},
    error::ReadlineError,
    highlight::Highlighter,
    hint::Hinter,
    line_buffer::LineBuffer,
    validate::{ValidationContext, ValidationResult, Validator},
};

use crate::{
    GLOBAL_VEC,
    builtin_commands::{BuiltinCommand, GLOBAL_COMPLETION_MANAGER},
    utils::find_all_executable_file_in_paths,
};
pub struct MyCompleter;
use strum::IntoEnumIterator;

static GLOBAL_TRIES: LazyLock<Trie<String, ()>> = LazyLock::new(|| {
    let iter = BuiltinCommand::iter();
    let mut commands: Vec<String> = iter.map(|cmd| cmd.to_string()).collect();
    commands.extend(
        find_all_executable_file_in_paths(&GLOBAL_VEC)
            .iter()
            .filter_map(|path| {
                path.file_name()                // Option<&OsStr>
                .and_then(|name| name.to_str()) // Option<&str>
                .map(|s| s.to_string())
            }),
    );
    commands
        .into_iter()
        .map(|cmd| (cmd.to_string(), ()))
        .collect::<Trie<String, ()>>()
});

impl Completer for MyCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let _start = 0; // 从行首开始补全
        let last_whitespace = line.rfind(char::is_whitespace);
        match last_whitespace {
            Some(idx) => {
                if (idx + 1 == pos || !line[idx + 1..pos].is_empty())
                    && let Some((pairs, _len)) =
                        find_complete_and_executable_file(line, &line[0..idx], &line[idx + 1..pos])
                {
                    return Ok((idx + 1, pairs));
                }
                let (pairs, len) = find_completed_file(&line[idx + 1..pos])?;
                Ok((idx + len, pairs))
            }
            None => {
                let prefix = &line[..pos];
                let prefix_keys: Vec<Pair> = GLOBAL_TRIES
                    .get_raw_descendant(prefix)
                    .map(|trie| {
                        trie.keys()
                            .map(|k| Pair {
                                display: k.clone(),
                                replacement: k.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Ok((0, prefix_keys))
            }
        }
    }
    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str, cl: &mut Changeset) {
        let end: usize = line.pos();
        let elected = if let Some(sub_trie) = GLOBAL_TRIES.subtrie(elected)
            && sub_trie.is_leaf()
        {
            Cow::Owned(elected.to_string() + " ")
        } else {
            Cow::Borrowed(elected)
        };
        line.replace(start..end, &elected, cl);
    }
}

impl Helper for MyCompleter {} // 必须实现 Helper trait
impl Hinter for MyCompleter {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        None // 不提供提示
    }
}

impl Highlighter for MyCompleter {} // 空实现

impl Validator for MyCompleter {
    fn validate(&self, _ctx: &mut ValidationContext) -> Result<ValidationResult, ReadlineError> {
        Ok(ValidationResult::Valid(None)) // 始终认为输入合法
    }
}

fn find_completed_file(original: &str) -> Result<(Vec<Pair>, usize), ReadlineError> {
    let (path, line, pos) = if let Some((path, line)) = original.rsplit_once('/') {
        (path, line, path.len() + 2)
    } else {
        ("./", original, 1)
    };
    // 1、列出所有文件
    let dir: std::fs::ReadDir = std::fs::read_dir(path)?;
    // 2.匹配到符合的文件
    Ok({
        let mut pairs = dir
            .into_iter()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let mut file_name = entry.file_name().to_string_lossy().to_string();
                let file_type = entry.file_type().ok()?;
                if file_type.is_dir() {
                    file_name.push('/');
                }
                if file_type.is_file() {
                    file_name.push(' ');
                }
                if file_name.starts_with(line) {
                    Some(Pair {
                        display: file_name.clone(),
                        replacement: file_name,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        pairs.sort_by(|a, b| a.display.cmp(&b.display));
        (pairs, pos)
    })
}

fn find_complete_and_executable_file(
    env_line: &str,
    original: &str,
    word: &str,
) -> Option<(Vec<Pair>, usize)> {
    let completion_manager = GLOBAL_COMPLETION_MANAGER.lock().unwrap();
    let line = original.split(' ').collect::<Vec<_>>();
    let arg1 = line.first().map_or("", |v| v);
    let arg2 = line.last().map_or("", |v| v);
    if let Some(path) = completion_manager.completions.get(arg1) {
        let cmd = std::process::Command::new(path)
            .args([arg1, word, arg2])
            .env("COMP_LINE", env_line)
            .env("COMP_POINT", env_line.len().to_string())
            .output()
            .expect("failed to execute");

        let suggestions = String::from_utf8_lossy(&cmd.stdout).to_string();
        if suggestions.is_empty() {
            return None;
        }
        let suggestions = suggestions
            .split_whitespace()
            .map(|s| Pair {
                replacement: format!("{s} "),
                display: s.to_string(),
            })
            .collect::<Vec<_>>();

        return Some((suggestions, 0));
    }
    None
}
