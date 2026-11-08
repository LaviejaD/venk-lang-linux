use crate::interp::{Interp, Signal, R};
use crate::value::*;
use std::cell::RefCell;
use std::rc::Rc;
use regex::Regex;

impl Interp {
    pub(crate) fn call_lvenk(&mut self, name: &str, mut args: Vec<Value>, arg_exprs: &[crate::ast::Expr]) -> R {
        // Локальные макросы
        macro_rules! s {
            ($v:expr, $name:expr) => {
                match $v {
                    Value::Str(s) => s,
                    other => return Err(Signal::Runtime(format!("{}: ожидалась строка, получен {}", $name, other.type_name()))),
                }
            };
        }
        macro_rules! int {
            ($v:expr, $name:expr) => {
                match $v {
                    Value::Int(n) => n,
                    other => return Err(Signal::Runtime(format!("{}: ожидалось число, получен {}", $name, other.type_name()))),
                }
            };
        }
        macro_rules! num {
            ($v:expr, $name:expr) => {
                match $v {
                    Value::Int(n) => n as f64,
                    Value::Float(f) => f,
                    other => return Err(Signal::Runtime(format!("{}: ожидалось число, получен {}", $name, other.type_name()))),
                }
            };
        }
        macro_rules! arr {
            ($v:expr, $name:expr) => {
                match $v {
                    Value::Array(a) => a,
                    other => return Err(Signal::Runtime(format!("{}: ожидался массив, получен {}", $name, other.type_name()))),
                }
            };
        }

        match name {
            // ==========================================
            // LUA PATTERNS (Строки)
            // ==========================================
            "lvenk_pattern_match" => {
                let text = s!(args.remove(0), name);
                let pattern = s!(args.remove(0), name);
                let rust_regex = translate_lua_pattern(&pattern);
                match Regex::new(&rust_regex) {
                    Ok(re) => Ok(Value::Bool(re.is_match(&text))),
                    Err(e) => Err(Signal::Runtime(format!("Lua Pattern Error: {}", e))),
                }
            }
            "lvenk_pattern_find" => {
                let text = s!(args.remove(0), name);
                let pattern = s!(args.remove(0), name);
                let rust_regex = translate_lua_pattern(&pattern);
                match Regex::new(&rust_regex) {
                    Ok(re) => {
                        if let Some(m) = re.find(&text) {
                            Ok(Value::str(m.as_str().to_string()))
                        } else {
                            Ok(Value::Null)
                        }
                    }
                    Err(e) => Err(Signal::Runtime(format!("Lua Pattern Error: {}", e))),
                }
            }
            "lvenk_pattern_replace" => {
                let text = s!(args.remove(0), name);
                let pattern = s!(args.remove(0), name);
                let replacement = s!(args.remove(0), name);
                let rust_regex = translate_lua_pattern(&pattern);
                match Regex::new(&rust_regex) {
                    Ok(re) => Ok(Value::str(re.replace_all(&text, replacement.as_str()).to_string())),
                    Err(e) => Err(Signal::Runtime(format!("Lua Pattern Error: {}", e))),
                }
            }

            // ==========================================
            // TABLE (Массивы как в Lua)
            // ==========================================
            "lvenk_table_insert" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let a = arr!(args.remove(0), name);
                let item = args.remove(0);
                a.borrow_mut().push(item);
                Ok(Value::Null)
            }
            "lvenk_table_remove" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let a = arr!(args.remove(0), name);
                let i = if !args.is_empty() { int!(args.remove(0), name) as usize } else { a.borrow().len().saturating_sub(1) };
                
                let mut b = a.borrow_mut();
                if i < b.len() {
                    Ok(Value::opt_some(b.remove(i)))
                } else {
                    Ok(Value::opt_none())
                }
            }
            "lvenk_table_concat" => {
                let a = arr!(args.remove(0), name);
                let sep = if !args.is_empty() { s!(args.remove(0), name) } else { Rc::new("".to_string()) };
                let parts: Vec<String> = a.borrow().iter().map(|v| v.to_display()).collect();
                Ok(Value::str(parts.join(&*sep)))
            }

            // ==========================================
            // MATH (Математика)
            // ==========================================
            "lvenk_math_fmod" => {
                let x = num!(args.remove(0), name);
                let y = num!(args.remove(0), name);
                if y == 0.0 {
                    return Ok(Value::Float(f64::NAN));
                }
                Ok(Value::Float(x % y))
            }
            "lvenk_math_log" => {
                let x = num!(args.remove(0), name);
                let base = if !args.is_empty() { num!(args.remove(0), name) } else { std::f64::consts::E };
                Ok(Value::Float(x.log(base)))
            }

            other => Err(Signal::Runtime(format!("lvenk '{}' не реализован", other))),
        }
    }
}

// ==========================================
// ПЕРЕВОДЧИК LUA PATTERNS В REGEX
// ==========================================
fn translate_lua_pattern(pattern: &str) -> String {
    let mut result = String::new();
    let mut chars = pattern.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(&next) = chars.peek() {
                match next {
                    'a' => { result.push_str("[a-zA-Z]"); chars.next(); }
                    'd' => { result.push_str("[0-9]"); chars.next(); }
                    's' => { result.push_str("[ \\t\\r\\n]"); chars.next(); }
                    'w' => { result.push_str("[a-zA-Z0-9]"); chars.next(); }
                    'l' => { result.push_str("[a-z]"); chars.next(); }
                    'u' => { result.push_str("[A-Z]"); chars.next(); }
                    '%' => { result.push('%'); chars.next(); }
                    '.' => { result.push_str("\\."); chars.next(); }
                    '-' => { result.push_str("*?"); chars.next(); } 
                    _ => { result.push(next); chars.next(); }
                }
            } else {
                result.push('%');
            }
        } else if c == '-' {
            result.push('-');
        } else {
            result.push(c);
        }
    }
    result
}