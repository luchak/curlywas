use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use chumsky::{prelude::*, stream::Stream};
use std::fmt;

pub type Span = std::ops::Range<usize>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Token {
    Import,
    Export,
    Fn,
    Let,
    Memory,
    Global,
    Mut,
    Loop,
    BranchIf,
    Defer,
    Ident(String),
    Str(String),
    Int(i32),
    Float(String),
    Op(String),
    Ctrl(char),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Import => write!(f, "import"),
            Token::Export => write!(f, "export"),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Memory => write!(f, "memory"),
            Token::Global => write!(f, "global"),
            Token::Mut => write!(f, "mut"),
            Token::Loop => write!(f, "loop"),
            Token::BranchIf => write!(f, "branch_if"),
            Token::Defer => write!(f, "defer"),
            Token::Ident(s) => write!(f, "{}", s),
            Token::Str(s) => write!(f, "{:?}", s),
            Token::Int(v) => write!(f, "{}", v),
            Token::Float(v) => write!(f, "{}", v),
            Token::Op(s) => write!(f, "{}", s),
            Token::Ctrl(c) => write!(f, "{}", c),
        }
    }
}

pub fn parse(source: &str) -> Result<(), ()> {
    let tokens = match lexer().parse(source) {
        Ok(tokens) => tokens,
        Err(errors) => {
            report_errors(
                errors
                    .into_iter()
                    .map(|e| e.map(|c| c.to_string()))
                    .collect(),
                source,
            );
            return Err(());
        }
    };

    let source_len = source.chars().count();
    let script = match script_parser().parse(Stream::from_iter(
        source_len..source_len + 1,
        tokens.into_iter(),
    )) {
        Ok(script) => script,
        Err(errors) => {
            report_errors(
                errors
                    .into_iter()
                    .map(|e| e.map(|t| t.to_string()))
                    .collect(),
                source,
            );
            return Err(());
        }
    };
    dbg!(script);
    Ok(())
}

fn report_errors(errors: Vec<Simple<String>>, source: &str) {
    for error in errors {
        let report = Report::build(ReportKind::Error, (), error.span().start());

        let report = match error.reason() {
            chumsky::error::SimpleReason::Unclosed { span, delimiter } => report
                .with_message(format!(
                    "Unclosed delimiter {}",
                    delimiter.fg(Color::Yellow)
                ))
                .with_label(
                    Label::new(span.clone())
                        .with_message(format!(
                            "Unclosed delimiter {}",
                            delimiter.fg(Color::Yellow)
                        ))
                        .with_color(Color::Yellow),
                )
                .with_label(
                    Label::new(error.span())
                        .with_message(format!(
                            "Must be closed before this {}",
                            error
                                .found()
                                .unwrap_or(&"end of file".to_string())
                                .fg(Color::Red)
                        ))
                        .with_color(Color::Red),
                ),
            chumsky::error::SimpleReason::Unexpected => report
                .with_message(format!(
                    "{}, expected one of {}",
                    if error.found().is_some() {
                        "Unexpected token in input"
                    } else {
                        "Unexpted end of input"
                    },
                    if error.expected().len() == 0 {
                        "end of input".to_string()
                    } else {
                        error
                            .expected()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ))
                .with_label(
                    Label::new(error.span())
                        .with_message(format!(
                            "Unexpected token {}",
                            error
                                .found()
                                .unwrap_or(&"end of file".to_string())
                                .fg(Color::Red)
                        ))
                        .with_color(Color::Red),
                ),
            chumsky::error::SimpleReason::Custom(msg) => report.with_message(msg).with_label(
                Label::new(error.span())
                    .with_message(format!("{}", msg.fg(Color::Red)))
                    .with_color(Color::Red),
            ),
        };

        report.finish().eprint(Source::from(source)).unwrap();
    }
}

fn lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Simple<char>> {
    let float = text::int(10)
        .chain::<char, _, _>(just('.').chain(text::digits(10)))
        .collect::<String>()
        .map(Token::Float);

    let int = text::int(10).map(|s: String| Token::Int(s.parse().unwrap()));

    let str_ = just('"')
        .ignore_then(filter(|c| *c != '"').repeated())
        .then_ignore(just('"'))
        .collect::<String>()
        .map(Token::Str);

    let op = one_of("+-*/%&^|<=>".chars())
        .repeated()
        .at_least(1)
        .or(just(':').chain(just('=')))
        .collect::<String>()
        .map(Token::Op);

    let ctrl = one_of("(){};,:?!".chars()).map(Token::Ctrl);

    let ident = text::ident().map(|ident: String| match ident.as_str() {
        "import" => Token::Import,
        "export" => Token::Export,
        "fn" => Token::Fn,
        "let" => Token::Let,
        "memory" => Token::Memory,
        "global" => Token::Global,
        "mut" => Token::Mut,
        "loop" => Token::Loop,
        "branch_if" => Token::BranchIf,
        "defer" => Token::Defer,
        _ => Token::Ident(ident),
    });

    let single_line =
        seq::<_, _, Simple<char>>("//".chars()).then_ignore(take_until(text::newline()));

    let multi_line =
        seq::<_, _, Simple<char>>("/*".chars()).then_ignore(take_until(seq("*/".chars())));

    let comment = single_line.or(multi_line);

    let token = float
        .or(int)
        .or(str_)
        .or(op)
        .or(ctrl)
        .or(ident)
        .recover_with(skip_then_retry_until([]));

    token
        .map_with_span(|tok, span| (tok, span))
        .padded()
        .padded_by(comment.padded().repeated())
        .repeated()
}

mod ast {
    use super::Span;

    #[derive(Debug)]
    pub struct Script {
        pub imports: Vec<Import>,
        pub global_vars: Vec<GlobalVar>,
        pub functions: Vec<Function>,
    }

    #[derive(Debug)]
    pub enum TopLevelItem {
        Import(Import),
        GlobalVar(GlobalVar),
        Function(Function),
    }

    #[derive(Debug)]
    pub struct Import {
        pub span: Span,
        pub import: String,
        pub type_: ImportType,
    }

    #[derive(Debug)]
    pub enum ImportType {
        Memory(u32),
        Variable {
            name: String,
            type_: Type,
            mutable: bool,
        },
        // Function { name: String, params: Vec<Type>, result: Option<Type> }
    }

    #[derive(Debug)]
    pub struct GlobalVar {
        pub span: Span,
        pub name: String,
        pub type_: Type,
    }

    #[derive(Debug)]
    pub struct Function {
        pub span: Span,
        pub export: bool,
        pub name: String,
        pub params: Vec<(String, Type)>,
        pub type_: Option<Type>,
        pub body: Block,
    }

    #[derive(Debug)]
    pub struct Block {
        pub statements: Vec<Expression>,
        pub final_expression: Option<Box<Expression>>,
    }

    impl Block {
        pub fn type_(&self) -> Option<Type> {
            self.final_expression.as_ref().and_then(|e| e.type_)
        }
    }

    #[derive(Debug)]
    pub struct MemoryLocation {
        pub span: Span,
        pub size: MemSize,
        pub left: Box<Expression>,
        pub right: Box<Expression>,
    }

    #[derive(Debug)]
    pub struct LocalVariable {
        pub span: Span,
        pub name: String,
        pub type_: Option<Type>,
        pub value: Option<Expression>,
        pub defer: bool,
    }

    #[derive(Debug)]
    pub struct Expression {
        pub type_: Option<Type>,
        pub expr: Expr,
        pub span: Span,
    }

    #[derive(Debug)]
    pub enum Expr {
        I32Const(i32),
        F32Const(f32),
        Variable(String),
        Let {
            name: String,
            type_: Option<Type>,
            value: Option<Box<Expression>>,
            defer: bool,
        },
        Poke {
            mem_location: MemoryLocation,
            value: Box<Expression>,
        },
        Loop {
            label: String,
            block: Box<Block>,
        },
        BranchIf {
            condition: Box<Expression>,
            label: String,
        },
        BinOp {
            op: BinOp,
            left: Box<Expression>,
            right: Box<Expression>,
        },
        LocalTee {
            name: String,
            value: Box<Expression>,
        },
        Cast {
            value: Box<Expression>,
            type_: Type,
        },
        FuncCall {
            name: String,
            params: Vec<Expression>,
        },
        Select {
            condition: Box<Expression>,
            if_true: Box<Expression>,
            if_false: Box<Expression>,
        },
        Error,
    }

    impl Expr {
        pub fn with_span(self, span: Span) -> Expression {
            Expression {
                type_: None,
                expr: self,
                span: span,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum BinOp {
        Add,
        Sub,
        Mul,
        Div,
        Rem,
        And,
        Or,
        Xor,
        Eq,
        Ne,
        Gt,
        Ge,
        Lt,
        Le,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MemSize {
        Byte,
        Word,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
    pub enum Type {
        I32,
        I64,
        F32,
        F64,
    }
}

fn map_token<O>(
    f: impl Fn(&Token) -> Option<O> + 'static + Clone,
) -> impl Parser<Token, O, Error = Simple<Token>> + Clone {
    filter_map(move |span, tok: Token| {
        if let Some(output) = f(&tok) {
            Ok(output)
        } else {
            Err(Simple::expected_input_found(span, Vec::new(), Some(tok)))
        }
    })
}

fn block_parser() -> impl Parser<Token, ast::Block, Error = Simple<Token>> + Clone {
    recursive(|block| {
        let expression = recursive(|expression| {
            let val = map_token(|tok| match tok {
                Token::Int(v) => Some(ast::Expr::I32Const(*v)),
                Token::Float(v) => Some(ast::Expr::F32Const(v.parse().unwrap())),
                _ => None,
            })
            .labelled("value");

            let variable = filter_map(|span, tok| match tok {
                Token::Ident(id) => Ok(ast::Expr::Variable(id)),
                _ => Err(Simple::expected_input_found(span, Vec::new(), Some(tok))),
            })
            .labelled("variable");

            let ident = filter_map(|span, tok| match tok {
                Token::Ident(id) => Ok(id),
                _ => Err(Simple::expected_input_found(span, Vec::new(), Some(tok))),
            })
            .labelled("identifier");

            let local_tee = ident
                .then(just(Token::Op(":=".to_string())).ignore_then(expression.clone()))
                .map(|(name, expr)| ast::Expr::LocalTee {
                    name,
                    value: Box::new(expr),
                });

            let loop_expr = just(Token::Loop)
                .ignore_then(ident)
                .then(
                    block
                        .clone()
                        .delimited_by(Token::Ctrl('{'), Token::Ctrl('}')),
                )
                .map(|(label, block)| ast::Expr::Loop {
                    label,
                    block: Box::new(block),
                });

            let branch_if = just(Token::BranchIf)
                .ignore_then(expression.clone())
                .then_ignore(just(Token::Ctrl(':')))
                .then(ident)
                .map(|(condition, label)| ast::Expr::BranchIf {
                    condition: Box::new(condition),
                    label,
                });

            let let_ = just(Token::Let)
                .ignore_then(just(Token::Defer).or_not())
                .then(ident.clone())
                .then(just(Token::Ctrl(':')).ignore_then(type_parser()).or_not())
                .then(
                    just(Token::Op("=".to_string()))
                        .ignore_then(expression.clone())
                        .or_not(),
                )
                .map(|(((defer, name), type_), value)| ast::Expr::Let {
                    name,
                    type_,
                    value: value.map(Box::new),
                    defer: defer.is_some(),
                });

            let tee = ident
                .clone()
                .then_ignore(just(Token::Op(":=".to_string())))
                .then(expression.clone())
                .map(|(name, value)| ast::Expr::LocalTee {
                    name,
                    value: Box::new(value),
                });

            let atom = val
                .or(tee)
                .or(variable)
                .or(local_tee)
                .or(loop_expr)
                .or(branch_if)
                .or(let_)
                .map_with_span(|expr, span| expr.with_span(span))
                .or(expression
                    .clone()
                    .delimited_by(Token::Ctrl('('), Token::Ctrl(')')))
                .recover_with(nested_delimiters(
                    Token::Ctrl('('),
                    Token::Ctrl(')'),
                    [(Token::Ctrl('{'), Token::Ctrl('}'))],
                    |span| ast::Expr::Error.with_span(span),
                ));

            let mem_size = just(Token::Ctrl('?'))
                .to(ast::MemSize::Byte)
                .or(just(Token::Ctrl('!')).to(ast::MemSize::Word));

            let memory_op = atom
                .clone()
                .then(
                    mem_size
                        .then(atom.clone())
                        .then_ignore(just(Token::Op("=".to_string())))
                        .then(expression.clone())
                        .repeated(),
                )
                .foldl(|left, ((size, right), value)| ast::Expression {
                    span: left.span.start..value.span.end,
                    expr: ast::Expr::Poke {
                        mem_location: ast::MemoryLocation {
                            span: left.span.start..right.span.end,
                            left: Box::new(left),
                            size,
                            right: Box::new(right),
                        },
                        value: Box::new(value),
                    },
                    type_: None,
                });

            let op_product = memory_op
                .clone()
                .then(
                    just(Token::Op("*".to_string()))
                        .to(ast::BinOp::Mul)
                        .or(just(Token::Op("/".to_string())).to(ast::BinOp::Div))
                        .or(just(Token::Op("%".to_string())).to(ast::BinOp::Rem))
                        .then(memory_op.clone())
                        .repeated(),
                )
                .foldl(|left, (op, right)| ast::Expression {
                    span: left.span.start..right.span.end,
                    expr: ast::Expr::BinOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    type_: None,
                });

            let op_sum = op_product
                .clone()
                .then(
                    just(Token::Op("+".to_string()))
                        .to(ast::BinOp::Add)
                        .or(just(Token::Op("-".to_string())).to(ast::BinOp::Sub))
                        .then(op_product.clone())
                        .repeated(),
                )
                .foldl(|left, (op, right)| ast::Expression {
                    span: left.span.start..right.span.end,
                    expr: ast::Expr::BinOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    type_: None,
                });

            let op_cmp = op_sum
                .clone()
                .then(
                    just(Token::Op("==".to_string()))
                        .to(ast::BinOp::Eq)
                        .or(just(Token::Op("!=".to_string())).to(ast::BinOp::Ne))
                        .or(just(Token::Op("<".to_string())).to(ast::BinOp::Lt))
                        .or(just(Token::Op("<=".to_string())).to(ast::BinOp::Le))
                        .or(just(Token::Op(">".to_string())).to(ast::BinOp::Gt))
                        .or(just(Token::Op(">=".to_string())).to(ast::BinOp::Ge))
                        .then(op_sum.clone())
                        .repeated(),
                )
                .foldl(|left, (op, right)| ast::Expression {
                    span: left.span.start..right.span.end,
                    expr: ast::Expr::BinOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    type_: None,
                });

            let op_bit = op_cmp
                .clone()
                .then(
                    just(Token::Op("&".to_string()))
                        .to(ast::BinOp::And)
                        .or(just(Token::Op("|".to_string())).to(ast::BinOp::Or))
                        .or(just(Token::Op("^".to_string())).to(ast::BinOp::Xor))
                        .then(op_cmp.clone())
                        .repeated(),
                )
                .foldl(|left, (op, right)| ast::Expression {
                    span: left.span.start..right.span.end,
                    expr: ast::Expr::BinOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    type_: None,
                });

            op_bit
        });

        expression
            .clone()
            .then_ignore(just(Token::Ctrl(';')))
            .repeated()
            .then(expression.clone().or_not())
            .map(|(statements, final_expression)| ast::Block {
                statements,
                final_expression: final_expression.map(|e| Box::new(e)),
            })
    })
}

fn type_parser() -> impl Parser<Token, ast::Type, Error = Simple<Token>> + Clone {
    filter_map(|span, tok| match tok {
        Token::Ident(id) if id == "i32" => Ok(ast::Type::I32),
        Token::Ident(id) if id == "i64" => Ok(ast::Type::I64),
        Token::Ident(id) if id == "f32" => Ok(ast::Type::F32),
        Token::Ident(id) if id == "f64" => Ok(ast::Type::F64),
        _ => Err(Simple::expected_input_found(
            span,
            vec![
                Token::Ident("i32".into()),
                Token::Ident("i64".into()),
                Token::Ident("f32".into()),
                Token::Ident("f64".into()),
            ],
            Some(tok),
        )),
    })
}

fn top_level_item_parser() -> impl Parser<Token, ast::TopLevelItem, Error = Simple<Token>> + Clone {
    let integer = map_token(|tok| match tok {
        Token::Int(v) => Some(*v),
        _ => None,
    });

    let string = map_token(|tok| match tok {
        Token::Str(s) => Some(s.clone()),
        _ => None,
    });

    let identifier = map_token(|tok| match tok {
        Token::Ident(id) => Some(id.clone()),
        _ => None,
    });

    let import_memory = just(Token::Memory)
        .ignore_then(integer.delimited_by(Token::Ctrl('('), Token::Ctrl(')')))
        .map(|min_size| ast::ImportType::Memory(min_size as u32));

    let import_global = just(Token::Global)
        .ignore_then(just(Token::Mut).or_not())
        .then(identifier.clone())
        .then_ignore(just(Token::Ctrl(':')))
        .then(type_parser())
        .map(|((mut_opt, name), type_)| ast::ImportType::Variable {
            mutable: mut_opt.is_some(),
            name,
            type_,
        });

    let import = just(Token::Import)
        .ignore_then(string)
        .then(import_memory.or(import_global))
        .then_ignore(just(Token::Ctrl(';')))
        .map_with_span(|(import, type_), span| {
            ast::TopLevelItem::Import(ast::Import {
                span,
                import,
                type_,
            })
        });

    let parameter = identifier
        .clone()
        .then_ignore(just(Token::Ctrl(':')))
        .then(type_parser());

    let function = just(Token::Export)
        .or_not()
        .then_ignore(just(Token::Fn))
        .then(identifier)
        .then(
            parameter
                .separated_by(just(Token::Ctrl(',')))
                .delimited_by(Token::Ctrl('('), Token::Ctrl(')')),
        )
        .then(
            just(Token::Op("->".to_string()))
                .ignore_then(type_parser())
                .or_not(),
        )
        .then(block_parser().delimited_by(Token::Ctrl('{'), Token::Ctrl('}')))
        .map_with_span(|((((export, name), params), type_), body), span| {
            ast::TopLevelItem::Function(ast::Function {
                span,
                params,
                export: export.is_some(),
                name,
                type_,
                body,
            })
        });

    import.or(function)
}

fn script_parser() -> impl Parser<Token, ast::Script, Error = Simple<Token>> + Clone {
    top_level_item_parser()
        .repeated()
        .then_ignore(end())
        .map(|items| {
            let mut script = ast::Script {
                imports: Vec::new(),
                global_vars: Vec::new(),
                functions: Vec::new(),
            };
            for item in items {
                match item {
                    ast::TopLevelItem::Import(i) => script.imports.push(i),
                    ast::TopLevelItem::GlobalVar(v) => script.global_vars.push(v),
                    ast::TopLevelItem::Function(f) => script.functions.push(f),
                }
            }
            script
        })
}
