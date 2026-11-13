use logos::Logos;

/// Diagnostic text for a match the lexer refused: a slice carrying an
/// embedded NUL was a string literal the String callback rejected — spell
/// out the law; anything else is the generic lexer error. Both front-end
/// Err arms (pipeline.rs, require.rs) route through here so the wording
/// cannot drift between them.
pub fn rejected_match_text(slice: &str, off: usize) -> String {
    if slice.contains('\0') {
        format!(
            "Lexer error: embedded NUL byte in string literal at byte {off} \
             — alm strings are NUL-terminated, so the literal would \
             silently truncate at print time"
        )
    } else {
        "Lexer error".to_string()
    }
}

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip(r"[ \t\n\r\f\v]+"))]
#[logos(skip(r"(?s)--\[\[.*?\]\]"))]
#[logos(skip(r"--[^\n]*|#![^\n]*", allow_greedy = true))]
pub enum Token<'a> {
    #[token("local")]
    Local,
    #[token("=")]
    Assign,
    #[token("do")]
    Do,
    #[token("end")]
    End,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("nil")]
    Nil,
    #[token("print")]
    Print,
    #[token("if")]
    If,
    #[token("then")]
    Then,
    #[token("elseif")]
    ElseIf,
    #[token("else")]
    Else,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("break")]
    Break,
    #[token("return")]
    Return,
    #[token("function")]
    Function,
    #[token("not")]
    Not,
    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("<")]
    LessThan,
    #[token("<=")]
    LessEq,
    #[token(">")]
    GreaterThan,
    #[token(">=")]
    GreaterEq,
    #[token("==")]
    Equal,
    #[token("~=")]
    NotEqual,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("//")]
    DoubleSlash,
    #[token("%")]
    Percent,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("..")]
    DotDot,
    #[token(":")]
    Colon,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("#")]
    Len,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier(&'a str),

    #[regex(r"[0-9]+", |lex| lex.slice().parse().ok())]
    Integer(i64),

    #[regex(r"[0-9]+\.[0-9]+", |lex| lex.slice().parse().ok())]
    Float(f64),

    // A literal carrying an embedded NUL rejects its match (None): the
    // front end's Err arm turns the failed span into the embedded-NUL
    // diagnostic. alm strings are NUL-terminated, so the literal would
    // silently truncate at print time — a diagnostic, never a panic.
    #[regex(r#""[^"]*""#, |lex| {
        let s = lex.slice();
        (!s.contains('\0')).then_some(s)
    })]
    String(&'a str),
}
