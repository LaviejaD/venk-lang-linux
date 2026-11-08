use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span { pub line: u32, pub col: u32 }

#[derive(Debug)]
pub struct Error { pub msg: String, pub span: Span }
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.span.line, self.span.col, self.msg)
    }
}
pub type Res<T> = Result<T, Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Int(i64),
    Float(f64),
    Str(String),
    Char(char),
    Ident(String),

    Let, Mut, Const, Fn, Return, If, Elif, Else, Match, For, In, While, Loop,
    Break, Continue, Import, From, As, Pub, Use, Struct, Enum, Impl, Trait,
    Type, Async, Await, Try, Catch, Throw, And, Or, Not, Is, Sizeof, SelfKw,
    Vip, Vipc, True, False, Null,

    Plus, Minus, Star, Slash, Percent, StarStar,
    EqEq, NotEq, Lt, Le, Gt, Ge,
    Amp, Pipe, Caret, Tilde, Shl, Shr,
    Assign, PlusAssign, MinusAssign, StarAssign, SlashAssign, PercentAssign,
    Question, QuestionDot, QuestionQuestion,
    PipeGt, Arrow, FatArrow,
    Dot, ColonColon, Comma, Semi, Colon,
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    At, DotDot, DotDotDot,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token { pub tok: Tok, pub span: Span }

pub fn lex(src: &str) -> Res<Vec<Token>> {
    let mut lx = Lexer { c: src.chars().collect(), i: 0, line: 1, col: 1 };
    let mut out = Vec::new();
    while let Some(t) = lx.next_token()? {
        out.push(t);
    }
    let sp = Span { line: lx.line, col: lx.col };
    out.push(Token { tok: Tok::Eof, span: sp });
    Ok(out)
}

struct Lexer { c: Vec<char>, i: usize, line: u32, col: u32 }

impl Lexer {
    fn peek(&self) -> Option<char> { self.c.get(self.i).copied() }
    fn peek_at(&self, k: usize) -> Option<char> { self.c.get(self.i + k).copied() }

    fn bump(&mut self) -> Option<char> {
        let ch = self.c.get(self.i).copied()?;
        self.i += 1;
        if ch == '\n' { self.line += 1; self.col = 1; } else { self.col += 1; }
        Some(ch)
    }
    fn eat(&mut self, want: char) -> bool {
        if self.peek() == Some(want) { self.bump(); true } else { false }
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => { self.bump(); }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() { if c == '\n' { break; } self.bump(); }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    self.bump(); self.bump();
                    let mut depth = 1;
                    while depth > 0 {
                        match self.peek() {
                            None => break,
                            Some('/') if self.peek_at(1) == Some('*') => { self.bump(); self.bump(); depth += 1; }
                            Some('*') if self.peek_at(1) == Some('/') => { self.bump(); self.bump(); depth -= 1; }
                            _ => { self.bump(); }
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Res<Option<Token>> {
        self.skip_trivia();
        let span = Span { line: self.line, col: self.col };
        let ch = match self.peek() { Some(c) => c, None => return Ok(None) };

        let tok = if ch == '"' {
            self.string_lit(span)?
        } else if ch == '\'' {
            self.char_lit(span)?
        } else if ch.is_ascii_digit() {
            self.number()?
        } else if ch.is_alphabetic() || ch == '_' {
            self.ident_or_kw()
        } else {
            self.bump();
            match ch {
                '+' => if self.eat('=') { Tok::PlusAssign } else { Tok::Plus },
                '-' => if self.eat('>') { Tok::Arrow }
                       else if self.eat('=') { Tok::MinusAssign } else { Tok::Minus },
                '*' => if self.eat('*') { Tok::StarStar }
                       else if self.eat('=') { Tok::StarAssign } else { Tok::Star },
                '/' => if self.eat('=') { Tok::SlashAssign } else { Tok::Slash },
                '%' => if self.eat('=') { Tok::PercentAssign } else { Tok::Percent },
                '=' => if self.eat('=') { Tok::EqEq }
                       else if self.eat('>') { Tok::FatArrow } else { Tok::Assign },
                '!' => if self.eat('=') { Tok::NotEq } else { return Err(err(span, "неизвестный символ '!'")); },
                '<' => if self.eat('=') { Tok::Le }
                       else if self.eat('<') { Tok::Shl } else { Tok::Lt },
                '>' => if self.eat('=') { Tok::Ge }
                       else if self.eat('>') { Tok::Shr } else { Tok::Gt },
                '&' => Tok::Amp,
                '|' => if self.eat('>') { Tok::PipeGt } else { Tok::Pipe },
                '^' => Tok::Caret,
                '~' => Tok::Tilde,
                '?' => if self.eat('.') { Tok::QuestionDot }
                       else if self.eat('?') { Tok::QuestionQuestion } else { Tok::Question },
                '.' => if self.eat('.') {
                           if self.eat('.') { Tok::DotDotDot } else { Tok::DotDot }
                       } else { Tok::Dot },
                ':' => if self.eat(':') { Tok::ColonColon } else { Tok::Colon },
                ',' => Tok::Comma,
                ';' => Tok::Semi,
                '@' => Tok::At,
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                '[' => Tok::LBracket,
                ']' => Tok::RBracket,
                '{' => Tok::LBrace,
                '}' => Tok::RBrace,
                _ => return Err(err(span, format!("неизвестный символ '{ch}'"))),
            }
        };
        Ok(Some(Token { tok, span }))
    }

    fn number(&mut self) -> Res<Tok> {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') { self.bump(); }
        let mut is_float = false;
        if self.peek() == Some('.') && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit()) {
            is_float = true;
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') { self.bump(); }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            let k = if matches!(self.peek_at(1), Some('+') | Some('-')) { 2 } else { 1 };
            if matches!(self.peek_at(k), Some(c) if c.is_ascii_digit()) {
                is_float = true;
                for _ in 0..k { self.bump(); }
                while matches!(self.peek(), Some(c) if c.is_ascii_digit()) { self.bump(); }
            }
        }
        let s: String = self.c[start..self.i].iter().filter(|c| **c != '_').collect();
        if is_float {
            s.parse::<f64>().map(Tok::Float).map_err(|_| err(Span { line: self.line, col: self.col }, "плохой float"))
        } else {
            s.parse::<i64>().map(Tok::Int).map_err(|_| err(Span { line: self.line, col: self.col }, "плохой int"))
        }
    }

    fn string_lit(&mut self, span: Span) -> Res<Tok> {
        self.bump(); // "
        let mut s = String::new();
        loop {
            match self.bump() {
                None => return Err(err(span, "незакрытая строка")),
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('r') => s.push('\r'),
                    Some('0') => s.push('\0'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some('{') => s.push('{'),
                    Some('}') => s.push('}'),
                    Some(c) => { s.push('\\'); s.push(c); }
                    None => return Err(err(span, "незакрытая строка")),
                },
                Some(c) => s.push(c),
            }
        }
        // TODO: интерполяция "{expr}" — развернуть в StrParts на этапе парсинга
        Ok(Tok::Str(s))
    }

    fn char_lit(&mut self, span: Span) -> Res<Tok> {
        self.bump(); // '
        let ch = match self.bump() {
            None => return Err(err(span, "незакрытый char")),
            Some('\\') => match self.bump() {
                Some('n') => '\n', Some('t') => '\t', Some('r') => '\r',
                Some('0') => '\0', Some('\\') => '\\', Some('\'') => '\'',
                _ => return Err(err(span, "плохой escape в char")),
            },
            Some(c) => c,
        };
        if !self.eat('\'') { return Err(err(span, "char должен быть закрыт '")); }
        Ok(Tok::Char(ch))
    }

    fn ident_or_kw(&mut self) -> Tok {
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') { self.bump(); }
        let s: String = self.c[start..self.i].iter().collect();
        match s.as_str() {
            "let" => Tok::Let, "mut" => Tok::Mut, "const" => Tok::Const,
            "fn" => Tok::Fn, "return" => Tok::Return,
            "if" => Tok::If, "elif" => Tok::Elif, "else" => Tok::Else,
            "match" => Tok::Match, "for" => Tok::For, "in" => Tok::In,
            "while" => Tok::While, "loop" => Tok::Loop,
            "break" => Tok::Break, "continue" => Tok::Continue,
            "import" => Tok::Import, "from" => Tok::From, "as" => Tok::As,
            "pub" => Tok::Pub, "use" => Tok::Use,
            "struct" => Tok::Struct, "enum" => Tok::Enum,
            "impl" => Tok::Impl, "trait" => Tok::Trait, "type" => Tok::Type,
            "async" => Tok::Async, "await" => Tok::Await,
            "try" => Tok::Try, "catch" => Tok::Catch, "throw" => Tok::Throw,
            "and" => Tok::And, "or" => Tok::Or, "not" => Tok::Not,
            "is" => Tok::Is, "sizeof" => Tok::Sizeof, "self" => Tok::SelfKw,
            "vip" => Tok::Vip, "vipc" => Tok::Vipc,
            "true" => Tok::True, "false" => Tok::False, "null" => Tok::Null,
            _ => Tok::Ident(s),
        }
    }
}

fn err(span: Span, msg: impl Into<String>) -> Error { Error { msg: msg.into(), span } }