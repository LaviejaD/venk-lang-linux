use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    Str(Rc<String>),

    Array(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<Vec<(MapKey, Value)>>>),
    Set(Rc<RefCell<Vec<MapKey>>>),
    Record(Rc<RefCell<Vec<(String, Value)>>>),
    Struct(Rc<String>, Rc<RefCell<Vec<(String, Value)>>>), // ДОБАВЛЕНО
    Tuple(Rc<Vec<Value>>),

    Fn(Rc<Closure>),
    Builtin(&'static str),

    Result(Result<Box<Value>, Box<Value>>),
    Option(Option<Box<Value>>),

    Range(i64, i64),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum MapKey {
    Int(i64),
    Str(Rc<String>),
    Bool(bool),
    Char(char),
}

pub struct Closure {
    pub params: Vec<Param>,
    pub body: FnBody,
    pub env: Rc<Env>,
    pub is_async: bool,
}

pub struct Binding {
    pub value: Rc<RefCell<Value>>,
    pub mutable: bool,
}

pub struct Env {
    pub parent: Option<Rc<Env>>,
    pub vars: RefCell<HashMap<String, Rc<Binding>>>,
}

impl Env {
    pub fn new(parent: Option<Rc<Env>>) -> Self {
        Env { parent, vars: RefCell::new(HashMap::new()) }
    }

    pub fn define(&self, name: String, value: Value, mutable: bool) -> Rc<Binding> {
        let b = Rc::new(Binding { value: Rc::new(RefCell::new(value)), mutable });
        self.vars.borrow_mut().insert(name, Rc::clone(&b));
        b
    }

    pub fn define_alias(&self, name: String, binding: Rc<Binding>) {
        self.vars.borrow_mut().insert(name, binding);
    }

    pub fn get(&self, name: &str) -> Option<Rc<Binding>> {
        if let Some(b) = self.vars.borrow().get(name) {
            return Some(Rc::clone(b));
        }
        if let Some(p) = &self.parent {
            return p.get(name);
        }
        None
    }

    pub fn snapshot(&self) -> Rc<Env> {
        let mut chain: Vec<Rc<Env>> = Vec::new();
        let mut cur = Some(self.snapshot_one());
        while let Some(c) = cur {
            let next = c.parent.clone();
            chain.push(c);
            cur = next;
        }
        let mut acc: Option<Rc<Env>> = None;
        for env in chain.into_iter().rev() {
            let new_env = Rc::new(Env {
                parent: acc.clone(),
                vars: RefCell::new(env.vars.borrow().clone()),
            });
            acc = Some(new_env);
        }
        acc.unwrap()
    }

    fn snapshot_one(&self) -> Rc<Env> {
        Rc::new(Env {
            parent: self.parent.clone(),
            vars: RefCell::new(self.vars.borrow().clone()),
        })
    }
}

impl Value {
    pub fn int(n: i64) -> Value { Value::Int(n) }
    pub fn float(f: f64) -> Value { Value::Float(f) }
    pub fn str(s: impl Into<String>) -> Value { Value::Str(Rc::new(s.into())) }
    pub fn bool(b: bool) -> Value { Value::Bool(b) }
    pub fn arr(v: Vec<Value>) -> Value { Value::Array(Rc::new(RefCell::new(v))) }
    pub fn opt_none() -> Value { Value::Option(None) }
    pub fn opt_some(v: Value) -> Value { Value::Option(Some(Box::new(v))) }
    pub fn res_ok(v: Value) -> Value { Value::Result(Ok(Box::new(v))) }
    pub fn res_err(v: Value) -> Value { Value::Result(Err(Box::new(v))) }

    pub fn type_name(&self) -> &str {
        match self {
            Value::Null => "null",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Char(_) => "char",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Map(_) => "map",
            Value::Set(_) => "set",
            Value::Record(_) => "record",
            Value::Struct(name, _) => name,
            Value::Tuple(_) => "tuple",
            Value::Fn(_) | Value::Builtin(_) => "function",
            Value::Result(_) => "Result",
            Value::Option(_) => "Option",
            Value::Range(_, _) => "range",
        }
    }

    pub fn to_display(&self) -> String {
        match self {
            Value::Null => "null".into(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                if f.is_nan() { "nan".into() }
                else if f.is_infinite() { if *f > 0.0 { "inf".into() } else { "-inf".into() } }
                else { format!("{}", f) }
            }
            Value::Bool(b) => b.to_string(),
            Value::Char(c) => c.to_string(),
            Value::Str(s) => (**s).clone(),
            Value::Array(a) => {
                let items: Vec<String> = a.borrow().iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Tuple(t) => {
                let items: Vec<String> = t.iter().map(|v| v.to_json_string()).collect();
                format!("({})", items.join(", "))
            }
            Value::Record(r) => {
                let items: Vec<String> = r.borrow().iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_json_string())).collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Struct(name, fields) => {
                let items: Vec<String> = fields.borrow().iter()
                    .map(|(k, v)| format!("{}: {}", k, v.to_json_string())).collect();
                format!("{} {{ {} }}", name, items.join(", "))
            }
            Value::Map(m) => {
                let items: Vec<String> = m.borrow().iter()
                    .map(|(k, v)| format!("{}: {}", k.to_display(), v.to_json_string())).collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Set(s) => {
                let items: Vec<String> = s.borrow().iter().map(|k| k.to_display()).collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Fn(_) => "<fn>".into(),
            Value::Builtin(n) => format!("<builtin {}>", n),
            Value::Result(Ok(v)) => format!("Ok({})", v.to_display()),
            Value::Result(Err(v)) => format!("Err({})", v.to_display()),
            Value::Option(Some(v)) => format!("Some({})", v.to_display()),
            Value::Option(None) => "None".into(),
            Value::Range(a, b) => format!("{}..{}", a, b),
        }
    }

    pub fn to_json_string(&self) -> String {
        match self {
            Value::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            _ => self.to_display(),
        }
    }

    pub fn to_key(&self) -> Option<MapKey> {
        match self {
            Value::Int(n) => Some(MapKey::Int(*n)),
            Value::Str(s) => Some(MapKey::Str(Rc::clone(s))),
            Value::Bool(b) => Some(MapKey::Bool(*b)),
            Value::Char(c) => Some(MapKey::Char(*c)),
            _ => None,
        }
    }
}

impl MapKey {
    pub fn to_display(&self) -> String {
        match self {
            MapKey::Int(n) => n.to_string(),
            MapKey::Str(s) => (**s).clone(),
            MapKey::Bool(b) => b.to_string(),
            MapKey::Char(c) => c.to_string(),
        }
    }
    pub fn to_value(&self) -> Value {
        match self {
            MapKey::Int(n) => Value::Int(*n),
            MapKey::Str(s) => Value::Str(Rc::clone(s)),
            MapKey::Bool(b) => Value::Bool(*b),
            MapKey::Char(c) => Value::Char(*c),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display())
    }
}