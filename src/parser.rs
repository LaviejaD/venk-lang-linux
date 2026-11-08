use crate::ast::*;
use crate::lexer::{Error, Res, Span, Tok, Token};

pub enum ReplInput {
    Item(Item),
    Stmt(Stmt),
    Expr(Expr),
}

pub fn parse(toks: Vec<Token>) -> Res<Program> {
    let mut p = Parser::new(toks);
    p.parse_program()
}

pub fn parse_expr_source(toks: Vec<Token>) -> Res<Expr> {
    let mut p = Parser::new(toks);
    p.parse_expr_public()
}

pub struct Parser {
    toks: Vec<Token>,
    i: usize,
    no_record: bool,
}

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Parser { toks, i: 0, no_record: false }
    }

    fn peek(&self) -> &Tok { &self.toks[self.i].tok }
    fn peek_at(&self, k: usize) -> &Tok {
        self.toks.get(self.i + k).map(|t| &t.tok).unwrap_or(&Tok::Eof)
    }
    fn span(&self) -> Span { self.toks[self.i].span }
    fn bump(&mut self) -> Token {
        let t = self.toks[self.i].clone();
        if self.i + 1 < self.toks.len() { self.i += 1; }
        t
    }
    fn check(&self, t: &Tok) -> bool { self.peek() == t }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.check(t) { self.bump(); true } else { false }
    }
    fn err<T>(&self, msg: impl Into<String>) -> Res<T> {
        Err(Error { msg: msg.into(), span: self.span() })
    }
    fn expect(&mut self, t: Tok) -> Res<()> {
        if self.check(&t) { self.bump(); Ok(()) }
        else { self.err(format!("ожидалось {:?}, получено {:?}", t, self.peek())) }
    }
    fn expect_ident(&mut self) -> Res<String> {
        match self.peek().clone() {
            Tok::Ident(s) => { self.bump(); Ok(s) }
            other => self.err(format!("ожидался идентификатор, получено {:?}", other)),
        }
    }

    // Разрешаем использовать ключевые слова после ::
    fn expect_path_segment(&mut self) -> Res<String> {
        match self.peek().clone() {
            Tok::Ident(s) => { self.bump(); Ok(s) }
            Tok::Match => { self.bump(); Ok("match".to_string()) }
            Tok::Type => { self.bump(); Ok("type".to_string()) }
            Tok::Struct => { self.bump(); Ok("struct".to_string()) }
            Tok::Enum => { self.bump(); Ok("enum".to_string()) }
            Tok::SelfKw => { self.bump(); Ok("self".to_string()) }
            Tok::Fn => { self.bump(); Ok("fn".to_string()) }
            Tok::Return => { self.bump(); Ok("return".to_string()) }
            Tok::If => { self.bump(); Ok("if".to_string()) }
            Tok::Else => { self.bump(); Ok("else".to_string()) }
            Tok::For => { self.bump(); Ok("for".to_string()) }
            Tok::In => { self.bump(); Ok("in".to_string()) }
            Tok::While => { self.bump(); Ok("while".to_string()) }
            Tok::Loop => { self.bump(); Ok("loop".to_string()) }
            Tok::Break => { self.bump(); Ok("break".to_string()) }
            Tok::Continue => { self.bump(); Ok("continue".to_string()) }
            Tok::Import => { self.bump(); Ok("import".to_string()) }
            Tok::From => { self.bump(); Ok("from".to_string()) }
            Tok::As => { self.bump(); Ok("as".to_string()) }
            Tok::Pub => { self.bump(); Ok("pub".to_string()) }
            Tok::Use => { self.bump(); Ok("use".to_string()) }
            Tok::Impl => { self.bump(); Ok("impl".to_string()) }
            Tok::Trait => { self.bump(); Ok("trait".to_string()) }
            Tok::Async => { self.bump(); Ok("async".to_string()) }
            Tok::Await => { self.bump(); Ok("await".to_string()) }
            Tok::Try => { self.bump(); Ok("try".to_string()) }
            Tok::Catch => { self.bump(); Ok("catch".to_string()) }
            Tok::Throw => { self.bump(); Ok("throw".to_string()) }
            Tok::And => { self.bump(); Ok("and".to_string()) }
            Tok::Or => { self.bump(); Ok("or".to_string()) }
            Tok::Not => { self.bump(); Ok("not".to_string()) }
            Tok::Is => { self.bump(); Ok("is".to_string()) }
            Tok::Sizeof => { self.bump(); Ok("sizeof".to_string()) }
            Tok::Vip => { self.bump(); Ok("vip".to_string()) }
            Tok::Vipc => { self.bump(); Ok("vipc".to_string()) }
            Tok::True => { self.bump(); Ok("true".to_string()) }
            Tok::False => { self.bump(); Ok("false".to_string()) }
            Tok::Null => { self.bump(); Ok("null".to_string()) }
            other => self.err(format!("ожидался идентификатор, получено {:?}", other)),
        }
    }

    fn parse_program(&mut self) -> Res<Program> {
        let mut items = Vec::new();
        while !self.check(&Tok::Eof) {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> Res<Item> {
        if self.eat(&Tok::Pub) {
            return match self.peek() {
                Tok::Fn => Ok(Item::Fn(self.parse_fn(true, false)?)),
                Tok::Async if *self.peek_at(1) == Tok::Fn => {
                    self.bump();
                    Ok(Item::Fn(self.parse_fn(true, true)?))
                }
                Tok::Struct => Ok(Item::Struct(self.parse_struct()?)),
                Tok::Enum => Ok(Item::Enum(self.parse_enum()?)),
                Tok::Impl => Ok(Item::Impl(self.parse_impl()?)),
                Tok::Trait => Ok(Item::Trait(self.parse_trait()?)),
                Tok::Type => self.parse_type_alias(),
                Tok::Const => self.parse_const(),
                _ => self.err("после 'pub' ожидалось объявление"),
            };
        }
        match self.peek() {
            Tok::Vipc => self.parse_vipc(),
            Tok::Vip => {
                let line = self.span().line;
                let mut raw = Vec::new();
                while !self.check(&Tok::Eof) && self.span().line == line {
                    raw.push(self.bump().tok);
                }
                Ok(Item::Vip(raw))
            }
            Tok::Import => {
                let d = self.parse_import()?;
                self.eat(&Tok::Semi);
                Ok(Item::Import(d))
            }
            Tok::Async if *self.peek_at(1) == Tok::Fn => {
                self.bump();
                Ok(Item::Fn(self.parse_fn(false, true)?))
            }
            Tok::Fn => Ok(Item::Fn(self.parse_fn(false, false)?)),
            Tok::Struct => Ok(Item::Struct(self.parse_struct()?)),
            Tok::Enum => Ok(Item::Enum(self.parse_enum()?)),
            Tok::Impl => Ok(Item::Impl(self.parse_impl()?)),
            Tok::Trait => Ok(Item::Trait(self.parse_trait()?)),
            Tok::Type => self.parse_type_alias(),
            Tok::Const => self.parse_const(),
            _ => Ok(Item::Stmt(self.parse_stmt()?)),
        }
    }

    fn parse_vipc(&mut self) -> Res<Item> {
        self.expect(Tok::Vipc)?;
        let name = self.expect_ident()?;
        let mut version = None;
        let mut meta = None;
        if let Tok::Str(s) = self.peek().clone() {
            self.bump();
            version = Some(s);
        } else if self.check(&Tok::LBrace) {
            self.bump();
            let mut m = Vec::new();
            while !self.check(&Tok::RBrace) {
                let k = self.expect_ident()?;
                self.expect(Tok::Colon)?;
                let v = self.parse_expr()?;
                m.push((k, v));
                if !self.eat(&Tok::Comma) { break; }
            }
            self.expect(Tok::RBrace)?;
            meta = Some(m);
        }
        Ok(Item::Vipc { name, version, meta })
    }

    fn parse_import(&mut self) -> Res<ImportDecl> {
        self.expect(Tok::Import)?;
        let names;
        let module: Vec<String>;
        if self.check(&Tok::LBrace) {
            self.bump();
            let mut ns = Vec::new();
            while !self.check(&Tok::RBrace) {
                ns.push(self.expect_ident()?);
                if !self.eat(&Tok::Comma) { break; }
            }
            self.expect(Tok::RBrace)?;
            self.expect(Tok::From)?;
            module = self.parse_path()?;
            names = Some(ns);
        } else {
            let mut segs = Vec::new();
            loop {
                if self.eat(&Tok::Star) {
                    segs.push("*".to_string());
                } else {
                    segs.push(self.expect_ident()?);
                }
                if !self.eat(&Tok::ColonColon) { break; }
            }
            if segs.last().map(|s| s == "*").unwrap_or(false) { segs.pop(); }
            module = segs;
            names = None;
        }
        let alias = if self.eat(&Tok::As) { Some(self.expect_ident()?) } else { None };
        Ok(ImportDecl { names, module, alias })
    }

    fn parse_path(&mut self) -> Res<Vec<String>> {
        let mut segs = vec![self.expect_ident()?];
        while self.eat(&Tok::ColonColon) { segs.push(self.expect_path_segment()?); }
        Ok(segs)
    }

    fn parse_fn(&mut self, public: bool, is_async: bool) -> Res<FnDecl> {
        self.expect(Tok::Fn)?;
        let name = self.expect_ident()?;
        self.expect(Tok::LParen)?;
        let mut params = Vec::new();
        while !self.check(&Tok::RParen) {
            let mutable = self.eat(&Tok::Mut);
            let pname = if self.eat(&Tok::SelfKw) { "self".to_string() } else { self.expect_ident()? };
            let ty = if self.eat(&Tok::Colon) { Some(self.parse_type()?) } else { None };
            params.push(Param { mutable, name: pname, ty });
            if !self.eat(&Tok::Comma) { break; }
        }
        self.expect(Tok::RParen)?;
        let ret = if self.eat(&Tok::Arrow) { Some(self.parse_type()?) } else { None };
        let body = if self.eat(&Tok::Assign) {
            FnBody::Expr(self.parse_expr()?)
        } else if self.check(&Tok::LBrace) {
            FnBody::Block(self.parse_block()?)
        } else if self.eat(&Tok::Semi) {
            FnBody::None
        } else {
            return self.err("ожидалось тело функции");
        };
        Ok(FnDecl { public, is_async, name, params, ret, body })
    }

    fn parse_struct(&mut self) -> Res<StructDecl> {
        self.expect(Tok::Struct)?;
        let name = self.expect_ident()?;
        self.expect(Tok::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&Tok::RBrace) {
            let public = self.eat(&Tok::Pub);
            let fname = self.expect_ident()?;
            self.expect(Tok::Colon)?;
            let ty = self.parse_type()?;
            fields.push(Field { public, name: fname, ty });
            if !self.eat(&Tok::Comma) { break; }
        }
        self.expect(Tok::RBrace)?;
        Ok(StructDecl { name, fields })
    }

    fn parse_enum(&mut self) -> Res<EnumDecl> {
        self.expect(Tok::Enum)?;
        let name = self.expect_ident()?;
        self.expect(Tok::LBrace)?;
        let mut variants = Vec::new();
        while !self.check(&Tok::RBrace) {
            let vname = self.expect_ident()?;
            let fields = if self.check(&Tok::LParen) {
                self.bump();
                let mut ts = Vec::new();
                while !self.check(&Tok::RParen) {
                    ts.push(self.parse_type()?);
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RParen)?;
                VariantFields::Tuple(ts)
            } else if self.check(&Tok::LBrace) {
                self.bump();
                let mut fs = Vec::new();
                while !self.check(&Tok::RBrace) {
                    let public = self.eat(&Tok::Pub);
                    let fname = self.expect_ident()?;
                    self.expect(Tok::Colon)?;
                    let ty = self.parse_type()?;
                    fs.push(Field { public, name: fname, ty });
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RBrace)?;
                VariantFields::Record(fs)
            } else {
                VariantFields::Unit
            };
            variants.push(Variant { name: vname, fields });
            if !self.eat(&Tok::Comma) { break; }
        }
        self.expect(Tok::RBrace)?;
        Ok(EnumDecl { name, variants })
    }

    fn parse_impl(&mut self) -> Res<ImplDecl> {
        self.expect(Tok::Impl)?;
        let ty = self.parse_type()?;
        self.expect(Tok::LBrace)?;
        let mut methods = Vec::new();
        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            let public = self.eat(&Tok::Pub);
            let is_async = self.eat(&Tok::Async);
            methods.push(self.parse_fn(public, is_async)?);
        }
        self.expect(Tok::RBrace)?;
        Ok(ImplDecl { ty, methods })
    }

    fn parse_trait(&mut self) -> Res<TraitDecl> {
        self.expect(Tok::Trait)?;
        let name = self.expect_ident()?;
        self.expect(Tok::LBrace)?;
        let mut methods = Vec::new();
        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            methods.push(self.parse_fn(false, false)?);
        }
        self.expect(Tok::RBrace)?;
        Ok(TraitDecl { name, methods })
    }

    fn parse_type_alias(&mut self) -> Res<Item> {
        self.expect(Tok::Type)?;
        let name = self.expect_ident()?;
        self.expect(Tok::Assign)?;
        let ty = self.parse_type()?;
        self.eat(&Tok::Semi);
        Ok(Item::TypeAlias { name, ty })
    }

    fn parse_const(&mut self) -> Res<Item> {
        self.expect(Tok::Const)?;
        let name = self.expect_ident()?;
        let ty = if self.eat(&Tok::Colon) { Some(self.parse_type()?) } else { None };
        self.expect(Tok::Assign)?;
        let value = self.parse_expr()?;
        self.eat(&Tok::Semi);
        Ok(Item::Const { name, ty, value })
    }

    fn parse_block(&mut self) -> Res<Block> {
        self.expect(Tok::LBrace)?;
        let mut stmts = Vec::new();
        while !self.check(&Tok::RBrace) && !self.check(&Tok::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(Tok::RBrace)?;
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> Res<Stmt> {
        match self.peek().clone() {
            Tok::Let => {
                self.bump();
                let mutable = self.eat(&Tok::Mut);
                let name = self.expect_ident()?;
                let ty = if self.eat(&Tok::Colon) { Some(self.parse_type()?) } else { None };
                let value = if self.eat(&Tok::Assign) { Some(self.parse_expr()?) } else { None };
                self.eat(&Tok::Semi);
                Ok(Stmt::Let { mutable, name, ty, value })
            }
            Tok::Mut if matches!(self.peek_at(1), Tok::Ident(_)) => {
                self.bump();
                let name = self.expect_ident()?;
                let ty = if self.eat(&Tok::Colon) { Some(self.parse_type()?) } else { None };
                let value = if self.eat(&Tok::Assign) { Some(self.parse_expr()?) } else { None };
                self.eat(&Tok::Semi);
                Ok(Stmt::Let { mutable: true, name, ty, value })
            }
            Tok::Return => {
                self.bump();
                let e = if self.check(&Tok::Semi) || self.check(&Tok::RBrace) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.eat(&Tok::Semi);
                Ok(Stmt::Return(e))
            }
            Tok::While => {
                self.bump();
                let cond = self.parse_cond()?;
                let body = self.parse_block()?;
                Ok(Stmt::While { cond, body })
            }
            Tok::Loop => {
                self.bump();
                Ok(Stmt::Loop(self.parse_block()?))
            }
            Tok::For => {
                self.bump();
                let pattern = self.parse_pattern()?;
                self.expect(Tok::In)?;
                let iter = self.parse_cond()?;
                let body = self.parse_block()?;
                Ok(Stmt::For { pattern, iter, body })
            }
            Tok::Break => { self.bump(); self.eat(&Tok::Semi); Ok(Stmt::Break) }
            Tok::Continue => { self.bump(); self.eat(&Tok::Semi); Ok(Stmt::Continue) }
            _ => {
                let expr = self.parse_expr()?;
                let semi = self.eat(&Tok::Semi);
                Ok(Stmt::Expr { expr, semi })
            }
        }
    }

    fn parse_cond(&mut self) -> Res<Expr> {
        let saved = self.no_record;
        self.no_record = true;
        let e = self.parse_expr();
        self.no_record = saved;
        e
    }

    pub fn parse_expr_public(&mut self) -> Res<Expr> {
        self.parse_expr()
    }

    pub fn parse_repl_line(&mut self) -> Res<ReplInput> {
        match self.peek() {
            Tok::Let | Tok::Mut | Tok::Return | Tok::While | Tok::Loop | Tok::For | Tok::Break | Tok::Continue => {
                Ok(ReplInput::Stmt(self.parse_stmt()?))
            }
            Tok::Pub | Tok::Fn | Tok::Struct | Tok::Enum | Tok::Impl | Tok::Trait | Tok::Type | Tok::Const | Tok::Import | Tok::Vip | Tok::Vipc => {
                Ok(ReplInput::Item(self.parse_item()?))
            }
            _ => Ok(ReplInput::Expr(self.parse_expr()?)),
        }
    }

    fn parse_expr(&mut self) -> Res<Expr> {
        self.parse_bp(0)
    }

    fn parse_bp(&mut self, min_bp: u8) -> Res<Expr> {
        let mut lhs = self.parse_prefix()?;
        loop {
            lhs = self.parse_postfix(lhs)?;

            enum K { Bin(BinOp), Asg(AssignOp), As, Is }

            let (l_bp, r_bp, kind) = match self.peek() {
                Tok::Assign => (1, 1, K::Asg(AssignOp::Set)),
                Tok::PlusAssign => (1, 1, K::Asg(AssignOp::Add)),
                Tok::MinusAssign => (1, 1, K::Asg(AssignOp::Sub)),
                Tok::StarAssign => (1, 1, K::Asg(AssignOp::Mul)),
                Tok::SlashAssign => (1, 1, K::Asg(AssignOp::Div)),
                Tok::PercentAssign => (1, 1, K::Asg(AssignOp::Rem)),
                Tok::PipeGt => (2, 3, K::Bin(BinOp::Pipe)),
                Tok::QuestionQuestion => (3, 4, K::Bin(BinOp::Default)),
                Tok::Or => (4, 5, K::Bin(BinOp::Or)),
                Tok::And => (5, 6, K::Bin(BinOp::And)),
                Tok::EqEq => (6, 7, K::Bin(BinOp::Eq)),
                Tok::NotEq => (6, 7, K::Bin(BinOp::Ne)),
                Tok::Lt => (6, 7, K::Bin(BinOp::Lt)),
                Tok::Le => (6, 7, K::Bin(BinOp::Le)),
                Tok::Gt => (6, 7, K::Bin(BinOp::Gt)),
                Tok::Ge => (6, 7, K::Bin(BinOp::Ge)),
                Tok::Is => (6, 7, K::Is),
                Tok::Pipe => (7, 8, K::Bin(BinOp::BitOr)),
                Tok::Caret => (8, 9, K::Bin(BinOp::BitXor)),
                Tok::Amp => (9, 10, K::Bin(BinOp::BitAnd)),
                Tok::Shl => (10, 11, K::Bin(BinOp::Shl)),
                Tok::Shr => (10, 11, K::Bin(BinOp::Shr)),
                Tok::Plus => (11, 12, K::Bin(BinOp::Add)),
                Tok::Minus => (11, 12, K::Bin(BinOp::Sub)),
                Tok::Star => (12, 13, K::Bin(BinOp::Mul)),
                Tok::Slash => (12, 13, K::Bin(BinOp::Div)),
                Tok::Percent => (12, 13, K::Bin(BinOp::Rem)),
                Tok::StarStar => (14, 13, K::Bin(BinOp::Pow)),
                Tok::As => (13, 14, K::As),
                _ => break,
            };
            if l_bp < min_bp { break; }
            self.bump();

            match kind {
                K::Asg(op) => {
                    let value = self.parse_bp(r_bp)?;
                    lhs = Expr::Assign { target: Box::new(lhs), op, value: Box::new(value) };
                }
                K::As => {
                    let ty = self.parse_type()?;
                    lhs = Expr::Cast { expr: Box::new(lhs), ty };
                }
                K::Is => {
                    let ty = self.parse_type()?;
                    lhs = Expr::IsType { expr: Box::new(lhs), ty };
                }
                K::Bin(op) => {
                    let rhs = self.parse_bp(r_bp)?;
                    lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
                }
            }
        }
        Ok(lhs)
    }

    fn parse_postfix(&mut self, mut e: Expr) -> Res<Expr> {
        loop {
            e = match self.peek() {
                Tok::LParen => {
                    let args = self.parse_args()?;
                    Expr::Call { callee: Box::new(e), args }
                }
                Tok::LBracket => {
                    self.bump();
                    let idx = self.parse_expr()?;
                    self.expect(Tok::RBracket)?;
                    Expr::Index { recv: Box::new(e), index: Box::new(idx) }
                }
                Tok::Dot => {
                    self.bump();
                    let name = self.expect_ident()?;
                    if self.check(&Tok::LParen) {
                        let args = self.parse_args()?;
                        Expr::Method { recv: Box::new(e), name, args }
                    } else {
                        Expr::Field { recv: Box::new(e), name }
                    }
                }
                Tok::QuestionDot => {
                    self.bump();
                    let name = self.expect_ident()?;
                    Expr::OptField { recv: Box::new(e), name }
                }
                Tok::Question => {
                    self.bump();
                    Expr::Propagate(Box::new(e))
                }
                _ => return Ok(e),
            };
        }
    }

    fn parse_args(&mut self) -> Res<Vec<Expr>> {
        self.expect(Tok::LParen)?;
        let mut args = Vec::new();
        while !self.check(&Tok::RParen) {
            args.push(self.parse_expr()?);
            if !self.eat(&Tok::Comma) { break; }
        }
        self.expect(Tok::RParen)?;
        Ok(args)
    }

    fn parse_prefix(&mut self) -> Res<Expr> {
        match self.peek().clone() {
            Tok::Int(n) => { self.bump(); Ok(Expr::Int(n)) }
            Tok::Float(f) => { self.bump(); Ok(Expr::Float(f)) }
            Tok::Str(s) => { self.bump(); Ok(Expr::Str(s)) }
            Tok::Char(c) => { self.bump(); Ok(Expr::Char(c)) }
            Tok::True => { self.bump(); Ok(Expr::Bool(true)) }
            Tok::False => { self.bump(); Ok(Expr::Bool(false)) }
            Tok::Null => { self.bump(); Ok(Expr::Null) }
            Tok::SelfKw => { self.bump(); Ok(Expr::Ident("self".into())) }

            Tok::Minus => {
                self.bump();
                let e = self.parse_bp(14)?;
                Ok(Expr::Unary { op: UnOp::Neg, expr: Box::new(e) })
            }
            Tok::Not => {
                self.bump();
                let e = self.parse_bp(14)?;
                Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(e) })
            }
            Tok::Tilde => {
                self.bump();
                let e = self.parse_bp(14)?;
                Ok(Expr::Unary { op: UnOp::BitNot, expr: Box::new(e) })
            }

            Tok::Sizeof => {
                self.bump();
                self.expect(Tok::LParen)?;
                let ty = self.parse_type()?;
                self.expect(Tok::RParen)?;
                Ok(Expr::Sizeof(ty))
            }

            Tok::Ident(name) => {
                self.bump();
                let mut path = vec![name];
                while self.eat(&Tok::ColonColon) { 
                    path.push(self.expect_path_segment()?); 
                }

                if path.len() == 1 && self.check(&Tok::LBrace) && !self.no_record {
                    self.bump();
                    let mut fields = Vec::new();
                    while !self.check(&Tok::RBrace) {
                        let k = self.expect_ident()?;
                        self.expect(Tok::Colon)?;
                        let v = self.parse_expr()?;
                        fields.push((k, v));
                        if !self.eat(&Tok::Comma) { break; }
                    }
                    self.expect(Tok::RBrace)?;
                    return Ok(Expr::StructInit(path, fields));
                }

                if path.len() == 1 && self.check(&Tok::FatArrow) {
                    self.bump();
                    let body = self.parse_expr()?;
                    return Ok(Expr::Lambda {
                        params: vec![Param { mutable: false, name: path.pop().unwrap(), ty: None }],
                        body: Box::new(body),
                        is_async: false,
                    });
                }
                if path.len() == 1 { Ok(Expr::Ident(path.pop().unwrap())) } else { Ok(Expr::Path(path)) }
            }

            Tok::LParen => {
                if let Some(l) = self.try_lambda()? { return Ok(l); }
                self.bump();
                let mut items = Vec::new();
                while !self.check(&Tok::RParen) {
                    items.push(self.parse_expr()?);
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RParen)?;
                if items.len() == 1 { Ok(items.pop().unwrap()) } else { Ok(Expr::Tuple(items)) }
            }

            Tok::LBracket => {
                self.bump();
                let mut items = Vec::new();
                while !self.check(&Tok::RBracket) {
                    items.push(self.parse_expr()?);
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RBracket)?;
                Ok(Expr::Array(items))
            }

            Tok::LBrace if !self.no_record => {
                self.bump();
                let mut fields = Vec::new();
                while !self.check(&Tok::RBrace) {
                    let name = self.expect_ident()?;
                    self.expect(Tok::Colon)?;
                    let v = self.parse_expr()?;
                    fields.push((name, v));
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RBrace)?;
                Ok(Expr::Record(fields))
            }
            Tok::LBrace => Ok(Expr::Block(self.parse_block()?)),

            Tok::If => {
                self.bump();
                let cond = self.parse_cond()?;
                let then = self.parse_block()?;
                let mut elifs = Vec::new();
                let mut els = None;
                loop {
                    if self.check(&Tok::Elif) {
                        self.bump();
                        let c = self.parse_cond()?;
                        let b = self.parse_block()?;
                        elifs.push((c, b));
                    } else if self.check(&Tok::Else) {
                        self.bump();
                        if self.check(&Tok::If) {
                            self.bump();
                            let c = self.parse_cond()?;
                            let b = self.parse_block()?;
                            elifs.push((c, b));
                        } else {
                            els = Some(self.parse_block()?);
                            break;
                        }
                    } else { break; }
                }
                Ok(Expr::If { cond: Box::new(cond), then, elifs, els })
            }

            Tok::Match => {
                self.bump();
                let scrutinee = self.parse_cond()?;
                self.expect(Tok::LBrace)?;
                let mut arms = Vec::new();
                while !self.check(&Tok::RBrace) {
                    let pattern = self.parse_pattern()?;
                    let guard = if self.eat(&Tok::If) { Some(self.parse_expr()?) } else { None };
                    self.expect(Tok::FatArrow)?;
                    let body = self.parse_expr()?;
                    arms.push(Arm { pattern, guard, body });
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RBrace)?;
                Ok(Expr::Match { scrutinee: Box::new(scrutinee), arms })
            }

            Tok::Try => {
                self.bump();
                let body = self.parse_block()?;
                let mut catches = Vec::new();
                while self.eat(&Tok::Catch) {
                    let binding = if matches!(self.peek(), Tok::Ident(_)) { Some(self.expect_ident()?) } else { None };
                    let body = self.parse_block()?;
                    catches.push(Catch { binding, body });
                }
                Ok(Expr::Try { body, catches })
            }

            Tok::Async => {
                self.bump();
                let inner = self.parse_expr()?;
                match inner {
                    Expr::Lambda { params, body, .. } => Ok(Expr::Lambda { params, body, is_async: true }),
                    other => Ok(other),
                }
            }

            other => self.err(format!("неожиданный токен в выражении: {:?}", other)),
        }
    }

    fn try_lambda(&mut self) -> Res<Option<Expr>> {
        let save = self.i;
        if !self.eat(&Tok::LParen) { return Ok(None); }
        let mut params = Vec::new();
        while let Tok::Ident(_) = self.peek() {
            let name = self.expect_ident()?;
            params.push(Param { mutable: false, name, ty: None });
            if !self.eat(&Tok::Comma) { break; }
        }
        if !self.eat(&Tok::RParen) || !self.eat(&Tok::FatArrow) {
            self.i = save;
            return Ok(None);
        }
        let body = self.parse_expr()?;
        Ok(Some(Expr::Lambda { params, body: Box::new(body), is_async: false }))
    }

    fn parse_pattern(&mut self) -> Res<Pattern> {
        let mut p = self.parse_pattern_atom()?;
        if self.check(&Tok::DotDot) || self.check(&Tok::DotDotDot) {
            let inclusive = self.check(&Tok::DotDotDot);
            self.bump();
            let end = self.parse_pattern_atom()?;
            p = Pattern::Range { start: Box::new(p), end: Box::new(end), inclusive };
        }
        Ok(p)
    }

    fn parse_pattern_atom(&mut self) -> Res<Pattern> {
        match self.peek().clone() {
            Tok::Int(n) => { self.bump(); Ok(Pattern::Lit(Expr::Int(n))) }
            Tok::Float(f) => { self.bump(); Ok(Pattern::Lit(Expr::Float(f))) }
            Tok::Str(s) => { self.bump(); Ok(Pattern::Lit(Expr::Str(s))) }
            Tok::Char(c) => { self.bump(); Ok(Pattern::Lit(Expr::Char(c))) }
            Tok::True => { self.bump(); Ok(Pattern::Lit(Expr::Bool(true))) }
            Tok::False => { self.bump(); Ok(Pattern::Lit(Expr::Bool(false))) }
            Tok::Null => { self.bump(); Ok(Pattern::Lit(Expr::Null)) }
            Tok::Ident(name) => {
                self.bump();
                if name == "_" { return Ok(Pattern::Wild); }
                let mut path = vec![name];
                while self.eat(&Tok::ColonColon) { path.push(self.expect_ident()?); }
                if self.check(&Tok::LParen) {
                    self.bump();
                    let mut args = Vec::new();
                    while !self.check(&Tok::RParen) {
                        args.push(self.parse_pattern()?);
                        if !self.eat(&Tok::Comma) { break; }
                    }
                    self.expect(Tok::RParen)?;
                    Ok(Pattern::Enum { path, args })
                } else if self.check(&Tok::LBrace) {
                    self.bump();
                    let mut fs = Vec::new();
                    while !self.check(&Tok::RBrace) {
                        let n = self.expect_ident()?;
                        let sub = if self.eat(&Tok::Colon) { self.parse_pattern()? } else { Pattern::Binding(n.clone()) };
                        fs.push((n, sub));
                        if !self.eat(&Tok::Comma) { break; }
                    }
                    self.expect(Tok::RBrace)?;
                    Ok(Pattern::Record(fs))
                } else if path.len() == 1 {
                    Ok(Pattern::Binding(path.pop().unwrap()))
                } else {
                    Ok(Pattern::Enum { path, args: Vec::new() })
                }
            }
            Tok::LParen => {
                self.bump();
                let mut items = Vec::new();
                while !self.check(&Tok::RParen) {
                    items.push(self.parse_pattern()?);
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RParen)?;
                Ok(Pattern::Tuple(items))
            }
            Tok::LBracket => {
                self.bump();
                let mut items = Vec::new();
                while !self.check(&Tok::RBracket) {
                    items.push(self.parse_pattern()?);
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RBracket)?;
                Ok(Pattern::Array(items))
            }
            other => self.err(format!("ожидался паттерн, получено {:?}", other)),
        }
    }

    fn parse_type(&mut self) -> Res<Type> {
        let base = match self.peek().clone() {
            Tok::Ident(s) => {
                self.bump();
                match s.as_str() {
                    "int" | "int32" | "int64" | "uint32" => Type::Int,
                    "float" => Type::Float,
                    "bool" => Type::Bool,
                    "char" => Type::Char,
                    "string" => Type::String,
                    "null" => Type::Null,
                    "any" => Type::Any,
                    _ => {
                        let mut path = vec![s];
                        while self.eat(&Tok::ColonColon) { path.push(self.expect_path_segment()?); }
                        let mut generics = Vec::new();
                        let close = if self.eat(&Tok::Lt) {
                            Some(Tok::Gt)
                        } else if self.eat(&Tok::LBracket) {
                            Some(Tok::RBracket)
                        } else {
                            None
                        };
                        if let Some(close) = close {
                            while !self.check(&close) {
                                generics.push(self.parse_type()?);
                                if !self.eat(&Tok::Comma) { break; }
                            }
                            self.expect(close)?;
                        }
                        Type::Named(path, generics)
                    }
                }
            }
            Tok::SelfKw => { self.bump(); Type::SelfTy }
            Tok::LBracket => {
                self.bump();
                let inner = self.parse_type()?;
                self.expect(Tok::RBracket)?;
                Type::Array(Box::new(inner))
            }
            Tok::LParen => {
                self.bump();
                let mut items = Vec::new();
                while !self.check(&Tok::RParen) {
                    items.push(self.parse_type()?);
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RParen)?;
                if items.len() == 1 { items.pop().unwrap() } else { Type::Tuple(items) }
            }
            Tok::LBrace => {
                self.bump();
                let mut fields = Vec::new();
                while !self.check(&Tok::RBrace) {
                    let n = self.expect_ident()?;
                    self.expect(Tok::Colon)?;
                    let t = self.parse_type()?;
                    fields.push((n, t));
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RBrace)?;
                Type::Record(fields)
            }
            Tok::Fn => {
                self.bump();
                self.expect(Tok::LParen)?;
                let mut ps = Vec::new();
                while !self.check(&Tok::RParen) {
                    ps.push(self.parse_type()?);
                    if !self.eat(&Tok::Comma) { break; }
                }
                self.expect(Tok::RParen)?;
                self.expect(Tok::Arrow)?;
                let r = self.parse_type()?;
                Type::Fn(ps, Box::new(r))
            }
            other => return self.err(format!("ожидался тип, получено {:?}", other)),
        };
        if self.eat(&Tok::Question) {
            Ok(Type::Option(Box::new(base)))
        } else {
            Ok(base)
        }
    }
}