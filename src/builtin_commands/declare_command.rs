use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use super::prelude::*;
/// Declare命令处理器
pub struct DeclareCommand;
pub static GLOBAL_COMPLETION_DECLARE: LazyLock<Mutex<DeclareMap>> =
    LazyLock::new(|| Mutex::new(DeclareMap::new()));
impl Builtin for DeclareCommand {
    fn execute(
        &self,
        params: Vec<String>,
        _context: &mut ExecutionContext,
    ) -> BuiltinCommandResult {
        let mut args = params.into_iter();
        match args.next().as_deref() {
            Some("-p") => match args.next() {
                Some(cmd) => {
                    if let Some(value) = GLOBAL_COMPLETION_DECLARE
                        .lock()
                        .unwrap()
                        .completions
                        .get(cmd.as_str())
                        .cloned()
                    {
                        // declare -- foo="bar2"
                        BuiltinCommandResult::new_with_stdout(format!(
                            "declare -- {}=\"{}\"\n",
                            cmd, value
                        ))
                    } else {
                        BuiltinCommandResult::new_with_stdout(format!(
                            "declare: {}: not found\n",
                            cmd
                        ))
                    }
                }
                None => BuiltinCommandResult::new_with_stderr(
                    "declare: option requires an argument -- p\n".to_string(),
                ),
            },
            Some(declare) => {
                if let Some((key, value)) = declare.split_once("=") {
                    match is_valid_identifier(key) {
                        true => {
                            GLOBAL_COMPLETION_DECLARE
                                .lock()
                                .unwrap()
                                .completions
                                .insert(key.to_string(), value.to_string());
                            BuiltinCommandResult::default()
                        }
                        // declare: `67=x': not a valid identifier
                        false => BuiltinCommandResult::new_with_stderr(
                            format!("declare: `{}={}': not a valid identifier\n", key, value)
                                .to_string(),
                        ),
                    }
                } else {
                    BuiltinCommandResult::default()
                }
            }
            _ => BuiltinCommandResult::new_with_stderr(
                "declare: option requires an argument -- p\n".to_string(),
            ),
        }
    }
}

pub struct DeclareMap {
    pub completions: HashMap<String, String>,
}

impl DeclareMap {
    pub fn new() -> Self {
        Self {
            completions: HashMap::new(),
        }
    }
}

fn is_valid_identifier(s: &str) -> bool {
    let mut chars = s.chars();

    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }

    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
