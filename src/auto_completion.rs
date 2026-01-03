use std::{borrow::Cow, sync::LazyLock};

use radix_trie::{Trie, TrieCommon};
use rustyline::{
    Changeset, Context, Helper,
    completion::{Completer, Pair},
    error::ReadlineError,
    highlight::Highlighter,
    hint::Hinter,
    line_buffer::LineBuffer,
    validate::{ValidationContext, ValidationResult, Validator},
};

use crate::{GLOBAL_VEC, builtin_commands::BuiltinCommand, find_all_executable_file_in_paths};
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
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let _start = 0; // 从行首开始补全
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
    fn update(&self, line: &mut LineBuffer, start: usize, elected: &str, cl: &mut Changeset) {
        let end = line.pos();
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
