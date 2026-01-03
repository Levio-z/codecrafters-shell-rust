#[derive(Debug, Clone)]
pub struct Command {
    pub argv: Vec<String>,
    pub redirections: Vec<Redirection>, // 有序，决定语义
}

#[derive(Debug, Clone)]
pub struct Redirection {
    pub src_fd: Option<u8>,   // None = 默认 fd（>, <）
    pub op: RedirectOp,
    pub target: RedirectTarget,
}

#[derive(Debug, Clone, Copy)]
pub enum RedirectOp {
    Out,        // >
    OutAppend, // >>
    In,         // <
    Heredoc,    // <<
    DupOut,     // >&
    DupIn,      // <&
}

#[derive(Debug, Clone)]
pub enum RedirectTarget {
    File(String),   // > file
    Fd(u8),         // 2>&1
    Close,          // 2>&-
    Heredoc(String),
}

pub fn parse_simple_command(tokens: &[RawToken]) -> Command {
    let mut argv = Vec::new();
    let mut redirections = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            RawToken::Word(w) => {
                argv.push(w.clone());
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
