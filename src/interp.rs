use crate::ast::*;
use crate::parser;
use crate::value::*;
use crate::import::ImportManager;
use crate::lexer::Tok;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub enum Signal {
    Runtime(String),
    Return(Value),
    Break,
    Continue,
    Propagate(Value),
}

pub type R<T = Value> = Result<T, Signal>;

pub struct Interp {
    globals: Rc<Env>,
    stack: Vec<Rc<Env>>,
    fns: HashMap<String, Rc<FnDecl>>,
    consts: HashMap<String, Value>,
    structs: HashMap<String, StructDecl>,
    impls: HashMap<String, Vec<FnDecl>>,
}

impl Interp {
    pub fn new() -> Self {
        let g = Rc::new(Env::new(None));
        Interp {
            globals: Rc::clone(&g),
            stack: vec![g],
            fns: HashMap::new(),
            consts: HashMap::new(),
            structs: HashMap::new(),
            impls: HashMap::new(),
        }
    }

    pub fn run(&mut self, prog: &Program) -> Result<(), String> {
        for item in &prog.items {
            self.register(item).map_err(signal_to_string)?;
        }
        for item in &prog.items {
            if let Item::Stmt(s) = item {
                if let Err(sig) = self.exec_top(s) {
                    return Err(signal_to_string(sig));
                }
            }
        }
        if self.fns.contains_key("main") {
            let f = Rc::clone(&self.fns["main"]);
            if let Err(sig) = self.call_decl(f, vec![], vec![]) {
                return Err(signal_to_string(sig));
            }
        }
        Ok(())
    }

    fn register(&mut self, item: &Item) -> R<()> {
        match item {
            Item::Fn(f) => {
                self.fns.insert(f.name.clone(), Rc::new(f.clone()));
            }
            Item::Const { name, value, .. } => {
                let v = self.eval(value)?;
                self.consts.insert(name.clone(), v);
            }
            Item::Struct(s) => {
                self.structs.insert(s.name.clone(), s.clone());
            }
            Item::Impl(i) => {
                if let Type::Named(path, _) = &i.ty {
                    if let Some(name) = path.first() {
                        self.impls
                            .entry(name.clone())
                            .or_default()
                            .extend(i.methods.clone());
                    }
                }
            }
            Item::Import(decl) => {
                let mut path = decl.module.join("/");
                if !path.ends_with(".venk") && path.contains('/') {
                    path.push_str(".venk");
                }
                let prog = ImportManager::load_module(&path).map_err(Signal::Runtime)?;
                for imported_item in &prog.items {
                    self.register(imported_item)?;
                }
            }
            Item::Vipc {
                name: _,
                version: _,
                ..
            } => {}
            Item::Vip(toks) => {
                let pkg_name = if let Some(Tok::Ident(name)) = toks.get(1) {
                    name.clone()
                } else {
                    return Err(Signal::Runtime("vip: ожидалось имя пакета".into()));
                };
                let prog = ImportManager::load_module(&pkg_name).map_err(Signal::Runtime)?;
                for imported_item in &prog.items {
                    self.register(imported_item)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn exec_top(&mut self, s: &Stmt) -> R<()> {
        self.exec_stmt(s).map(|_| ())
    }

    pub(crate) fn env(&self) -> Rc<Env> {
        Rc::clone(self.stack.last().unwrap())
    }

    pub fn exec_repl_stmt(&mut self, s: &Stmt) -> R<()> {
        self.exec_stmt(s)
    }

    pub fn eval_repl_expr(&mut self, e: &Expr) -> R {
        self.eval(e)
    }

    pub fn register_repl(&mut self, item: &Item) -> R<()> {
        self.register(item)
    }

    fn exec_stmt(&mut self, s: &Stmt) -> R<()> {
        match s {
            Stmt::Let {
                mutable,
                name,
                value,
                ..
            } => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Null,
                };
                self.env().define(name.clone(), v, *mutable);
                Ok(())
            }
            Stmt::Expr { expr, .. } => {
                self.eval(expr)?;
                Ok(())
            }
            Stmt::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e)?,
                    None => Value::Null,
                };
                Err(Signal::Return(v))
            }
            Stmt::While { cond, body } => {
                loop {
                    let c = self.eval(cond)?;
                    if !c.is_truthy() {
                        break;
                    }
                    match self.exec_block(body) {
                        Ok(_) => {}
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            }
            Stmt::Loop(body) => {
                loop {
                    match self.exec_block(body) {
                        Ok(_) => {}
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            }
            Stmt::For {
                pattern,
                iter,
                body,
            } => {
                let it = self.eval(iter)?;
                let items: Vec<Value> = match it {
                    Value::Array(a) => a.borrow().clone(),
                    Value::Map(m) => m
                        .borrow()
                        .iter()
                        .map(|(k, v)| Value::Tuple(Rc::new(vec![k.to_value(), v.clone()])))
                        .collect(),
                    Value::Set(s) => s.borrow().iter().map(|k| k.to_value()).collect(),
                    Value::Str(st) => st.chars().map(Value::Char).collect(),
                    Value::Range(a, b) => (a..b).map(Value::Int).collect(),
                    v => {
                        return Err(Signal::Runtime(format!(
                            "не итерируемо: {}",
                            v.type_name()
                        )))
                    }
                };
                for item in items {
                    let child = Rc::new(Env::new(Some(self.env())));
                    self.stack.push(child);
                    self.bind_pattern(pattern, &item)?;
                    let r = self.exec_block(body);
                    self.stack.pop();
                    match r {
                        Ok(_) => {}
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => continue,
                        Err(e) => return Err(e),
                    }
                }
                Ok(())
            }
            Stmt::Break => Err(Signal::Break),
            Stmt::Continue => Err(Signal::Continue),
        }
    }

    fn exec_block(&mut self, b: &Block) -> R<()> {
        self.stack.push(Rc::new(Env::new(Some(self.env()))));
        let mut r = Ok(());
        for s in &b.stmts {
            if let Err(e) = self.exec_stmt(s) {
                r = Err(e);
                break;
            }
        }
        self.stack.pop();
        r
    }

    fn eval(&mut self, e: &Expr) -> R {
        match e {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Char(c) => Ok(Value::Char(*c)),
            Expr::Str(s) => self.eval_str(s),
            Expr::Ident(n) => self.lookup(n),
            Expr::Path(segs) => {
                let builtin_name = map_path_to_builtin(segs);
                if is_builtin(&builtin_name) {
                    return Ok(Value::Builtin(Box::leak(builtin_name.into_boxed_str())));
                }
                self.lookup(segs.last().unwrap())
            }
            Expr::Array(items) => {
                let mut vs = Vec::with_capacity(items.len());
                for i in items {
                    vs.push(self.eval(i)?);
                }
                Ok(Value::arr(vs))
            }
            Expr::Tuple(items) => {
                let mut vs = Vec::with_capacity(items.len());
                for i in items {
                    vs.push(self.eval(i)?);
                }
                Ok(Value::Tuple(Rc::new(vs)))
            }
            Expr::Record(fields) => {
                let mut vs = Vec::with_capacity(fields.len());
                for (k, v) in fields {
                    vs.push((k.clone(), self.eval(v)?));
                }
                Ok(Value::Record(Rc::new(RefCell::new(vs))))
            }
            Expr::StructInit(path, fields) => {
                let name = path.last().unwrap();
                if !self.structs.contains_key(name) {
                    return Err(Signal::Runtime(format!("struct '{}' не найден", name)));
                }
                let mut vs = Vec::with_capacity(fields.len());
                for (k, v) in fields {
                    vs.push((k.clone(), self.eval(v)?));
                }
                Ok(Value::Struct(
                    Rc::new(name.clone()),
                    Rc::new(RefCell::new(vs)),
                ))
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(expr)?;
                self.unop(*op, v)
            }
            Expr::Binary { op, lhs, rhs } => self.eval_bin(*op, lhs, rhs),
            Expr::Assign { target, op, value } => self.eval_assign(target, *op, value),
            Expr::Call { callee, args } => {
                let f = self.eval(callee)?;
                let mut vs = Vec::with_capacity(args.len());
                for a in args {
                    vs.push(self.eval(a)?);
                }
                self.call_value(f, vs, args)
            }
            Expr::Method { recv, name, args } => {
                let r = self.eval(recv)?;

                if let Value::Struct(sname, _) = &r {
                    let method_opt = if let Some(methods) = self.impls.get(&**sname) {
                        methods.iter().find(|m| m.name == *name).cloned()
                    } else {
                        None
                    };

                    if let Some(m) = method_opt {
                        let mut vs = Vec::with_capacity(args.len() + 1);
                        vs.push(r.clone());
                        for a in args {
                            vs.push(self.eval(a)?);
                        }

                        let mut arg_exprs = vec![Expr::Ident("self".into())];
                        arg_exprs.extend(args.clone());

                        let closure = Closure {
                            params: m.params.clone(),
                            body: m.body.clone(),
                            env: Rc::clone(&self.globals),
                            is_async: m.is_async,
                        };
                        return self.call_closure(Rc::new(closure), vs, &arg_exprs);
                    }
                }

                if let Value::Record(fields) = &r {
                    let method = fields
                        .borrow()
                        .iter()
                        .find(|(k, _)| k == name)
                        .map(|(_, v)| v.clone());
                    if let Some(m) = method {
                        let mut vs = Vec::with_capacity(args.len());
                        for a in args {
                            vs.push(self.eval(a)?);
                        }
                        return self.call_value(m, vs, args);
                    }
                }
                Err(Signal::Runtime(format!("метод '{}' не найден", name)))
            }
            Expr::Field { recv, name } => {
                let r = self.eval(recv)?;
                match &r {
                    Value::Record(fields) => {
                        if let Some((_, v)) = fields.borrow().iter().find(|(k, _)| k == name) {
                            Ok(v.clone())
                        } else {
                            Err(Signal::Runtime(format!("поле '{}' не найдено", name)))
                        }
                    }
                    Value::Struct(_, fields) => {
                        if let Some((_, v)) = fields.borrow().iter().find(|(k, _)| k == name) {
                            Ok(v.clone())
                        } else {
                            Err(Signal::Runtime(format!("поле '{}' не найдено", name)))
                        }
                    }
                    Value::Map(m) => {
                        let key = MapKey::Str(Rc::new(name.clone()));
                        m.borrow()
                            .iter()
                            .find(|(k, _)| *k == key)
                            .map(|(_, v)| v.clone())
                            .ok_or_else(|| Signal::Runtime(format!("ключ '{}' не найден", name)))
                    }
                    _ => Err(Signal::Runtime(format!(
                        "нельзя взять .{} у {}",
                        name,
                        r.type_name()
                    ))),
                }
            }
            Expr::OptField { recv, name } => {
                let r = self.eval(recv)?;
                match &r {
                    Value::Null => Ok(Value::Null),
                    Value::Option(None) => Ok(Value::Null),
                    Value::Option(Some(v)) => match &**v {
                        Value::Record(fields) | Value::Struct(_, fields) => Ok(fields
                            .borrow()
                            .iter()
                            .find(|(k, _)| k == name)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(Value::Null)),
                        _ => Ok(Value::Null),
                    },
                    Value::Record(fields) | Value::Struct(_, fields) => Ok(fields
                        .borrow()
                        .iter()
                        .find(|(k, _)| k == name)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(Value::Null)),
                    _ => Ok(Value::Null),
                }
            }
            Expr::Index { recv, index } => {
                let r = self.eval(recv)?;
                let i = self.eval(index)?;
                self.index_get(r, i)
            }
            Expr::If {
                cond,
                then,
                elifs,
                els,
            } => {
                if self.eval(cond)?.is_truthy() {
                    return self.eval_block_expr(then);
                }
                for (c, b) in elifs {
                    if self.eval(c)?.is_truthy() {
                        return self.eval_block_expr(b);
                    }
                }
                match els {
                    Some(b) => self.eval_block_expr(b),
                    None => Ok(Value::Null),
                }
            }
            Expr::Match { scrutinee, arms } => {
                let v = self.eval(scrutinee)?;
                for arm in arms {
                    let mut binds: Vec<(String, Value)> = Vec::new();
                    if self.match_pattern(&arm.pattern, &v, &mut binds) {
                        self.stack.push(Rc::new(Env::new(Some(self.env()))));
                        for (n, val) in binds {
                            self.env().define(n, val, false);
                        }
                        let guard_ok = match &arm.guard {
                            Some(g) => self.eval(g).map(|x| x.is_truthy()).unwrap_or(false),
                            None => true,
                        };
                        if guard_ok {
                            let r = self.eval(&arm.body);
                            self.stack.pop();
                            return r;
                        }
                        self.stack.pop();
                    }
                }
                Err(Signal::Runtime("match: ни один паттерн не совпал".into()))
            }
            Expr::Block(b) => self.eval_block_expr(b),
            Expr::Try { body, catches } => {
                self.stack.push(Rc::new(Env::new(Some(self.env()))));
                let r = self.eval_block_expr(body);
                self.stack.pop();
                let should_catch = match &r {
                    Ok(Value::Result(Err(_))) | Ok(Value::Option(None)) => !catches.is_empty(),
                    _ => false,
                };
                if should_catch {
                    let c = &catches[0];
                    self.stack.push(Rc::new(Env::new(Some(self.env()))));
                    if let Some(bind) = &c.binding {
                        self.env().define(bind.clone(), r.unwrap(), false);
                    }
                    let out = self.eval_block_expr(&c.body);
                    self.stack.pop();
                    out
                } else {
                    r
                }
            }
            Expr::Propagate(inner) => {
                let v = self.eval(inner)?;
                match v {
                    Value::Result(Ok(x)) => Ok(*x),
                    Value::Result(Err(e)) => Err(Signal::Propagate(Value::Result(Err(e)))),
                    Value::Option(Some(x)) => Ok(*x),
                    Value::Option(None) => Err(Signal::Propagate(Value::Option(None))),
                    other => Err(Signal::Runtime(format!("? к {}", other.type_name()))),
                }
            }
            Expr::Cast { expr, ty } => {
                let v = self.eval(expr)?;
                self.cast(v, ty)
            }
            Expr::IsType { expr, ty } => {
                let v = self.eval(expr)?;
                Ok(Value::Bool(matches_type(&v, ty)))
            }
            Expr::Sizeof(ty) => Ok(Value::Int(sizeof_type(ty))),
            Expr::Lambda {
                params,
                body,
                is_async,
            } => {
                let env = self.env().snapshot();
                Ok(Value::Fn(Rc::new(Closure {
                    params: params.clone(),
                    body: FnBody::Expr((**body).clone()),
                    env,
                    is_async: *is_async,
                })))
            }
        }
    }

    fn eval_block_expr(&mut self, b: &Block) -> R {
        self.stack.push(Rc::new(Env::new(Some(self.env()))));
        let mut last = Value::Null;
        let n = b.stmts.len();
        let mut err = None;
        for (i, s) in b.stmts.iter().enumerate() {
            if let Stmt::Expr { expr, semi: false } = s {
                if i == n - 1 {
                    match self.eval(expr) {
                        Ok(v) => last = v,
                        Err(e) => {
                            err = Some(e);
                            break;
                        }
                    }
                    continue;
                }
            }
            if let Err(e) = self.exec_stmt(s) {
                err = Some(e);
                break;
            }
        }
        self.stack.pop();
        match err {
            Some(e) => Err(e),
            None => Ok(last),
        }
    }

    fn eval_str(&mut self, s: &str) -> R {
        if !s.contains('{') {
            return Ok(Value::str(s));
        }
        let mut out = String::new();
        let mut i = 0;
        let bytes = s.as_bytes();
        while i < bytes.len() {
            let c = s[i..].chars().next().unwrap();
            if c == '{' {
                if s[i + 1..].starts_with('{') {
                    out.push('{');
                    i += 2;
                    continue;
                }
                if let Some(end) = s[i + 1..].find('}') {
                    let inner = s[i + 1..i + 1 + end].trim();
                    let v = self.eval_inline(inner)?;
                    out.push_str(&v.to_display());
                    i += end + 2;
                    continue;
                }
            } else if c == '}' && s[i + 1..].starts_with('}') {
                out.push('}');
                i += 2;
                continue;
            }
            out.push(c);
            i += c.len_utf8();
        }
        Ok(Value::str(out))
    }

    fn eval_inline(&mut self, src: &str) -> R {
        let toks = crate::lexer::lex(src).map_err(|e| Signal::Runtime(format!("{}", e)))?;
        let expr = parser::parse_expr_source(toks)
            .map_err(|e| Signal::Runtime(format!("{}", e)))?;
        self.eval(&expr)
    }

    fn lookup(&mut self, name: &str) -> R {
        if let Some(b) = self.env().get(name) {
            return Ok(b.value.borrow().clone());
        }
        if let Some(v) = self.consts.get(name) {
            return Ok(v.clone());
        }
        match name {
            "PI" => return Ok(Value::Float(std::f64::consts::PI)),
            "E" => return Ok(Value::Float(std::f64::consts::E)),
            "None" => return Ok(Value::Option(None)),
            _ => {}
        }

        if let Some(f) = self.fns.get(name) {
            let closure = Closure {
                params: f.params.clone(),
                body: f.body.clone(),
                env: Rc::clone(&self.globals),
                is_async: f.is_async,
            };
            return Ok(Value::Fn(Rc::new(closure)));
        }

        if is_builtin(name) {
            return Ok(Value::Builtin(Box::leak(name.to_string().into_boxed_str())));
        }
        Err(Signal::Runtime(format!("неизвестное имя '{}'", name)))
    }

    fn unop(&mut self, op: UnOp, v: Value) -> R {
        match op {
            UnOp::Neg => match v {
                Value::Int(n) => Ok(Value::Int(n.wrapping_neg())),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(Signal::Runtime("унарный '-' не для чисел".into())),
            },
            UnOp::Not => Ok(Value::Bool(!v.is_truthy())),
            UnOp::BitNot => match v {
                Value::Int(n) => Ok(Value::Int(!n)),
                _ => Err(Signal::Runtime("~ только для int".into())),
            },
        }
    }

    fn eval_bin(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr) -> R {
        match op {
            BinOp::And => {
                let l = self.eval(lhs)?;
                if !l.is_truthy() {
                    return Ok(Value::Bool(false));
                }
                return Ok(Value::Bool(self.eval(rhs)?.is_truthy()));
            }
            BinOp::Or => {
                let l = self.eval(lhs)?;
                if l.is_truthy() {
                    return Ok(Value::Bool(true));
                }
                return Ok(Value::Bool(self.eval(rhs)?.is_truthy()));
            }
            BinOp::Default => {
                let l = self.eval(lhs)?;
                return match l {
                    Value::Null | Value::Option(None) => self.eval(rhs),
                    Value::Result(Err(_)) => self.eval(rhs),
                    Value::Option(Some(v)) => Ok(*v),
                    Value::Result(Ok(v)) => Ok(*v),
                    other => Ok(other),
                };
            }
            BinOp::Pipe => {
                let l = self.eval(lhs)?;
                return self.eval_pipe(l, rhs);
            }
            _ => {}
        }
        let l = self.eval(lhs)?;
        let r = self.eval(rhs)?;
        self.binop(op, l, r)
    }

    fn eval_pipe(&mut self, l: Value, rhs: &Expr) -> R {
        if let Expr::Call { callee, args } = rhs {
            let f = self.eval(callee)?;
            let mut vs = Vec::with_capacity(args.len() + 1);
            vs.push(l);
            for a in args {
                vs.push(self.eval(a)?);
            }
            let mut arg_exprs: Vec<Expr> = vec![Expr::Null];
            arg_exprs.extend(args.clone());
            return self.call_value(f, vs, &arg_exprs);
        }
        let f = self.eval(rhs)?;
        self.call_value(f, vec![l], &[])
    }

    fn binop(&mut self, op: BinOp, l: Value, r: Value) -> R {
        use Value::*;
        match op {
            BinOp::Add => match (l, r) {
                (Int(a), Int(b)) => Ok(Value::Int(a.wrapping_add(b))),
                (Float(a), Float(b)) => Ok(Value::Float(a + b)),
                (Int(a), Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Float(a), Int(b)) => Ok(Value::Float(a + b as f64)),
                (Str(a), Str(b)) => Ok(Value::str(format!("{}{}", a, b))),
                (Array(a), Array(b)) => {
                    let mut v = a.borrow().clone();
                    v.extend(b.borrow().iter().cloned());
                    Ok(Value::arr(v))
                }
                _ => Err(Signal::Runtime("неверные типы для +".into())),
            },
            BinOp::Sub => match (l, r) {
                (Int(a), Int(b)) => Ok(Value::Int(a.wrapping_sub(b))),
                (Float(a), Float(b)) => Ok(Value::Float(a - b)),
                (Int(a), Float(b)) => Ok(Value::Float(a as f64 - b)),
                (Float(a), Int(b)) => Ok(Value::Float(a - b as f64)),
                _ => Err(Signal::Runtime("неверные типы для -".into())),
            },
            BinOp::Mul => match (l, r) {
                (Int(a), Int(b)) => Ok(Value::Int(a.wrapping_mul(b))),
                (Float(a), Float(b)) => Ok(Value::Float(a * b)),
                (Int(a), Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Float(a), Int(b)) => Ok(Value::Float(a * b as f64)),
                (Str(s), Int(n)) => Ok(Value::str(s.as_str().repeat(n.max(0) as usize))),
                _ => Err(Signal::Runtime("неверные типы для *".into())),
            },
            BinOp::Div => match (l, r) {
                (Int(a), Int(b)) => {
                    if b == 0 {
                        return Err(Signal::Runtime("деление на ноль (int)".into()));
                    }
                    Ok(Value::Int(a / b))
                }
                (Float(a), Float(b)) => Ok(Value::Float(a / b)),
                (Int(a), Float(b)) => Ok(Value::Float(a as f64 / b)),
                (Float(a), Int(b)) => Ok(Value::Float(a / b as f64)),
                _ => Err(Signal::Runtime("неверные типы для /".into())),
            },
            BinOp::Rem => match (l, r) {
                (Int(a), Int(b)) => {
                    if b == 0 {
                        return Err(Signal::Runtime("%: деление на ноль".into()));
                    }
                    Ok(Value::Int(a % b))
                }
                (Float(a), Float(b)) => Ok(Value::Float(a % b)),
                _ => Err(Signal::Runtime("% только для чисел".into())),
            },
            BinOp::Pow => {
                let a = self.as_float(&l)?;
                let b = self.as_float(&r)?;
                Ok(Value::Float(a.powf(b)))
            }
            BinOp::Eq => Ok(Value::Bool(self.values_eq(&l, &r))),
            BinOp::Ne => Ok(Value::Bool(!self.values_eq(&l, &r))),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let ord = match (&l, &r) {
                    (Int(a), Int(b)) => a.partial_cmp(b),
                    (Float(a), Float(b)) => a.partial_cmp(b),
                    (Int(a), Float(b)) => (*a as f64).partial_cmp(b),
                    (Float(a), Int(b)) => a.partial_cmp(&(*b as f64)),
                    (Str(a), Str(b)) => a.partial_cmp(b),
                    (Char(a), Char(b)) => a.partial_cmp(b),
                    _ => return Err(Signal::Runtime("неверные типы для сравнения".into())),
                };
                let ord = ord.unwrap_or(std::cmp::Ordering::Equal);
                use std::cmp::Ordering::*;
                Ok(Value::Bool(match op {
                    BinOp::Lt => ord == Less,
                    BinOp::Le => ord != Greater,
                    BinOp::Gt => ord == Greater,
                    BinOp::Ge => ord != Less,
                    _ => unreachable!(),
                }))
            }
            BinOp::BitAnd => match (l, r) {
                (Int(a), Int(b)) => Ok(Value::Int(a & b)),
                _ => Err(Signal::Runtime("& только для int".into())),
            },
            BinOp::BitOr => match (l, r) {
                (Int(a), Int(b)) => Ok(Value::Int(a | b)),
                _ => Err(Signal::Runtime("| только для int".into())),
            },
            BinOp::BitXor => match (l, r) {
                (Int(a), Int(b)) => Ok(Value::Int(a ^ b)),
                _ => Err(Signal::Runtime("^ только для int".into())),
            },
            BinOp::Shl => match (l, r) {
                (Int(a), Int(b)) => Ok(Value::Int(a.wrapping_shl(b as u32))),
                _ => Err(Signal::Runtime("<< только для int".into())),
            },
            BinOp::Shr => match (l, r) {
                (Int(a), Int(b)) => Ok(Value::Int(a.wrapping_shr(b as u32))),
                _ => Err(Signal::Runtime(">> только для int".into())),
            },
            BinOp::And | BinOp::Or | BinOp::Default | BinOp::Pipe => unreachable!(),
        }
    }

    fn as_float(&self, v: &Value) -> Result<f64, Signal> {
        match v {
            Value::Int(n) => Ok(*n as f64),
            Value::Float(f) => Ok(*f),
            _ => Err(Signal::Runtime("ожидалось число".into())),
        }
    }

    pub(crate) fn values_eq(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(x), Value::Float(y)) => (*x as f64) == *y,
            (Value::Float(x), Value::Int(y)) => *x == (*y as f64),
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Char(x), Value::Char(y)) => x == y,
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Option(None), Value::Option(None)) => true,
            (Value::Option(Some(x)), Value::Option(Some(y))) => self.values_eq(x, y),
            (Value::Result(Ok(x)), Value::Result(Ok(y))) => self.values_eq(x, y),
            (Value::Result(Err(x)), Value::Result(Err(y))) => self.values_eq(x, y),
            (Value::Tuple(x), Value::Tuple(y)) => {
                x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| self.values_eq(a, b))
            }
            (Value::Array(x), Value::Array(y)) => {
                let x = x.borrow();
                let y = y.borrow();
                x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| self.values_eq(a, b))
            }
            (Value::Record(x), Value::Record(y)) => {
                let x = x.borrow();
                let y = y.borrow();
                if x.len() != y.len() {
                    return false;
                }
                x.iter()
                    .all(|(k, v)| y.iter().any(|(k2, v2)| k == k2 && self.values_eq(v, v2)))
            }
            (Value::Struct(n1, x), Value::Struct(n2, y)) => {
                if n1 != n2 {
                    return false;
                }
                let x = x.borrow();
                let y = y.borrow();
                if x.len() != y.len() {
                    return false;
                }
                x.iter()
                    .all(|(k, v)| y.iter().any(|(k2, v2)| k == k2 && self.values_eq(v, v2)))
            }
            _ => false,
        }
    }

    fn eval_assign(&mut self, target: &Expr, op: AssignOp, value: &Expr) -> R {
        match target {
            Expr::Ident(name) => {
                let binding = self
                    .env()
                    .get(name)
                    .or_else(|| self.globals.get(name))
                    .ok_or_else(|| Signal::Runtime(format!("неизвестная переменная '{}'", name)))?;
                if !binding.mutable {
                    return Err(Signal::Runtime(format!("'{}' объявлена как let", name)));
                }
                let new_val = if matches!(op, AssignOp::Set) {
                    self.eval(value)?
                } else {
                    let cur = binding.value.borrow().clone();
                    let rhs = self.eval(value)?;
                    self.binop(assign_to_bin(op), cur, rhs)?
                };
                *binding.value.borrow_mut() = new_val;
                Ok(Value::Null)
            }
            Expr::Index { recv, index } => {
                let container = self.eval(recv)?;
                let key = self.eval(index)?;
                let rhs = self.eval(value)?;
                self.index_set(container, key, rhs)
            }
            Expr::Field { recv, name } => {
                let container = self.eval(recv)?;
                let rhs = self.eval(value)?;
                match container {
                    Value::Record(fields) => {
                        let mut f = fields.borrow_mut();
                        if let Some(slot) = f.iter_mut().find(|(k, _)| k == name) {
                            slot.1 = rhs;
                        } else {
                            f.push((name.clone(), rhs));
                        }
                        Ok(Value::Null)
                    }
                    Value::Struct(_, fields) => {
                        let mut f = fields.borrow_mut();
                        if let Some(slot) = f.iter_mut().find(|(k, _)| k == name) {
                            slot.1 = rhs;
                        } else {
                            f.push((name.clone(), rhs));
                        }
                        Ok(Value::Null)
                    }
                    Value::Map(m) => {
                        let key = MapKey::Str(Rc::new(name.clone()));
                        let mut mm = m.borrow_mut();
                        if let Some(slot) = mm.iter_mut().find(|(k, _)| *k == key) {
                            slot.1 = rhs;
                        } else {
                            mm.push((key, rhs));
                        }
                        Ok(Value::Null)
                    }
                    other => Err(Signal::Runtime(format!(
                        "нельзя присвоить .{} у {}",
                        name,
                        other.type_name()
                    ))),
                }
            }
            _ => Err(Signal::Runtime("неверная цель присваивания".into())),
        }
    }

    fn index_get(&self, v: Value, i: Value) -> R {
        match (&v, &i) {
            (Value::Array(a), Value::Int(n)) => {
                let n = normalize_index(*n, a.borrow().len());
                a.borrow()
                    .get(n)
                    .cloned()
                    .ok_or_else(|| Signal::Runtime("индекс вне границ".into()))
            }
            (Value::Str(s), Value::Int(n)) => {
                let n = normalize_index(*n, s.len());
                s.as_bytes()
                    .get(n)
                    .map(|b| Value::Int(*b as i64))
                    .ok_or_else(|| Signal::Runtime("индекс вне границ".into()))
            }
            (Value::Map(m), _) => {
                let key = i.to_key().ok_or_else(|| Signal::Runtime("невалидный ключ map".into()))?;
                m.borrow()
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.clone())
                    .ok_or_else(|| Signal::Runtime(format!("ключ {} не найден", key.to_display())))
            }
            (Value::Tuple(t), Value::Int(n)) => {
                let n = normalize_index(*n, t.len());
                t.get(n)
                    .cloned()
                    .ok_or_else(|| Signal::Runtime("индекс вне границ".into()))
            }
            _ => Err(Signal::Runtime("неверные типы для индексации".into())),
        }
    }

    fn index_set(&mut self, container: Value, key: Value, rhs: Value) -> R {
        match container {
            Value::Array(a) => {
                let n = match key {
                    Value::Int(n) => normalize_index(n, a.borrow().len()),
                    _ => return Err(Signal::Runtime("индекс массива — int".into())),
                };
                let mut arr = a.borrow_mut();
                if n >= arr.len() {
                    return Err(Signal::Runtime("индекс вне границ".into()));
                }
                arr[n] = rhs;
                Ok(Value::Null)
            }
            Value::Map(m) => {
                let k = key.to_key().ok_or_else(|| Signal::Runtime("невалидный ключ map".into()))?;
                let mut mm = m.borrow_mut();
                if let Some(slot) = mm.iter_mut().find(|(kk, _)| *kk == k) {
                    slot.1 = rhs;
                } else {
                    mm.push((k, rhs));
                }
                Ok(Value::Null)
            }
            other => Err(Signal::Runtime(format!(
                "нельзя индексировать {}",
                other.type_name()
            ))),
        }
    }

    pub(crate) fn call_value(&mut self, f: Value, args: Vec<Value>, arg_exprs: &[Expr]) -> R {
        match f {
            Value::Fn(c) => self.call_closure(c, args, arg_exprs),
            Value::Builtin(name) => self.call_builtin(name, args, arg_exprs),
            other => Err(Signal::Runtime(format!(
                "{} не вызываемо",
                other.type_name()
            ))),
        }
    }

    fn call_closure(&mut self, c: Rc<Closure>, args: Vec<Value>, arg_exprs: &[Expr]) -> R {
        let env = Rc::new(Env::new(Some(Rc::clone(&c.env))));
        for (i, p) in c.params.iter().enumerate() {
            let v = args.get(i).cloned().unwrap_or(Value::Null);
            if p.mutable {
                if let Some(Expr::Ident(name)) = arg_exprs.get(i) {
                    if let Some(b) = self.env().get(name) {
                        if !b.mutable {
                            return Err(Signal::Runtime(format!("'{}' должен быть mut", name)));
                        }
                        env.define_alias(p.name.clone(), b);
                        continue;
                    }
                }
            }
            env.define(p.name.clone(), v, p.mutable);
        }
        self.stack.push(env);
        let r = match &c.body {
            FnBody::Expr(e) => self.eval(e),
            FnBody::Block(b) => self.eval_block_expr(b),
            FnBody::None => Ok(Value::Null),
        };
        self.stack.pop();
        match r {
            Ok(v) => Ok(v),
            Err(Signal::Return(v)) => Ok(v),
            Err(Signal::Propagate(v)) => Ok(v),
            Err(e) => Err(e),
        }
    }

    fn call_decl(&mut self, f: Rc<FnDecl>, args: Vec<Value>, arg_exprs: Vec<Expr>) -> R {
        let closure = Closure {
            params: f.params.clone(),
            body: f.body.clone(),
            env: Rc::clone(&self.globals),
            is_async: f.is_async,
        };
        self.call_closure(Rc::new(closure), args, &arg_exprs)
    }

    fn match_pattern(&self, p: &Pattern, v: &Value, binds: &mut Vec<(String, Value)>) -> bool {
        match p {
            Pattern::Wild => true,
            Pattern::Binding(name) => {
                if name == "_" {
                    return true;
                }
                binds.push((name.clone(), v.clone()));
                true
            }
            Pattern::Lit(e) => {
                if let Ok(lit) = self.eval_lit(e) {
                    self.values_eq(&lit, v)
                } else {
                    false
                }
            }
            Pattern::Tuple(ps) => {
                if let Value::Tuple(t) = v {
                    if ps.len() != t.len() {
                        return false;
                    }
                    for (pat, item) in ps.iter().zip(t.iter()) {
                        if !self.match_pattern(pat, item, binds) {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            }
            Pattern::Array(ps) => {
                if let Value::Array(a) = v {
                    let a = a.borrow();
                    if ps.len() != a.len() {
                        return false;
                    }
                    for (pat, item) in ps.iter().zip(a.iter()) {
                        if !self.match_pattern(pat, item, binds) {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            }
            Pattern::Record(fs) => {
                if let Value::Record(r) | Value::Struct(_, r) = v {
                    let r = r.borrow();
                    for (k, pat) in fs {
                        if let Some((_, val)) = r.iter().find(|(kk, _)| kk == k) {
                            if !self.match_pattern(pat, val, binds) {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            }
            Pattern::Enum { .. } => false,
            Pattern::Range {
                start,
                end,
                inclusive,
            } => {
                let s = match self.eval_pattern_lit(start) {
                    Some(v) => v,
                    None => return false,
                };
                let e = match self.eval_pattern_lit(end) {
                    Some(v) => v,
                    None => return false,
                };
                if let (Value::Int(x), Value::Int(a), Value::Int(b)) = (v, &s, &e) {
                    if *inclusive {
                        *x >= *a && *x <= *b
                    } else {
                        *x >= *a && *x < *b
                    }
                } else {
                    false
                }
            }
        }
    }

    fn eval_lit(&self, e: &Expr) -> Result<Value, ()> {
        match e {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Char(c) => Ok(Value::Char(*c)),
            Expr::Str(s) => Ok(Value::str(s.clone())),
            Expr::Null => Ok(Value::Null),
            _ => Err(()),
        }
    }

    fn eval_pattern_lit(&self, p: &Pattern) -> Option<Value> {
        match p {
            Pattern::Lit(e) => self.eval_lit(e).ok(),
            _ => None,
        }
    }

    fn bind_pattern(&mut self, p: &Pattern, v: &Value) -> R<()> {
        match p {
            Pattern::Wild => Ok(()),
            Pattern::Binding(name) => {
                self.env().define(name.clone(), v.clone(), true);
                Ok(())
            }
            Pattern::Tuple(ps) | Pattern::Array(ps) => {
                let items: Vec<Value> = match v {
                    Value::Tuple(t) => t.to_vec(),
                    Value::Array(a) => a.borrow().clone(),
                    _ => {
                        return Err(Signal::Runtime(
                            "деструктуризация: не кортеж/массив".into(),
                        ))
                    }
                };
                for (pat, item) in ps.iter().zip(items.iter()) {
                    self.bind_pattern(pat, item)?;
                }
                Ok(())
            }
            Pattern::Record(fs) => {
                let fields = match v {
                    Value::Record(r) | Value::Struct(_, r) => r.clone(),
                    _ => return Err(Signal::Runtime("деструктуризация: не record".into())),
                };
                for (k, pat) in fs {
                    let val = fields
                        .borrow()
                        .iter()
                        .find(|(kk, _)| kk == k)
                        .map(|(_, v)| v.clone());
                    if let Some(val) = val {
                        self.bind_pattern(pat, &val)?;
                    }
                }
                Ok(())
            }
            _ => Err(Signal::Runtime("for: неподдерживаемый паттерн".into())),
        }
    }

    fn cast(&self, v: Value, ty: &Type) -> R {
        match ty {
            Type::Int => match v {
                Value::Int(n) => Ok(Value::Int(n)),
                Value::Float(f) => Ok(Value::Int(f as i64)),
                Value::Bool(b) => Ok(Value::Int(b as i64)),
                Value::Str(s) => Ok(Value::Int(s.parse::<i64>().unwrap_or(0))),
                Value::Char(c) => Ok(Value::Int(c as i64)),
                _ => Err(Signal::Runtime("cast to int".into())),
            },
            Type::Float => match v {
                Value::Int(n) => Ok(Value::Float(n as f64)),
                Value::Float(f) => Ok(Value::Float(f)),
                Value::Str(s) => Ok(Value::Float(s.parse::<f64>().unwrap_or(0.0))),
                _ => Err(Signal::Runtime("cast to float".into())),
            },
            Type::String => Ok(Value::str(v.to_display())),
            Type::Bool => Ok(Value::Bool(v.is_truthy())),
            Type::Char => match v {
                Value::Str(s) => s
                    .chars()
                    .next()
                    .map(Value::Char)
                    .ok_or_else(|| Signal::Runtime("пустая строка → char".into())),
                _ => Err(Signal::Runtime("cast to char".into())),
            },
            _ => Err(Signal::Runtime("cast: неподдерживаемый тип".into())),
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Array(a) => !a.borrow().is_empty(),
            Value::Map(m) => !m.borrow().is_empty(),
            Value::Set(s) => !s.borrow().is_empty(),
            Value::Record(r) | Value::Struct(_, r) => !r.borrow().is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Option(None) => false,
            Value::Option(Some(v)) => v.is_truthy(),
            Value::Result(Ok(v)) => v.is_truthy(),
            Value::Result(Err(_)) => false,
            _ => true,
        }
    }
}

fn normalize_index(i: i64, len: usize) -> usize {
    if i < 0 {
        (len as i64 + i).max(0) as usize
    } else {
        i as usize
    }
}

fn assign_to_bin(op: AssignOp) -> BinOp {
    match op {
        AssignOp::Add => BinOp::Add,
        AssignOp::Sub => BinOp::Sub,
        AssignOp::Mul => BinOp::Mul,
        AssignOp::Div => BinOp::Div,
        AssignOp::Rem => BinOp::Rem,
        AssignOp::Set => unreachable!(),
    }
}

fn matches_type(v: &Value, ty: &Type) -> bool {
    match ty {
        Type::Int => matches!(v, Value::Int(_)),
        Type::Float => matches!(v, Value::Float(_)),
        Type::Bool => matches!(v, Value::Bool(_)),
        Type::Char => matches!(v, Value::Char(_)),
        Type::String => matches!(v, Value::Str(_)),
        Type::Null => matches!(v, Value::Null),
        Type::Any => true,
        Type::Array(_) => matches!(v, Value::Array(_)),
        Type::Tuple(_) => matches!(v, Value::Tuple(_)),
        Type::Record(_) => matches!(v, Value::Record(_)),
        Type::Named(n, _) => match n.first().map(|s| s.as_str()) {
            Some("Result") => matches!(v, Value::Result(_)),
            Some(name) => matches!(v, Value::Struct(s, _) if &**s == name),
            _ => false,
        },
        Type::Option(_) => matches!(v, Value::Option(_) | Value::Null),
        Type::Fn(_, _) => matches!(v, Value::Fn(_) | Value::Builtin(_)),
        _ => false,
    }
}

fn sizeof_type(ty: &Type) -> i64 {
    match ty {
        Type::Int | Type::Float => 8,
        Type::Bool => 1,
        Type::Char => 4,
        Type::String | Type::Array(_) | Type::Record(_) => 8,
        _ => 8,
    }
}

fn signal_to_string(s: Signal) -> String {
    match s {
        Signal::Runtime(m) => format!("runtime error: {}", m),
        Signal::Return(v) => format!("неожиданный return: {}", v.to_display()),
        Signal::Break => "break вне цикла".into(),
        Signal::Continue => "continue вне цикла".into(),
        Signal::Propagate(v) => format!("необработанный ?: {}", v.to_display()),
    }
}

// ==========================================
// МАГИЯ КОРОТКИХ ИМЕН МОДУЛЕЙ
// ==========================================
fn map_path_to_builtin(segs: &[String]) -> String {
    if segs.is_empty() {
        return "".to_string();
    }
    let m = segs[0].as_str();
    let rest = segs[1..].join("_");

    match m {
        "os" | "fs" | "file" => format!("pvenk_{}", rest),
        "re" | "regex" => format!("pvenk_regex_{}", rest),
        "time" => format!("pvenk_time_{}", rest),
        "json" => format!("jvenk_json_{}", rest),
        "http" => format!("jvenk_http_{}", rest),
        "url" => format!("jvenk_url_{}", rest),
        "vec" | "vec2" => {
            if rest == "create" || rest == "new" {
                "gdvenk_vec2".to_string()
            } else {
                format!("gdvenk_vec2_{}", rest)
            }
        }
        "signal" => format!("gdvenk_signal_{}", rest),
        "timer" => format!("gdvenk_timer_{}", rest),
        "lua" => format!("lvenk_{}", rest),
        "table" => format!("lvenk_table_{}", rest),
        "sys" => format!("cppvenk_os_{}", rest),
        "mem" => format!("cppvenk_mem_{}", rest),
        "thread" => format!("cppvenk_thread_{}", rest),
        "queue" => format!("cppvenk_queue_{}", rest),
        // 3D МОДУЛИ
        "vec3" => format!("vek3d_vec3_{}", rest),
        "camera3d" => format!("vek3d_camera_{}", rest),
        "mesh" => format!("vek3d_mesh_{}", rest),
        "model3d" | "model" => format!("vek3d_model_{}", rest),
        "ray3d" | "ray" => format!("vek3d_ray_{}", rest),
        "cube3d" => format!("vek3d_draw_cube_{}", rest),
        "sphere3d" => format!("vek3d_draw_sphere_{}", rest),
        "cylinder" => format!("vek3d_draw_cylinder_{}", rest),
        "plane3d" => format!("vek3d_draw_plane_{}", rest),
        "grid3d" => "vek3d_draw_grid".to_string(),
        _ => segs.join("_"),
    }
}

fn is_builtin(name: &str) -> bool {
    if name.starts_with("pvenk_")
        || name.starts_with("jvenk_")
        || name.starts_with("gdvenk_")
        || name.starts_with("lvenk_")
        || name.starts_with("cppvenk_")
        || name.starts_with("vek3d_")
    {
        return true;
    }

    matches!(
        name,
        // Базовые функции
        "build" |
        "abs" | "sign" | "min" | "max" | "clamp" | "sum" | "avg" |
        "floor" | "ceil" | "round" | "sqrt" | "cbrt" | "pow" |
        "exp" | "log" | "log2" | "log10" | "sin" | "cos" | "tan" |
        "random" | "random_int" | "random_float" | "random_bool" | "seed" |
        "len" | "char_count" | "upper" | "lower" | "capitalize" | "title" |
        "trim" | "trim_start" | "trim_end" | "split" | "join" |
        "replace" | "replace_all" | "starts_with" | "ends_with" | "includes" |
        "index_of" | "slice" | "repeat" | "pad_start" | "pad_end" |
        "chars" | "bytes" |
        "push" | "pop" | "shift" | "unshift" | "insert" | "remove_at" |
        "first" | "last" | "fill" | "range" |
        "concat" | "reversed" | "unique" | "chunk" | "flatten" | "zip" |
        "with" | "without_last" | "enumerate" |
        "keys" | "values" | "entries" | "get" | "get_or" | "set" | "has" | "delete" | "merge" |
        "clone" | "from_entries" | "to_entries" |
        "set_create" | "set_add" | "set_remove" | "set_has" | "set_union" |
        "set_intersect" | "set_difference" | "set_to_array" |
        "to_int" | "to_float" | "to_string" | "to_bool" | "to_array" | "to_map" | "to_set" |
        "parse_int" | "parse_float" | "parse_bool" |
        "type_of" | "is_null" | "is_number" | "is_int" | "is_float" | "is_string" |
        "is_bool" | "is_array" | "is_map" | "is_function" |
        "map" | "filter" | "reduce" | "for_each" | "find" | "find_index" |
        "some" | "every" | "group_by" | "partition" |
        "sorted" | "sorted_by" | "sorted_by_key" | "sorted_desc" |
        "binary_search" | "linear_search" | "min_by" | "max_by" | "top_k" | "count_by" |
        "md5" | "sha1" | "sha256" | "hmac_sha256" |
        "base64_encode" | "base64_decode" | "hex_encode" | "hex_decode" |
        "url_encode" | "url_decode" |
        "now" | "now_ms" | "now_unix" | "sleep" |
        "read_file" | "write_file" | "append_file" | "file_exists" | "read_lines" | "write_lines" |
        "print" | "input" | "env_get" | "args" | "cwd" | "platform" | "exit" | "assert" | "panic" |
        "Ok" | "Err" | "Some" |
        
        // Окно, время и система
        "init_window" | "close_window" | "should_close" | "begin_drawing" | "end_drawing" |
        "clear" | "clear_alpha" | "screen_width" | "screen_height" | "set_fps" | "get_fps" |
        "get_frame_time" | "toggle_fullscreen" | "set_window_title" | "set_window_icon" |
        "set_window_resizable" | "set_vsync" | "time_delta" | "get_time" | "set_time" | "get_time_speed" |

        // Векторы 2D
        "vec2_add" | "vec2_sub" | "vec2_mul" | "vec2_div" | "vec2_length" | "vec2_normalize" |
        "vec2_dot" | "vec2_distance" | "vec2_lerp" | "vec2_rotate" | "angle_from_vec" | "vec_from_angle" |

        // Текстуры и спрайты
        "load_texture" | "unload_texture" | "get_texture_width" | "get_texture_height" |
        "draw_sprite" | "draw_anim" | "draw_texture_pro" | "draw_texture_tiled" |

        // Шрифты и текст
        "load_font" | "load_system_font" | "draw_text_ex" | "draw_text_centered" | "draw_text_shadow" |
        "draw_text_wrapped" | "measure_text" | "measure_text_ex" | "text" |

        // Примитивы 2D и UI
        "line" | "rect" | "rect_alpha" | "rect_rounded" | "rect_gradient_v" | "rect_gradient_h" |
        "rect_lines" | "circle" | "circle_lines" | "circle_sector" | "ring" | "triangle" |
        "triangle_lines" | "poly" | "ellipse" | "pixel" | "draw_health_bar" |
        "gui_button" | "gui_button_font" | "gui_slider" |

        // Коллизии 2D, камера, ввод
        "collision" | "collision_circle" | "collision_circle_rect" | "get_distance" | "get_angle" |
        "lerp" | "apply_gravity" | "begin_camera" | "end_camera" | "shake_camera" |
        "camera_screen_to_world" | "camera_world_to_screen" |
        "key_down" | "key_pressed" | "is_key_up" | "is_key_released" | "get_char_pressed" |
        "mouse_click" | "mouse_button_down" | "mouse_button_released" | "mouse_pos" |
        "mouse_x" | "mouse_y" | "mouse_delta_x" | "mouse_delta_y" | "get_mouse_wheel" |
        "set_mouse_position" | "mouse_in_rect" |

        // Аудио и сеть
        "load_sound" | "play_sound" | "stop_sound" | "unload_sound" |
        "load_music" | "play_music" | "update_music" | "stop_music" |
        "open_url" | "http_get" | "json_parse" | "draw_tilemap" | "gen_map" | "eval"
    )
}