/// 原始词法分析结果
#[derive(Debug, Clone, PartialEq)]
pub enum RawToken {
    Word(String),
    Pipe,         // |
    IoNumber(u8), // 0,1,2... 仅在重定向前有意义
    Redirect(RedirectOp),
}

/// 重定向操作符
#[derive(Debug, Clone, Copy, PartialEq)]
enum RedirectOp {
    Out,       // >
    OutAppend, // >>
    In,        // <
    Heredoc,   // <<
    DupOut,    // >&
    DupIn,     // <&
}

/// 词法分析器状态
#[derive(Debug, Clone, Copy, PartialEq)]
enum LexerState {
    Normal,
    DoubleQuote,
    SingleQuote,
    Escaping,
    DoubleQuoteEscaping,
}

/// 更符合Linux真实shell风格的词法分析器
pub fn tokenize_line(line: &str) -> Vec<RawToken> {
    // todo 修改tokens为result
    let mut tokens = Vec::new();
    let mut current_word = String::new();
    let mut state = LexerState::Normal;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match state {
            LexerState::Normal => {
                match ch {
                    // 空白字符
                    ch if ch.is_whitespace() => {
                        // 这块内容能不能消除重复
                        if !current_word.is_empty() {
                            tokens.push(RawToken::Word(current_word.clone()));
                            current_word.clear();
                        }
                    }
                    // 管道
                    '|' => {
                        if !current_word.is_empty() {
                            tokens.push(RawToken::Word(current_word.clone()));
                            current_word.clear();
                        }
                        tokens.push(RawToken::Pipe);
                    }
                    // 重定向操作符
                    '>' | '<' => {
                        if !current_word.is_empty() {
                            tokens.push(parse_word(&current_word));
                            current_word.clear();
                        }
                        let op = parse_redirect_op(ch, &mut chars);
                        tokens.push(RawToken::Redirect(op));
                    }
                    // 引号
                    '\'' => {
                        state = LexerState::SingleQuote;
                    }
                    '"' => {
                        state = LexerState::DoubleQuote;
                    }
                    // 转义字符
                    '\\' => {
                        state = LexerState::Escaping;
                    }
                    // 普通字符
                    _ => {
                        current_word.push(ch);
                    }
                }
            }
            LexerState::SingleQuote => match ch {
                '\'' => {
                    state = LexerState::Normal;
                }
                _ => {
                    current_word.push(ch);
                }
            },
            LexerState::DoubleQuote => match ch {
                '"' => {
                    state = LexerState::Normal;
                }
                '\\' => {
                    state = LexerState::DoubleQuoteEscaping;
                }
                _ => {
                    current_word.push(ch);
                }
            },
            LexerState::Escaping => {
                current_word.push(ch);
                state = LexerState::Normal;
            }
            LexerState::DoubleQuoteEscaping => {
                // 在双引号内，只有特定字符需要转义
                match ch {
                    '"' | '\\' | '$' | '`' => {
                        current_word.push(ch);
                    }
                    _ => {
                        current_word.push('\\');
                        current_word.push(ch);
                    }
                }
                state = LexerState::DoubleQuote;
            }
        }
    }
    // 处理最后一个单词
    if !current_word.is_empty() {
        tokens.push(RawToken::Word(current_word.clone()));
    }
    tokens
}

/// 解析单词，识别IO编号
fn parse_word(word: &str) -> RawToken {
    // 检查是否为IO编号（仅数字，且在重定向前有意义）
    if let Ok(num) = word.parse::<u8>() {
        RawToken::IoNumber(num)
    } else {
        RawToken::Word(word.to_string())
    }
}

/// 解析重定向操作符
fn parse_redirect_op(
    first_char: char,
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> RedirectOp {
    match first_char {
        '>' => {
            match chars.peek() {
                Some('>') => {
                    chars.next(); // 消耗下一个字符
                    RedirectOp::OutAppend
                }
                Some('&') => {
                    chars.next(); // 消耗下一个字符
                    RedirectOp::DupOut
                }
                _ => RedirectOp::Out,
            }
        }
        '<' => {
            match chars.peek() {
                Some('<') => {
                    chars.next(); // 消耗下一个字符
                    RedirectOp::Heredoc
                }
                Some('&') => {
                    chars.next(); // 消耗下一个字符
                    RedirectOp::DupIn
                }
                _ => RedirectOp::In,
            }
        }
        _ => unreachable!(),
    }
}


use crate::Token;
/// 将原始词法分析结果转换为Token结构
pub fn raw_tokens_to_tokens(raw_tokens: Vec<RawToken>) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current_token = Token::new();
    let mut expecting_redirect_file = false;
    let mut current_redirect_op: Option<RedirectOp> = None;
    let mut fd = None;

    for raw_token in raw_tokens {
        match raw_token {
            RawToken::Word(word) => {
                if expecting_redirect_file {
                    if let Some(op) = current_redirect_op {
                        match op {
                            RedirectOp::Out | RedirectOp::OutAppend => {
                                if fd == Some(1) || fd == None {
                                    let _ = current_token.set_redirect(word, op == RedirectOp::OutAppend);
                                }else {
                                    let _ = current_token.set_redirect_err(word, op == RedirectOp::OutAppend);
                                }
                                fd = None;
                            }
                            RedirectOp::In | RedirectOp::Heredoc => {
                                // 输入重定向处理
                            }
                            RedirectOp::DupOut | RedirectOp::DupIn => {
                                // 文件描述符复制处理
                            }
                        }
                        expecting_redirect_file = false;
                        current_redirect_op = None;
                    }
                } else {
                    current_token.add_content(word);
                }
            }
            RawToken::Pipe => {
                if !current_token.content.is_empty() {
                    tokens.push(current_token);
                    current_token = Token::new();
                }
            }
            RawToken::IoNumber(num) => {
                // IO编号仅在重定向前有意义，这里暂时忽略
                fd = Some(num);
            }
            RawToken::Redirect(op) => {
                expecting_redirect_file = true;
                current_redirect_op = Some(op);
            }
        }
    }

    // 添加最后一个token
    if !current_token.content.is_empty() {
        tokens.push(current_token);
    }
    tokens
}
