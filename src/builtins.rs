use crate::ast::Expr;
use crate::interp::{Interp, Signal, R};
use crate::value::*;
use std::cell::RefCell;
use std::rc::Rc;

impl Interp {
    pub(crate) fn call_builtin(&mut self, name: &str, mut args: Vec<Value>, arg_exprs: &[Expr]) -> R {
        // ==========================================
        // МАРШРУТИЗАЦИЯ МОДУЛЕЙ
        // ==========================================
        if name.starts_with("pvenk_") { 
            return self.call_pvenk(name, args); 
        }
        if name.starts_with("jvenk_") { 
            return self.call_jvenk(name, args); 
        }
        if name.starts_with("gdvenk_") { 
            return self.call_gdvenk(name, args); 
        }
        if name.starts_with("lvenk_") { 
            return self.call_lvenk(name, args, arg_exprs); 
        }
        if name.starts_with("cppvenk_") { 
            return self.call_cppvenk(name, args); 
        }
        if name.starts_with("vek3d_") { 
            return self.call_vek3d(name, args); 
        }
        // ==========================================

        // Макросы для удобного извлечения аргументов
        macro_rules! int {
            ($v:expr, $pos:expr, $name:expr) => {
                match $v {
                    Value::Int(n) => n,
                    other => return Err(Signal::Runtime(format!("{}: аргумент {} — ожидался int, получен {}", $name, $pos, other.type_name()))),
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
        macro_rules! s {
            ($v:expr, $name:expr) => {
                match $v {
                    Value::Str(s) => s,
                    other => return Err(Signal::Runtime(format!("{}: ожидалась строка, получен {}", $name, other.type_name()))),
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
            // СБОРЩИК В EXE
            // ==========================================
            "build" => {
                let path = match args.remove(0) {
                    Value::Str(s) => s,
                    other => return Err(Signal::Runtime(format!("build: ожидалось имя файла, получен {}", other.type_name()))),
                };
                
                println!("\x1b[33m[СБОРКА] Чтение и объединение зависимостей...\x1b[0m");
                let mut seen = std::collections::HashSet::new();
                
                let bundled_code = match bundle_file(&path, &mut seen) {
                    Ok(c) => c,
                    Err(e) => return Ok(Value::res_err(Value::str(e))),
                };

                let current_exe = match std::env::current_exe() {
                    Ok(p) => p,
                    Err(e) => return Ok(Value::res_err(Value::str(e.to_string()))),
                };
                
                let output_path = std::path::Path::new(&*path).with_extension("exe");
                
                if let Err(e) = std::fs::copy(&current_exe, &output_path) {
                    return Ok(Value::res_err(Value::str(format!("Ошибка копирования (возможно, файл занят): {}", e))));
                }

                let mut out_file = match std::fs::OpenOptions::new().write(true).append(true).open(&output_path) {
                    Ok(f) => f,
                    Err(e) => return Ok(Value::res_err(Value::str(e.to_string()))),
                };

                use std::io::Write;
                if let Err(e) = out_file.write_all(crate::MAGIC_HEADER) {
                     return Ok(Value::res_err(Value::str(format!("Ошибка записи заголовка: {}", e))));
                }
                if let Err(e) = out_file.write_all(bundled_code.as_bytes()) {
                     return Ok(Value::res_err(Value::str(format!("Ошибка записи кода: {}", e))));
                }
                if let Err(e) = out_file.flush() {
                     return Ok(Value::res_err(Value::str(format!("Ошибка сохранения файла: {}", e))));
                }

                println!("\x1b[32m✔ Готово! Игра собрана: {}\x1b[0m", output_path.display());
                Ok(Value::str(format!("Собрано: {}", output_path.display())))
            }

            // ==========================================
            // ЯДРО: МАТЕМАТИКА
            // ==========================================
            "abs" => {
                let v = args.remove(0);
                match v {
                    Value::Int(n) => Ok(Value::Int(n.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(Signal::Runtime("abs: ожидалось число".into())),
                }
            }
            "sign" => {
                let v = args.remove(0);
                let res = match v {
                    Value::Int(n) => n.signum(),
                    Value::Float(f) => {
                        if f > 0.0 { 1 } 
                        else if f < 0.0 { -1 } 
                        else { 0 }
                    },
                    _ => 0,
                };
                Ok(Value::Int(res))
            }
            "min" | "max" => {
                let cmp_less = name == "min";
                if args.is_empty() { 
                    return Err(Signal::Runtime(format!("{}: нужен аргумент", name))); 
                }
                let mut acc = num!(args.remove(0), name);
                for v in args {
                    let x = num!(v, name);
                    if cmp_less {
                        if x < acc { acc = x; }
                    } else {
                        if x > acc { acc = x; }
                    }
                }
                Ok(Value::Float(acc))
            }
            "clamp" => {
                let x = num!(args.remove(0), name);
                let lo = num!(args.remove(0), name);
                let hi = num!(args.remove(0), name);
                Ok(Value::Float(x.max(lo).min(hi)))
            }
            "sum" => {
                let a = arr!(args.remove(0), name);
                let mut sum = 0.0;
                {
                    let borrowed = a.borrow();
                    for v in borrowed.iter() {
                        sum += num!(v.clone(), name);
                    }
                }
                Ok(Value::Float(sum))
            }
            "avg" => {
                let a = arr!(args.remove(0), name);
                let mut sum = 0.0;
                let len;
                {
                    let borrowed = a.borrow();
                    len = borrowed.len();
                    if len == 0 { 
                        return Ok(Value::Float(0.0)); 
                    }
                    for v in borrowed.iter() { 
                        sum += num!(v.clone(), name); 
                    }
                }
                Ok(Value::Float(sum / len as f64))
            }
            "floor" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.floor()))
            }
            "ceil" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.ceil()))
            }
            "round" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.round()))
            }
            "sqrt" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.sqrt()))
            }
            "cbrt" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.cbrt()))
            }
            "pow" => {
                let a = num!(args.remove(0), name);
                let b = num!(args.remove(0), name);
                Ok(Value::Float(a.powf(b)))
            }
            "exp" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.exp()))
            }
            "log" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.ln()))
            }
            "log2" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.log2()))
            }
            "log10" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.log10()))
            }
            "sin" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.sin()))
            }
            "cos" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.cos()))
            }
            "tan" => {
                let x = num!(args.remove(0), name);
                Ok(Value::Float(x.tan()))
            }

            // ==========================================
            // ЯДРО: RANDOM
            // ==========================================
            "random" => {
                Ok(Value::Float(rand_unit()))
            }
            "random_int" => {
                let lo = int!(args.remove(0), 1, name);
                let hi = int!(args.remove(0), 2, name);
                if hi < lo { 
                    return Ok(Value::Int(lo)); 
                }
                let span = (hi - lo + 1) as u64;
                Ok(Value::Int(lo + (rand_u64() % span) as i64))
            }
            "random_float" => {
                let lo = num!(args.remove(0), name);
                let hi = num!(args.remove(0), name);
                Ok(Value::Float(lo + rand_unit() * (hi - lo)))
            }
            "random_bool" => {
                Ok(Value::Bool(rand_unit() < 0.5))
            }
            "seed" => { 
                let _ = int!(args.remove(0), 1, name); 
                Ok(Value::Null) 
            }

            // ==========================================
            // ЯДРО: СТРОКИ
            // ==========================================
            "len" => {
                let v = args.remove(0);
                match v {
                    Value::Str(s) => Ok(Value::Int(s.len() as i64)),
                    Value::Array(a) => { 
                        let b = a.borrow(); 
                        Ok(Value::Int(b.len() as i64)) 
                    }
                    Value::Map(m) => { 
                        let b = m.borrow(); 
                        Ok(Value::Int(b.len() as i64)) 
                    }
                    Value::Set(s) => { 
                        let b = s.borrow(); 
                        Ok(Value::Int(b.len() as i64)) 
                    }
                    Value::Record(r) | Value::Struct(_, r) => { 
                        let b = r.borrow(); 
                        Ok(Value::Int(b.len() as i64)) 
                    }
                    Value::Tuple(t) => Ok(Value::Int(t.len() as i64)),
                    other => Err(Signal::Runtime(format!("len: {}", other.type_name()))),
                }
            }
            "char_count" => {
                let text = s!(args.remove(0), name);
                Ok(Value::Int(text.chars().count() as i64))
            }
            "upper" => {
                let text = s!(args.remove(0), name);
                Ok(Value::str(text.to_uppercase()))
            }
            "lower" => {
                let text = s!(args.remove(0), name);
                Ok(Value::str(text.to_lowercase()))
            }
            "capitalize" => {
                let text = s!(args.remove(0), name);
                let mut chars = text.chars();
                match chars.next() {
                    Some(first) => {
                        let result = first.to_uppercase().collect::<String>() + chars.as_str();
                        Ok(Value::str(result))
                    }
                    None => Ok(Value::str("")),
                }
            }
            "title" => {
                let text = s!(args.remove(0), name);
                let result = text.split_whitespace().map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                }).collect::<Vec<_>>().join(" ");
                Ok(Value::str(result))
            }
            "trim" => {
                let text = s!(args.remove(0), name);
                Ok(Value::str(text.trim()))
            }
            "trim_start" => {
                let text = s!(args.remove(0), name);
                Ok(Value::str(text.trim_start()))
            }
            "trim_end" => {
                let text = s!(args.remove(0), name);
                Ok(Value::str(text.trim_end()))
            }
            "split" => {
                let text = s!(args.remove(0), name);
                let sep = s!(args.remove(0), name);
                let parts: Vec<Value>;
                
                if sep.is_empty() {
                    parts = text.chars().map(|c| Value::str(c.to_string())).collect();
                } else {
                    parts = text.split(sep.as_str()).map(|x| Value::str(x.to_string())).collect();
                }
                Ok(Value::arr(parts))
            }
            "join" => {
                let a = arr!(args.remove(0), name);
                let sep = s!(args.remove(0), name);
                let parts: Vec<String>;
                {
                    let borrowed = a.borrow();
                    parts = borrowed.iter().map(|v| v.to_display()).collect();
                }
                Ok(Value::str(parts.join(&sep)))
            }
            "replace" => {
                let text = s!(args.remove(0), name);
                let from = s!(args.remove(0), name);
                let to = s!(args.remove(0), name);
                Ok(Value::str(text.replacen(from.as_str(), to.as_str(), 1)))
            }
            "replace_all" => {
                let text = s!(args.remove(0), name);
                let from = s!(args.remove(0), name);
                let to = s!(args.remove(0), name);
                Ok(Value::str(text.replace(from.as_str(), to.as_str())))
            }
            "starts_with" => {
                let text = s!(args.remove(0), name);
                let prefix = s!(args.remove(0), name);
                Ok(Value::Bool(text.starts_with(prefix.as_str())))
            }
            "ends_with" => {
                let text = s!(args.remove(0), name);
                let suffix = s!(args.remove(0), name);
                Ok(Value::Bool(text.ends_with(suffix.as_str())))
            }
            "includes" => {
                let first = args.remove(0);
                let second = args.remove(0);
                match (&first, &second) {
                    (Value::Str(a), Value::Str(b)) => {
                        Ok(Value::Bool(a.contains(b.as_str())))
                    }
                    (Value::Array(a), b) => {
                        let has;
                        { 
                            let borrowed = a.borrow(); 
                            has = borrowed.iter().any(|v| self.values_eq(v, b)); 
                        }
                        Ok(Value::Bool(has))
                    }
                    _ => Ok(Value::Bool(false)),
                }
            }
            "index_of" => {
                let first = args.remove(0);
                let second = args.remove(0);
                match (&first, &second) {
                    (Value::Str(a), Value::Str(b)) => {
                        match a.find(b.as_str()) { 
                            Some(pos) => Ok(Value::Int(a[..pos].chars().count() as i64)), 
                            None => Ok(Value::Int(-1)) 
                        }
                    }
                    (Value::Array(a), _) => {
                        let mut idx = -1;
                        {
                            let borrowed = a.borrow();
                            for (i, v) in borrowed.iter().enumerate() {
                                if self.values_eq(v, &second) { 
                                    idx = i as i64; 
                                    break; 
                                }
                            }
                        }
                        Ok(Value::Int(idx))
                    }
                    _ => Ok(Value::Int(-1)),
                }
            }
            "slice" => {
                let v = args.remove(0);
                let start = int!(args.remove(0), 2, name);
                match v {
                    Value::Str(st) => {
                        let end;
                        if !args.is_empty() { 
                            end = int!(args.remove(0), 3, name); 
                        } else { 
                            end = st.chars().count() as i64; 
                        }
                        let chars: Vec<char> = st.chars().collect();
                        let s_idx = normalize_idx(start, chars.len());
                        let e_idx = normalize_idx(end, chars.len()).max(s_idx);
                        Ok(Value::str(chars[s_idx..e_idx.min(chars.len())].iter().collect::<String>()))
                    }
                    Value::Array(a) => {
                        let end;
                        if !args.is_empty() { 
                            end = int!(args.remove(0), 3, name); 
                        } else { 
                            let b = a.borrow(); 
                            end = b.len() as i64; 
                        }
                        let res;
                        {
                            let arr = a.borrow();
                            let s_idx = normalize_idx(start, arr.len());
                            let e_idx = normalize_idx(end, arr.len()).max(s_idx);
                            res = arr[s_idx..e_idx.min(arr.len())].to_vec();
                        }
                        Ok(Value::arr(res))
                    }
                    other => Err(Signal::Runtime(format!("slice: {}", other.type_name()))),
                }
            }
            "repeat" => {
                let st = s!(args.remove(0), name);
                let n = int!(args.remove(0), 2, name);
                Ok(Value::str(st.as_str().repeat(n.max(0) as usize)))
            }
            "pad_start" | "pad_end" => {
                let st = s!(args.remove(0), name);
                let width = int!(args.remove(0), 2, name) as usize;
                let ch;
                if !args.is_empty() { 
                    match args.remove(0) { 
                        Value::Str(s) => ch = s.chars().next().unwrap_or(' '), 
                        _ => ch = ' ' 
                    } 
                } else { 
                    ch = ' '; 
                }
                let cur = st.chars().count();
                if cur >= width { 
                    return Ok(Value::str(st.as_str())); 
                }
                let pad: String = std::iter::repeat(ch).take(width - cur).collect();
                if name == "pad_start" {
                    Ok(Value::str(pad + &st))
                } else {
                    Ok(Value::str(st.as_str().to_string() + &pad))
                }
            }
            "chars" => {
                let text = s!(args.remove(0), name);
                Ok(Value::arr(text.chars().map(|c| Value::str(c.to_string())).collect()))
            }
            "bytes" => {
                let text = s!(args.remove(0), name);
                Ok(Value::arr(text.bytes().map(|b| Value::Int(b as i64)).collect()))
            }

            // ==========================================
            // ЯДРО: МАССИВЫ
            // ==========================================
            "push" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let a = arr!(args.remove(0), name);
                let item = args.remove(0);
                let len;
                { 
                    let mut b = a.borrow_mut(); 
                    b.push(item); 
                    len = b.len() as i64; 
                }
                Ok(Value::Int(len))
            }
            "pop" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let a = arr!(args.remove(0), name);
                let popped;
                { 
                    let mut b = a.borrow_mut(); 
                    popped = b.pop(); 
                }
                match popped { 
                    Some(v) => Ok(Value::opt_some(v)), 
                    None => Ok(Value::opt_none()) 
                }
            }
            "shift" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let a = arr!(args.remove(0), name);
                let shifted;
                { 
                    let mut b = a.borrow_mut(); 
                    if b.is_empty() { 
                        shifted = None; 
                    } else { 
                        shifted = Some(b.remove(0)); 
                    } 
                }
                match shifted { 
                    Some(v) => Ok(Value::opt_some(v)), 
                    None => Ok(Value::opt_none()) 
                }
            }
            "unshift" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let a = arr!(args.remove(0), name);
                let item = args.remove(0);
                let len;
                { 
                    let mut b = a.borrow_mut(); 
                    b.insert(0, item); 
                    len = b.len() as i64; 
                }
                Ok(Value::Int(len))
            }
            "insert" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let a = arr!(args.remove(0), name);
                let i = int!(args.remove(0), 2, name) as usize;
                let v = args.remove(0);
                {
                    let mut b = a.borrow_mut();
                    if i > b.len() { 
                        b.push(v); 
                    } else { 
                        b.insert(i, v); 
                    }
                }
                Ok(Value::Null)
            }
            "remove_at" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let a = arr!(args.remove(0), name);
                let i = int!(args.remove(0), 2, name) as usize;
                let removed;
                {
                    let mut b = a.borrow_mut();
                    if i >= b.len() { 
                        removed = None; 
                    } else { 
                        removed = Some(b.remove(i)); 
                    }
                }
                match removed { 
                    Some(v) => Ok(Value::opt_some(v)), 
                    None => Ok(Value::opt_none()) 
                }
            }
            "first" => {
                let v = args.remove(0);
                match v {
                    Value::Array(a) => {
                        let f;
                        { 
                            let b = a.borrow(); 
                            f = b.first().cloned(); 
                        }
                        match f { 
                            Some(val) => Ok(Value::opt_some(val)), 
                            None => Ok(Value::opt_none()) 
                        }
                    }
                    Value::Str(s) => {
                        match s.chars().next() { 
                            Some(c) => Ok(Value::opt_some(Value::Char(c))), 
                            None => Ok(Value::opt_none()) 
                        }
                    }
                    _ => Ok(Value::opt_none()),
                }
            }
            "last" => {
                let v = args.remove(0);
                match v {
                    Value::Array(a) => {
                        let f;
                        { 
                            let b = a.borrow(); 
                            f = b.last().cloned(); 
                        }
                        match f { 
                            Some(val) => Ok(Value::opt_some(val)), 
                            None => Ok(Value::opt_none()) 
                        }
                    }
                    Value::Str(s) => {
                        match s.chars().last() { 
                            Some(c) => Ok(Value::opt_some(Value::Char(c))), 
                            None => Ok(Value::opt_none()) 
                        }
                    }
                    _ => Ok(Value::opt_none()),
                }
            }
            "fill" => {
                let n = int!(args.remove(0), 1, name).max(0) as usize;
                let v = args.remove(0);
                Ok(Value::arr(vec![v; n]))
            }
            "range" => {
                let start = int!(args.remove(0), 1, name);
                let end;
                if !args.is_empty() { 
                    end = int!(args.remove(0), 2, name); 
                } else { 
                    end = start; 
                }
                let step;
                if !args.is_empty() { 
                    step = int!(args.remove(0), 3, name); 
                } else { 
                    step = 1; 
                }
                
                if step == 0 { 
                    return Err(Signal::Runtime("range: step=0".into())); 
                }
                
                let mut v = Vec::new();
                let mut i = start;
                while (step > 0 && i < end) || (step < 0 && i > end) { 
                    v.push(Value::Int(i)); 
                    i += step; 
                }
                Ok(Value::arr(v))
            }
            "concat" => {
                let mut all = Vec::new();
                for v in args { 
                    if let Value::Array(a) = v { 
                        let borrowed = a.borrow();
                        all.extend(borrowed.iter().cloned()); 
                    } 
                }
                Ok(Value::arr(all))
            }
            "reversed" => {
                let v = args.remove(0);
                match v {
                    Value::Array(a) => { 
                        let mut x = { let b = a.borrow(); b.clone() }; 
                        x.reverse(); 
                        Ok(Value::arr(x)) 
                    }
                    Value::Str(s) => Ok(Value::str(s.chars().rev().collect::<String>())),
                    _ => Err(Signal::Runtime("reversed".into())),
                }
            }
            "unique" => {
                let a = arr!(args.remove(0), name);
                let mut out: Vec<Value> = Vec::new();
                {
                    let borrowed = a.borrow();
                    for v in borrowed.iter() {
                        if !out.iter().any(|x| self.values_eq(x, v)) { 
                            out.push(v.clone()); 
                        }
                    }
                }
                Ok(Value::arr(out))
            }
            "chunk" => {
                let a = arr!(args.remove(0), name);
                let n = int!(args.remove(0), 2, name).max(1) as usize;
                let items = { let b = a.borrow(); b.clone() };
                let mut out = Vec::new();
                let mut cur = Vec::new();
                for v in items { 
                    cur.push(v); 
                    if cur.len() == n { 
                        out.push(Value::arr(std::mem::take(&mut cur))); 
                    } 
                }
                if !cur.is_empty() { 
                    out.push(Value::arr(cur)); 
                }
                Ok(Value::arr(out))
            }
            "flatten" => {
                let a = arr!(args.remove(0), name);
                let mut out = Vec::new();
                {
                    let borrowed = a.borrow();
                    for v in borrowed.iter() {
                        match v { 
                            Value::Array(inner) => {
                                let inner_borrowed = inner.borrow();
                                out.extend(inner_borrowed.iter().cloned()); 
                            }
                            x => out.push(x.clone()) 
                        }
                    }
                }
                Ok(Value::arr(out))
            }
            "zip" => {
                let a = arr!(args.remove(0), name);
                let b = arr!(args.remove(0), name);
                let aa = { let br = a.borrow(); br.clone() };
                let bb = { let br = b.borrow(); br.clone() };
                let out: Vec<Value> = aa.iter().zip(bb.iter()).map(|(x, y)| Value::Tuple(Rc::new(vec![x.clone(), y.clone()]))).collect();
                Ok(Value::arr(out))
            }
            "with" => {
                let a = arr!(args.remove(0), name);
                let item = args.remove(0);
                let mut v = { let b = a.borrow(); b.clone() };
                v.push(item);
                Ok(Value::arr(v))
            }
            "without_last" => {
                let a = arr!(args.remove(0), name);
                let mut v = { let b = a.borrow(); b.clone() };
                v.pop();
                Ok(Value::arr(v))
            }
            "enumerate" => {
                let a = arr!(args.remove(0), name);
                let res = {
                    let borrowed = a.borrow();
                    borrowed.iter().enumerate().map(|(i, v)| Value::Tuple(Rc::new(vec![Value::Int(i as i64), v.clone()]))).collect()
                };
                Ok(Value::arr(res))
            }

            // ==========================================
            // ЯДРО: MAP / SET
            // ==========================================
            "keys" => {
                let m = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("keys".into())) 
                };
                let keys = {
                    let borrowed = m.borrow();
                    borrowed.iter().map(|(k, _)| k.to_value()).collect()
                };
                Ok(Value::arr(keys))
            }
            "values" => {
                let m = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("values".into())) 
                };
                let vals = {
                    let borrowed = m.borrow();
                    borrowed.iter().map(|(_, v)| v.clone()).collect()
                };
                Ok(Value::arr(vals))
            }
            "entries" | "to_entries" => {
                let m = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("entries".into())) 
                };
                let ents = {
                    let borrowed = m.borrow();
                    borrowed.iter().map(|(k, v)| Value::Tuple(Rc::new(vec![k.to_value(), v.clone()]))).collect()
                };
                Ok(Value::arr(ents))
            }
            "get" => {
                let m = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("get".into())) 
                };
                let k = args.remove(0).to_key().ok_or_else(|| Signal::Runtime("get: key".into()))?;
                let res = {
                    let borrowed = m.borrow();
                    borrowed.iter().find(|(kk, _)| *kk == k).map(|(_, v)| v.clone())
                };
                match res {
                    Some(v) => Ok(Value::opt_some(v)), 
                    None => Ok(Value::opt_none())
                }
            }
            "get_or" => {
                let m = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("get_or".into())) 
                };
                let k = args.remove(0).to_key().ok_or_else(|| Signal::Runtime("get_or: key".into()))?;
                let default = args.remove(0);
                let res = {
                    let borrowed = m.borrow();
                    borrowed.iter().find(|(kk, _)| *kk == k).map(|(_, v)| v.clone())
                };
                Ok(res.unwrap_or(default))
            }
            "set" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let m = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("set".into())) 
                };
                let k = args.remove(0).to_key().ok_or_else(|| Signal::Runtime("set: key".into()))?;
                let v = args.remove(0);
                {
                    let mut mm = m.borrow_mut();
                    if let Some(slot) = mm.iter_mut().find(|(kk, _)| *kk == k) { 
                        slot.1 = v; 
                    } else { 
                        mm.push((k, v)); 
                    }
                }
                Ok(Value::Null)
            }
            "has" => {
                let m = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("has".into())) 
                };
                let k = args.remove(0).to_key().ok_or_else(|| Signal::Runtime("has: key".into()))?;
                let has = { let mm = m.borrow(); mm.iter().any(|(kk, _)| *kk == k) };
                Ok(Value::Bool(has))
            }
            "delete" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let m = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("delete".into())) 
                };
                let k = args.remove(0).to_key().ok_or_else(|| Signal::Runtime("delete: key".into()))?;
                let removed = {
                    let mut mm = m.borrow_mut();
                    if let Some(pos) = mm.iter().position(|(kk, _)| *kk == k) {
                        Some(mm.remove(pos).1)
                    } else { 
                        None 
                    }
                };
                match removed { 
                    Some(v) => Ok(Value::opt_some(v)), 
                    None => Ok(Value::opt_none()) 
                }
            }
            "merge" => {
                let a = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("merge".into())) 
                };
                let b = match args.remove(0) { 
                    Value::Map(m) => m, 
                    _ => return Err(Signal::Runtime("merge".into())) 
                };
                let mut out = { let br = a.borrow(); br.clone() };
                {
                    let borrowed_b = b.borrow();
                    for (k, v) in borrowed_b.iter() {
                        if let Some(slot) = out.iter_mut().find(|(kk, _)| kk == k) { 
                            slot.1 = v.clone(); 
                        } else { 
                            out.push((k.clone(), v.clone())); 
                        }
                    }
                }
                Ok(Value::Map(Rc::new(RefCell::new(out))))
            }
            "clone" => {
                let v = args.remove(0);
                match v {
                    Value::Array(a) => { 
                        let b = a.borrow(); 
                        Ok(Value::arr(b.clone())) 
                    }
                    Value::Map(m) => { 
                        let b = m.borrow(); 
                        Ok(Value::Map(Rc::new(RefCell::new(b.clone())))) 
                    }
                    Value::Record(r) => { 
                        let b = r.borrow(); 
                        Ok(Value::Record(Rc::new(RefCell::new(b.clone())))) 
                    }
                    Value::Set(s) => { 
                        let b = s.borrow(); 
                        Ok(Value::Set(Rc::new(RefCell::new(b.clone())))) 
                    }
                    other => Ok(other),
                }
            }
            "from_entries" | "to_map" => {
                let a = arr!(args.remove(0), name);
                let mut m: Vec<(MapKey, Value)> = Vec::new();
                {
                    let borrowed = a.borrow();
                    for v in borrowed.iter() {
                        if let Value::Tuple(t) = v {
                            if t.len() == 2 {
                                if let Some(k) = t[0].to_key() { 
                                    m.push((k, t[1].clone())); 
                                }
                            }
                        }
                    }
                }
                Ok(Value::Map(Rc::new(RefCell::new(m))))
            }
            "set_create" => {
                let items = if !args.is_empty() { 
                    match args.remove(0) { 
                        Value::Array(a) => { let b = a.borrow(); b.clone() }, 
                        _ => Vec::new() 
                    } 
                } else { 
                    Vec::new() 
                };
                let mut s: Vec<MapKey> = Vec::new();
                for v in items { 
                    if let Some(k) = v.to_key() { 
                        if !s.contains(&k) { 
                            s.push(k); 
                        } 
                    } 
                }
                Ok(Value::Set(Rc::new(RefCell::new(s))))
            }
            "set_add" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let s = match args.remove(0) { 
                    Value::Set(s) => s, 
                    _ => return Err(Signal::Runtime("set_add".into())) 
                };
                let k = args.remove(0).to_key().ok_or_else(|| Signal::Runtime("set_add: key".into()))?;
                let added = {
                    let mut ss = s.borrow_mut();
                    let added = !ss.contains(&k);
                    if added { 
                        ss.push(k); 
                    }
                    added
                };
                Ok(Value::Bool(added))
            }
            "set_remove" => {
                self.check_mut_arg(0, arg_exprs, name)?;
                let s = match args.remove(0) { 
                    Value::Set(s) => s, 
                    _ => return Err(Signal::Runtime("set_remove".into())) 
                };
                let k = args.remove(0).to_key().ok_or_else(|| Signal::Runtime("set_remove: key".into()))?;
                let removed = {
                    let mut ss = s.borrow_mut();
                    if let Some(pos) = ss.iter().position(|x| *x == k) { 
                        ss.remove(pos); 
                        true 
                    } else { 
                        false 
                    }
                };
                Ok(Value::Bool(removed))
            }
            "set_has" => {
                let s = match args.remove(0) { 
                    Value::Set(s) => s, 
                    _ => return Err(Signal::Runtime("set_has".into())) 
                };
                let k = args.remove(0).to_key().ok_or_else(|| Signal::Runtime("set_has: key".into()))?;
                let has = { let ss = s.borrow(); ss.contains(&k) };
                Ok(Value::Bool(has))
            }
            "set_union" | "set_intersect" | "set_difference" => {
                let a = match args.remove(0) { 
                    Value::Set(s) => s, 
                    _ => return Err(Signal::Runtime(name.into())) 
                };
                let b = match args.remove(0) { 
                    Value::Set(s) => s, 
                    _ => return Err(Signal::Runtime(name.into())) 
                };
                let aa = { let br = a.borrow(); br.clone() };
                let bb = { let br = b.borrow(); br.clone() };
                let out = match name {
                    "set_union" => { 
                        let mut o = aa.clone(); 
                        for k in &bb { 
                            if !o.contains(k) { 
                                o.push(k.clone()); 
                            } 
                        } 
                        o 
                    }
                    "set_intersect" => aa.iter().filter(|k| bb.contains(k)).cloned().collect(),
                    "set_difference" => aa.iter().filter(|k| !bb.contains(k)).cloned().collect(),
                    _ => unreachable!(),
                };
                Ok(Value::Set(Rc::new(RefCell::new(out))))
            }
            "set_to_array" => {
                let s = match args.remove(0) { 
                    Value::Set(s) => s, 
                    _ => return Err(Signal::Runtime("set_to_array".into())) 
                };
                let arr = {
                    let borrowed = s.borrow();
                    borrowed.iter().map(|k| k.to_value()).collect()
                };
                Ok(Value::arr(arr))
            }
            "to_set" => {
                let a = arr!(args.remove(0), name);
                let mut s: Vec<MapKey> = Vec::new();
                {
                    let borrowed = a.borrow();
                    for v in borrowed.iter() { 
                        if let Some(k) = v.to_key() { 
                            if !s.contains(&k) { 
                                s.push(k); 
                            } 
                        } 
                    }
                }
                Ok(Value::Set(Rc::new(RefCell::new(s))))
            }

            // ==========================================
            // ЯДРО: КОНВЕРТАЦИЯ
            // ==========================================
            "to_int" => {
                let v = args.remove(0);
                let res = match v {
                    Value::Int(n) => n,
                    Value::Float(f) => f as i64,
                    Value::Bool(b) => if b { 1 } else { 0 },
                    Value::Str(s) => s.parse().unwrap_or(0),
                    Value::Char(c) => c as i64,
                    _ => 0,
                };
                Ok(Value::Int(res))
            }
            "to_float" => {
                let v = args.remove(0);
                let res = match v {
                    Value::Int(n) => n as f64,
                    Value::Float(f) => f,
                    Value::Str(s) => s.parse().unwrap_or(0.0),
                    _ => 0.0,
                };
                Ok(Value::Float(res))
            }
            "to_string" => {
                let v = args.remove(0);
                Ok(Value::str(v.to_display()))
            }
            "to_bool" => {
                let v = args.remove(0);
                Ok(Value::Bool(v.is_truthy()))
            }
            "to_array" => {
                let v = args.remove(0);
                match v {
                    Value::Array(a) => Ok(Value::Array(a)),
                    Value::Set(s) => {
                        let res = { let b = s.borrow(); b.iter().map(|k| k.to_value()).collect() };
                        Ok(Value::arr(res))
                    }
                    Value::Map(m) => {
                        let res = { let b = m.borrow(); b.iter().map(|(k, v)| Value::Tuple(Rc::new(vec![k.to_value(), v.clone()]))).collect() };
                        Ok(Value::arr(res))
                    }
                    Value::Str(st) => {
                        Ok(Value::arr(st.chars().map(|c| Value::str(c.to_string())).collect()))
                    }
                    other => Ok(Value::arr(vec![other])),
                }
            }
            "parse_int" => {
                let st = s!(args.remove(0), name);
                match st.parse::<i64>() { 
                    Ok(n) => Ok(Value::res_ok(Value::Int(n))), 
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))) 
                }
            }
            "parse_float" => {
                let st = s!(args.remove(0), name);
                match st.parse::<f64>() { 
                    Ok(n) => Ok(Value::res_ok(Value::Float(n))), 
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))) 
                }
            }
            "parse_bool" => {
                let st = s!(args.remove(0), name);
                match st.to_lowercase().as_str() {
                    "true" | "1" | "yes" => Ok(Value::res_ok(Value::Bool(true))),
                    "false" | "0" | "no" => Ok(Value::res_ok(Value::Bool(false))),
                    _ => Ok(Value::res_err(Value::str(format!("не bool: {}", st)))),
                }
            }
            "type_of" => {
                let v = args.remove(0);
                Ok(Value::str(v.type_name()))
            }
            "is_null" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Null)))
            }
            "is_number" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Int(_) | Value::Float(_))))
            }
            "is_int" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Int(_))))
            }
            "is_float" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Float(_))))
            }
            "is_string" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Str(_))))
            }
            "is_bool" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Bool(_))))
            }
            "is_array" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Array(_))))
            }
            "is_map" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Map(_))))
            }
            "is_function" => {
                let v = args.remove(0);
                Ok(Value::Bool(matches!(v, Value::Fn(_) | Value::Builtin(_))))
            }

            // ==========================================
            // ЯДРО: ФУНКЦИОНАЛЬНОЕ
            // ==========================================
            "map" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                let mut out = Vec::with_capacity(items.len());
                for v in items { 
                    out.push(self.call_value(f.clone(), vec![v], &[])?); 
                }
                Ok(Value::arr(out))
            }
            "filter" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                let mut out = Vec::new();
                for v in items {
                    if self.call_value(f.clone(), vec![v.clone()], &[])?.is_truthy() { 
                        out.push(v); 
                    }
                }
                Ok(Value::arr(out))
            }
            "reduce" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                let mut acc;
                if !args.is_empty() { 
                    acc = args.remove(0); 
                } else if !items.is_empty() { 
                    acc = items[0].clone(); 
                } else { 
                    acc = Value::Null; 
                }
                let start = if args.is_empty() && !items.is_empty() { 1 } else { 0 };
                for v in items.into_iter().skip(start) {
                    acc = self.call_value(f.clone(), vec![acc, v], &[])?;
                }
                Ok(acc)
            }
            "for_each" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                for v in items { 
                    self.call_value(f.clone(), vec![v], &[])?; 
                }
                Ok(Value::Null)
            }
            "find" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                for v in items {
                    if self.call_value(f.clone(), vec![v.clone()], &[])?.is_truthy() { 
                        return Ok(Value::opt_some(v)); 
                    }
                }
                Ok(Value::opt_none())
            }
            "find_index" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                for (i, v) in items.into_iter().enumerate() {
                    if self.call_value(f.clone(), vec![v], &[])?.is_truthy() { 
                        return Ok(Value::Int(i as i64)); 
                    }
                }
                Ok(Value::Int(-1))
            }
            "some" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                for v in items { 
                    if self.call_value(f.clone(), vec![v], &[])?.is_truthy() { 
                        return Ok(Value::Bool(true)); 
                    } 
                }
                Ok(Value::Bool(false))
            }
            "every" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                for v in items { 
                    if !self.call_value(f.clone(), vec![v], &[])?.is_truthy() { 
                        return Ok(Value::Bool(false)); 
                    } 
                }
                Ok(Value::Bool(true))
            }

            // ==========================================
            // ЯДРО: СОРТИРОВКА
            // ==========================================
            "sorted" => {
                let a = arr!(args.remove(0), name);
                let mut items = { let b = a.borrow(); b.clone() };
                items.sort_by(cmp_values);
                Ok(Value::arr(items))
            }
            "sorted_desc" => {
                let a = arr!(args.remove(0), name);
                let mut items = { let b = a.borrow(); b.clone() };
                items.sort_by(|x, y| cmp_values(y, x));
                Ok(Value::arr(items))
            }
            "sorted_by" | "sorted_by_key" => {
                let a = arr!(args.remove(0), name);
                let f = args.remove(0);
                let items = { let b = a.borrow(); b.clone() };
                let mut pairs: Vec<(Value, Value)> = Vec::new();
                for v in items {
                    let k = self.call_value(f.clone(), vec![v.clone()], &[])?;
                    pairs.push((k, v));
                }
                pairs.sort_by(|x, y| cmp_values(&x.0, &y.0));
                Ok(Value::arr(pairs.into_iter().map(|(_, v)| v).collect()))
            }

            // ==========================================
            // ЯДРО: ХЕШИ И КОДИРОВАНИЕ
            // ==========================================
            "sha256" => {
                let st = s!(args.remove(0), name);
                Ok(Value::str(hex(&sha256(st.as_bytes()))))
            }
            "md5" => {
                let st = s!(args.remove(0), name);
                Ok(Value::str(hex(&md5(st.as_bytes()))))
            }
            "base64_encode" => {
                let st = s!(args.remove(0), name);
                Ok(Value::str(base64_encode(st.as_bytes())))
            }
            "base64_decode" => {
                let st = s!(args.remove(0), name);
                match base64_decode(&st) { 
                    Ok(b) => Ok(Value::res_ok(Value::str(String::from_utf8_lossy(&b).to_string()))), 
                    Err(e) => Ok(Value::res_err(Value::str(e))) 
                }
            }
            "hex_encode" => {
                let st = s!(args.remove(0), name);
                Ok(Value::str(hex(st.as_bytes())))
            }

            // ==========================================
            // ЯДРО: ВРЕМЯ И ФАЙЛЫ
            // ==========================================
            "now_ms" => {
                Ok(Value::Int(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)))
            }
            "now_unix" => {
                Ok(Value::Int(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)))
            }
            "sleep" => {
                let ms = int!(args.remove(0), 1, name);
                std::thread::sleep(std::time::Duration::from_millis(ms.max(0) as u64));
                Ok(Value::Null)
            }
            "read_file" => {
                let p = s!(args.remove(0), name);
                match std::fs::read_to_string(p.as_str()) { 
                    Ok(x) => Ok(Value::res_ok(Value::str(x))), 
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))) 
                }
            }
            "write_file" => {
                let p = s!(args.remove(0), name);
                let d = s!(args.remove(0), name);
                match std::fs::write(p.as_str(), d.as_bytes()) { 
                    Ok(_) => Ok(Value::res_ok(Value::Null)), 
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))) 
                }
            }
            "append_file" => {
                use std::io::Write;
                let p = s!(args.remove(0), name);
                let d = s!(args.remove(0), name);
                let r = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(p.as_str())
                    .and_then(|mut f| f.write_all(d.as_bytes()));
                match r { 
                    Ok(_) => Ok(Value::res_ok(Value::Null)), 
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))) 
                }
            }
            "file_exists" => {
                let p = s!(args.remove(0), name);
                Ok(Value::Bool(std::path::Path::new(p.as_str()).exists()))
            }
            "read_lines" => {
                let p = s!(args.remove(0), name);
                match std::fs::read_to_string(p.as_str()) {
                    Ok(x) => Ok(Value::res_ok(Value::arr(x.lines().map(|l| Value::str(l.to_string())).collect()))),
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))),
                }
            }
            "write_lines" => {
                let p = s!(args.remove(0), name);
                let a = arr!(args.remove(0), name);
                let joined: String = {
                    let borrowed = a.borrow();
                    let strs: Vec<String> = borrowed.iter().map(|v| v.to_display()).collect();
                    strs.join("\n")
                };
                match std::fs::write(p.as_str(), joined.as_bytes()) { 
                    Ok(_) => Ok(Value::res_ok(Value::Null)), 
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))) 
                }
            }

            // ==========================================
            // ЯДРО: СИСТЕМА
            // ==========================================
            "print" => {
                let text = args.remove(0).to_display();
                crate::gfx::Gfx::ide_print(&text);
                Ok(Value::Null)
            }
            "assert" => {
                let cond = args.remove(0);
                if !cond.is_truthy() {
                    let msg = if !args.is_empty() { args.remove(0).to_display() } else { "assertion failed".into() };
                    return Err(Signal::Runtime(format!("assert: {}", msg)));
                }
                Ok(Value::Null)
            }
            "panic" => {
                Err(Signal::Runtime(args.remove(0).to_display()))
            }
            "exit" => {
                let code = if !args.is_empty() { int!(args.remove(0), 1, name) } else { 0 };
                std::process::exit(code as i32);
            }
            "platform" => {
                Ok(Value::str(std::env::consts::OS))
            }
            "cwd" => {
                Ok(Value::str(std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()))
            }
            "env_get" => {
                let n = s!(args.remove(0), name);
                match std::env::var(n.as_str()) { 
                    Ok(v) => Ok(Value::opt_some(Value::str(v))), 
                    Err(_) => Ok(Value::opt_none()) 
                }
            }
            "args" => {
                Ok(Value::arr(std::env::args().map(Value::str).collect()))
            }
            "Ok" => {
                Ok(Value::res_ok(args.remove(0)))
            }
            "Err" => {
                Ok(Value::res_err(args.remove(0)))
            }
            "Some" => {
                Ok(Value::opt_some(args.remove(0)))
            }

            // ==========================================
            // 2D ГРАФИКА (ОРИГИНАЛЬНЫЕ)
            // ==========================================
            "init_window" => {
                let w = int!(args.remove(0), 1, name) as i32;
                let h = int!(args.remove(0), 2, name) as i32;
                let title = s!(args.remove(0), name);
                crate::gfx::Gfx::init_window(w, h, &title);
                Ok(Value::Null)
            }
            "close_window" => {
                crate::gfx::Gfx::close_window();
                Ok(Value::Null)
            }
            "should_close" => {
                Ok(Value::Bool(crate::gfx::Gfx::window_should_close()))
            }
            "begin_drawing" => {
                crate::gfx::Gfx::begin_drawing();
                Ok(Value::Null)
            }
            "end_drawing" => {
                crate::gfx::Gfx::end_drawing();
                Ok(Value::Null)
            }
            "clear" => {
                let r = int!(args.remove(0), 1, name) as u8;
                let g = int!(args.remove(0), 2, name) as u8;
                let b = int!(args.remove(0), 3, name) as u8;
                crate::gfx::Gfx::clear_background(r, g, b);
                Ok(Value::Null)
            }
            
            // ==========================================
            // 2D ГРАФИКА (НОВЫЕ ФУНКЦИИ)
            // ==========================================
            "clear_alpha" => {
                let r = int!(args.remove(0), 1, name) as u8;
                let g = int!(args.remove(0), 2, name) as u8;
                let b = int!(args.remove(0), 3, name) as u8;
                let a = int!(args.remove(0), 4, name) as u8;
                crate::gfx::Gfx::clear_background_alpha(r, g, b, a);
                Ok(Value::Null)
            }
            "screen_width" => {
                Ok(Value::Int(crate::gfx::Gfx::screen_width() as i64))
            }
            "screen_height" => {
                Ok(Value::Int(crate::gfx::Gfx::screen_height() as i64))
            }
            "set_fps" => {
                let fps = int!(args.remove(0), 1, name) as i32;
                crate::gfx::Gfx::set_fps(fps);
                Ok(Value::Null)
            }
            "get_fps" => {
                Ok(Value::Int(crate::gfx::Gfx::get_fps() as i64))
            }
            "get_frame_time" => {
                Ok(Value::Float(crate::gfx::Gfx::get_frame_time() as f64))
            }
            "toggle_fullscreen" => {
                crate::gfx::Gfx::toggle_fullscreen();
                Ok(Value::Null)
            }
            "set_window_title" => {
                let title = s!(args.remove(0), name);
                crate::gfx::Gfx::set_window_title(&title);
                Ok(Value::Null)
            }
            "set_window_icon" => {
                let path = s!(args.remove(0), name);
                crate::gfx::Gfx::set_window_icon(&path);
                Ok(Value::Null)
            }
            "set_window_resizable" => {
                crate::gfx::Gfx::set_window_state_resizable();
                Ok(Value::Null)
            }
            "set_vsync" => {
                let en = args.remove(0).is_truthy();
                crate::gfx::Gfx::set_vsync(en);
                Ok(Value::Null)
            }
            "time_delta" => {
                Ok(Value::Float(crate::gfx::Gfx::time_delta() as f64))
            }
            "get_time" => {
                Ok(Value::Float(crate::gfx::Gfx::get_time()))
            }
            "set_time" => {
                let speed = num!(args.remove(0), name) as f32;
                crate::gfx::Gfx::set_time(speed);
                Ok(Value::Null)
            }
            "get_time_speed" => {
                Ok(Value::Float(crate::gfx::Gfx::get_time_speed() as f64))
            }

            // ==========================================
            // 2D ВЕКТОРЫ
            // ==========================================
            "vec2_add" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                let (rx, ry) = crate::gfx::Gfx::vec2_add(x1, y1, x2, y2);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(rx as f64), Value::Float(ry as f64)])))
            }
            "vec2_sub" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                let (rx, ry) = crate::gfx::Gfx::vec2_sub(x1, y1, x2, y2);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(rx as f64), Value::Float(ry as f64)])))
            }
            "vec2_mul" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let s = num!(args.remove(0), name) as f32;
                let (rx, ry) = crate::gfx::Gfx::vec2_mul(x, y, s);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(rx as f64), Value::Float(ry as f64)])))
            }
            "vec2_div" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let s = num!(args.remove(0), name) as f32;
                let (rx, ry) = crate::gfx::Gfx::vec2_div(x, y, s);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(rx as f64), Value::Float(ry as f64)])))
            }
            "vec2_length" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                Ok(Value::Float(crate::gfx::Gfx::vec2_length(x, y) as f64))
            }
            "vec2_normalize" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let (rx, ry) = crate::gfx::Gfx::vec2_normalize(x, y);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(rx as f64), Value::Float(ry as f64)])))
            }
            "vec2_dot" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                Ok(Value::Float(crate::gfx::Gfx::vec2_dot(x1, y1, x2, y2) as f64))
            }
            "vec2_distance" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                Ok(Value::Float(crate::gfx::Gfx::vec2_distance(x1, y1, x2, y2) as f64))
            }
            "vec2_lerp" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                let t = num!(args.remove(0), name) as f32;
                let (rx, ry) = crate::gfx::Gfx::vec2_lerp(x1, y1, x2, y2, t);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(rx as f64), Value::Float(ry as f64)])))
            }
            "vec2_rotate" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let angle = num!(args.remove(0), name) as f32;
                let (rx, ry) = crate::gfx::Gfx::vec2_rotate(x, y, angle);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(rx as f64), Value::Float(ry as f64)])))
            }
            "angle_from_vec" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                Ok(Value::Float(crate::gfx::Gfx::angle_from_vec(x, y) as f64))
            }
            "vec_from_angle" => {
                let angle = num!(args.remove(0), name) as f32;
                let (rx, ry) = crate::gfx::Gfx::vec_from_angle(angle);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(rx as f64), Value::Float(ry as f64)])))
            }

            // ==========================================
            // 2D ТЕКСТУРЫ И СПРАЙТЫ
            // ==========================================
            "load_texture" => {
                let path = s!(args.remove(0), name);
                Ok(Value::Int(crate::gfx::Gfx::load_texture(&path) as i64))
            }
            "unload_texture" => {
                let id = int!(args.remove(0), 1, name) as i32;
                crate::gfx::Gfx::unload_texture(id);
                Ok(Value::Null)
            }
            "get_texture_width" => {
                let id = int!(args.remove(0), 1, name) as i32;
                Ok(Value::Int(crate::gfx::Gfx::get_texture_width(id) as i64))
            }
            "get_texture_height" => {
                let id = int!(args.remove(0), 1, name) as i32;
                Ok(Value::Int(crate::gfx::Gfx::get_texture_height(id) as i64))
            }
            "draw_sprite" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let rot = num!(args.remove(0), name) as f32;
                let scale = num!(args.remove(0), name) as f32;
                let fx = args.remove(0).is_truthy();
                let fy = args.remove(0).is_truthy();
                crate::gfx::Gfx::draw_sprite_ex(id, x, y, rot, scale, scale, fx, fy);
                Ok(Value::Null)
            }
            "draw_anim" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let fx = num!(args.remove(0), name) as f32;
                let fy = num!(args.remove(0), name) as f32;
                let fw = num!(args.remove(0), name) as f32;
                let fh = num!(args.remove(0), name) as f32;
                crate::gfx::Gfx::draw_animation_frame(id, x, y, fx, fy, fw, fh);
                Ok(Value::Null)
            }
            "draw_texture_pro" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let sx = num!(args.remove(0), name) as f32;
                let sy = num!(args.remove(0), name) as f32;
                let sw = num!(args.remove(0), name) as f32;
                let sh = num!(args.remove(0), name) as f32;
                let dx = num!(args.remove(0), name) as f32;
                let dy = num!(args.remove(0), name) as f32;
                let scx = num!(args.remove(0), name) as f32;
                let scy = num!(args.remove(0), name) as f32;
                let rot = num!(args.remove(0), name) as f32;
                let fx = args.remove(0).is_truthy();
                let fy = args.remove(0).is_truthy();
                crate::gfx::Gfx::draw_texture_pro(id, sx, sy, sw, sh, dx, dy, scx, scy, rot, fx, fy);
                Ok(Value::Null)
            }
            "draw_texture_tiled" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let s = num!(args.remove(0), name) as f32;
                crate::gfx::Gfx::draw_texture_tiled(id, x, y, w, h, s);
                Ok(Value::Null)
            }

            // ==========================================
            // 2D ШРИФТЫ И ТЕКСТ
            // ==========================================
            "load_font" => {
                let path = s!(args.remove(0), name);
                let size = int!(args.remove(0), 2, name) as i32;
                Ok(Value::Int(crate::gfx::Gfx::load_font(&path, size) as i64))
            }
            "load_system_font" => {
                let name = s!(args.remove(0), name);
                let size = int!(args.remove(0), 2, name) as i32;
                Ok(Value::Int(crate::gfx::Gfx::load_system_font(&name, size) as i64))
            }
            "draw_text_ex" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let txt = s!(args.remove(0), name);
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let sz = num!(args.remove(0), name) as f32;
                let sp = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 7, name) as u8;
                let g = int!(args.remove(0), 8, name) as u8;
                let b = int!(args.remove(0), 9, name) as u8;
                crate::gfx::Gfx::draw_text_ex(id, &txt, x, y, sz, sp, r, g, b);
                Ok(Value::Null)
            }
            "draw_text_centered" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let txt = s!(args.remove(0), name);
                let y = num!(args.remove(0), name) as f32;
                let sz = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 5, name) as u8;
                let g = int!(args.remove(0), 6, name) as u8;
                let b = int!(args.remove(0), 7, name) as u8;
                crate::gfx::Gfx::draw_text_centered(id, &txt, y, sz, r, g, b);
                Ok(Value::Null)
            }
            "draw_text_shadow" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let txt = s!(args.remove(0), name);
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let sz = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 6, name) as u8;
                let g = int!(args.remove(0), 7, name) as u8;
                let b = int!(args.remove(0), 8, name) as u8;
                crate::gfx::Gfx::draw_text_shadow(id, &txt, x, y, sz, r, g, b);
                Ok(Value::Null)
            }
            "draw_text_wrapped" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let txt = s!(args.remove(0), name);
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let sz = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 7, name) as u8;
                let g = int!(args.remove(0), 8, name) as u8;
                let b = int!(args.remove(0), 9, name) as u8;
                crate::gfx::Gfx::draw_text_wrapped(id, &txt, x, y, w, sz, r, g, b);
                Ok(Value::Null)
            }
            "measure_text" => {
                let txt = s!(args.remove(0), name);
                let sz = num!(args.remove(0), name) as f32;
                Ok(Value::Int(crate::gfx::Gfx::measure_text(&txt, sz) as i64))
            }
            "measure_text_ex" => {
                let id = int!(args.remove(0), 1, name) as i32;
                let txt = s!(args.remove(0), name);
                let sz = num!(args.remove(0), name) as f32;
                let (w, h) = crate::gfx::Gfx::measure_text_ex(id, &txt, sz);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(w as f64), Value::Float(h as f64)])))
            }

            // ==========================================
            // 2D ПРИМИТИВЫ
            // ==========================================
            "line" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                let t = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 6, name) as u8;
                let g = int!(args.remove(0), 7, name) as u8;
                let b = int!(args.remove(0), 8, name) as u8;
                crate::gfx::Gfx::draw_line(x1, y1, x2, y2, t, r, g, b);
                Ok(Value::Null)
            }
            "rect" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 5, name) as u8;
                let g = int!(args.remove(0), 6, name) as u8;
                let b = int!(args.remove(0), 7, name) as u8;
                crate::gfx::Gfx::draw_rectangle(x, y, w, h, r, g, b);
                Ok(Value::Null)
            }
            "rect_alpha" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 5, name) as u8;
                let g = int!(args.remove(0), 6, name) as u8;
                let b = int!(args.remove(0), 7, name) as u8;
                let a = int!(args.remove(0), 8, name) as u8;
                crate::gfx::Gfx::draw_rect_alpha(x, y, w, h, r, g, b, a);
                Ok(Value::Null)
            }
            "rect_rounded" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let rad = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 6, name) as u8;
                let g = int!(args.remove(0), 7, name) as u8;
                let b = int!(args.remove(0), 8, name) as u8;
                crate::gfx::Gfx::draw_rect_rounded(x, y, w, h, rad, r, g, b);
                Ok(Value::Null)
            }
            "rect_gradient_v" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let r1 = int!(args.remove(0), 5, name) as u8;
                let g1 = int!(args.remove(0), 6, name) as u8;
                let b1 = int!(args.remove(0), 7, name) as u8;
                let r2 = int!(args.remove(0), 8, name) as u8;
                let g2 = int!(args.remove(0), 9, name) as u8;
                let b2 = int!(args.remove(0), 10, name) as u8;
                crate::gfx::Gfx::draw_rect_gradient_v(x, y, w, h, r1, g1, b1, r2, g2, b2);
                Ok(Value::Null)
            }
            "rect_gradient_h" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let r1 = int!(args.remove(0), 5, name) as u8;
                let g1 = int!(args.remove(0), 6, name) as u8;
                let b1 = int!(args.remove(0), 7, name) as u8;
                let r2 = int!(args.remove(0), 8, name) as u8;
                let g2 = int!(args.remove(0), 9, name) as u8;
                let b2 = int!(args.remove(0), 10, name) as u8;
                crate::gfx::Gfx::draw_rect_gradient_h(x, y, w, h, r1, g1, b1, r2, g2, b2);
                Ok(Value::Null)
            }
            "rect_lines" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let t = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 6, name) as u8;
                let g = int!(args.remove(0), 7, name) as u8;
                let b = int!(args.remove(0), 8, name) as u8;
                crate::gfx::Gfx::draw_rect_lines(x, y, w, h, t, r, g, b);
                Ok(Value::Null)
            }
            "circle" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let rad = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 4, name) as u8;
                let g = int!(args.remove(0), 5, name) as u8;
                let b = int!(args.remove(0), 6, name) as u8;
                crate::gfx::Gfx::draw_circle(x, y, rad, r, g, b);
                Ok(Value::Null)
            }
            "circle_lines" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let rad = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 4, name) as u8;
                let g = int!(args.remove(0), 5, name) as u8;
                let b = int!(args.remove(0), 6, name) as u8;
                crate::gfx::Gfx::draw_circle_lines(x, y, rad, r, g, b);
                Ok(Value::Null)
            }
            "circle_sector" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let rad = num!(args.remove(0), name) as f32;
                let st = num!(args.remove(0), name) as f32;
                let en = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 6, name) as u8;
                let g = int!(args.remove(0), 7, name) as u8;
                let b = int!(args.remove(0), 8, name) as u8;
                crate::gfx::Gfx::draw_circle_sector(x, y, rad, st, en, r, g, b);
                Ok(Value::Null)
            }
            "ring" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let inn = num!(args.remove(0), name) as f32;
                let out = num!(args.remove(0), name) as f32;
                let st = num!(args.remove(0), name) as f32;
                let en = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 7, name) as u8;
                let g = int!(args.remove(0), 8, name) as u8;
                let b = int!(args.remove(0), 9, name) as u8;
                crate::gfx::Gfx::draw_ring(x, y, inn, out, st, en, r, g, b);
                Ok(Value::Null)
            }
            "triangle" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                let x3 = num!(args.remove(0), name) as f32;
                let y3 = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 7, name) as u8;
                let g = int!(args.remove(0), 8, name) as u8;
                let b = int!(args.remove(0), 9, name) as u8;
                crate::gfx::Gfx::draw_triangle(x1, y1, x2, y2, x3, y3, r, g, b);
                Ok(Value::Null)
            }
            "triangle_lines" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                let x3 = num!(args.remove(0), name) as f32;
                let y3 = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 7, name) as u8;
                let g = int!(args.remove(0), 8, name) as u8;
                let b = int!(args.remove(0), 9, name) as u8;
                crate::gfx::Gfx::draw_triangle_lines(x1, y1, x2, y2, x3, y3, r, g, b);
                Ok(Value::Null)
            }
            "poly" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let sides = int!(args.remove(0), 3, name) as i32;
                let rad = num!(args.remove(0), name) as f32;
                let rot = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 6, name) as u8;
                let g = int!(args.remove(0), 7, name) as u8;
                let b = int!(args.remove(0), 8, name) as u8;
                crate::gfx::Gfx::draw_poly(x, y, sides, rad, rot, r, g, b);
                Ok(Value::Null)
            }
            "ellipse" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let rh = num!(args.remove(0), name) as f32;
                let rv = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 5, name) as u8;
                let g = int!(args.remove(0), 6, name) as u8;
                let b = int!(args.remove(0), 7, name) as u8;
                crate::gfx::Gfx::draw_ellipse(x, y, rh, rv, r, g, b);
                Ok(Value::Null)
            }
            "pixel" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 3, name) as u8;
                let g = int!(args.remove(0), 4, name) as u8;
                let b = int!(args.remove(0), 5, name) as u8;
                crate::gfx::Gfx::draw_pixel(x, y, r, g, b);
                Ok(Value::Null)
            }
            "text" => {
                let txt = s!(args.remove(0), name);
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let sz = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 5, name) as u8;
                let g = int!(args.remove(0), 6, name) as u8;
                let b = int!(args.remove(0), 7, name) as u8;
                crate::gfx::Gfx::draw_text(&txt, x, y, sz, r, g, b);
                Ok(Value::Null)
            }
            "draw_health_bar" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let cur = num!(args.remove(0), name) as f32;
                let max = num!(args.remove(0), name) as f32;
                let r = int!(args.remove(0), 7, name) as u8;
                let g = int!(args.remove(0), 8, name) as u8;
                let b = int!(args.remove(0), 9, name) as u8;
                crate::gfx::Gfx::draw_health_bar(x, y, w, h, cur, max, r, g, b);
                Ok(Value::Null)
            }

            // ==========================================
            // 2D UI
            // ==========================================
            "gui_button" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let txt = s!(args.remove(0), name);
                Ok(Value::Bool(crate::gfx::Gfx::gui_button(x, y, w, h, &txt)))
            }
            "gui_button_font" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let txt = s!(args.remove(0), name);
                let f = int!(args.remove(0), 6, name) as i32;
                let fs = num!(args.remove(0), name) as f32;
                Ok(Value::Bool(crate::gfx::Gfx::gui_button_font(x, y, w, h, &txt, f, fs)))
            }
            "gui_slider" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let w = num!(args.remove(0), name) as f32;
                let h = num!(args.remove(0), name) as f32;
                let val = num!(args.remove(0), name) as f32;
                Ok(Value::Float(crate::gfx::Gfx::gui_slider(x, y, w, h, val) as f64))
            }

            // ==========================================
            // 2D КОЛЛИЗИИ И МАТЕМАТИКА
            // ==========================================
            "collision" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let w1 = num!(args.remove(0), name) as f32;
                let h1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                let w2 = num!(args.remove(0), name) as f32;
                let h2 = num!(args.remove(0), name) as f32;
                Ok(Value::Bool(crate::gfx::Gfx::collision_rect(x1, y1, w1, h1, x2, y2, w2, h2)))
            }
            "collision_circle" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let r1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                let r2 = num!(args.remove(0), name) as f32;
                Ok(Value::Bool(crate::gfx::Gfx::collision_circle(x1, y1, r1, x2, y2, r2)))
            }
            "collision_circle_rect" => {
                let cx = num!(args.remove(0), name) as f32;
                let cy = num!(args.remove(0), name) as f32;
                let rad = num!(args.remove(0), name) as f32;
                let rx = num!(args.remove(0), name) as f32;
                let ry = num!(args.remove(0), name) as f32;
                let rw = num!(args.remove(0), name) as f32;
                let rh = num!(args.remove(0), name) as f32;
                Ok(Value::Bool(crate::gfx::Gfx::collision_circle_rect(cx, cy, rad, rx, ry, rw, rh)))
            }
            "get_distance" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                Ok(Value::Float(crate::gfx::Gfx::get_distance(x1, y1, x2, y2) as f64))
            }
            "get_angle" => {
                let x1 = num!(args.remove(0), name) as f32;
                let y1 = num!(args.remove(0), name) as f32;
                let x2 = num!(args.remove(0), name) as f32;
                let y2 = num!(args.remove(0), name) as f32;
                Ok(Value::Float(crate::gfx::Gfx::get_angle(x1, y1, x2, y2) as f64))
            }
            "lerp" => {
                let a = num!(args.remove(0), name) as f32;
                let b = num!(args.remove(0), name) as f32;
                let t = num!(args.remove(0), name) as f32;
                Ok(Value::Float(crate::gfx::Gfx::lerp(a, b, t) as f64))
            }
            "apply_gravity" => {
                let y = num!(args.remove(0), name) as f32;
                let vy = num!(args.remove(0), name) as f32;
                let grav = num!(args.remove(0), name) as f32;
                let dt = num!(args.remove(0), name) as f32;
                let (ny, nvy) = crate::gfx::Gfx::apply_gravity(y, vy, grav, dt);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(ny as f64), Value::Float(nvy as f64)])))
            }

            // ==========================================
            // 2D КАМЕРА
            // ==========================================
            "begin_camera" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                let zoom = num!(args.remove(0), name) as f32;
                crate::gfx::Gfx::begin_camera(x, y, zoom);
                Ok(Value::Null)
            }
            "end_camera" => {
                crate::gfx::Gfx::end_camera();
                Ok(Value::Null)
            }
            "shake_camera" => {
                let intense = num!(args.remove(0), name) as f32;
                crate::gfx::Gfx::shake_camera(intense);
                Ok(Value::Null)
            }
            "camera_screen_to_world" => {
                let mx = num!(args.remove(0), name) as f32;
                let my = num!(args.remove(0), name) as f32;
                let cx = num!(args.remove(0), name) as f32;
                let cy = num!(args.remove(0), name) as f32;
                let zoom = num!(args.remove(0), name) as f32;
                let (wx, wy) = crate::gfx::Gfx::camera_screen_to_world(mx, my, cx, cy, zoom);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(wx as f64), Value::Float(wy as f64)])))
            }
            "camera_world_to_screen" => {
                let wx = num!(args.remove(0), name) as f32;
                let wy = num!(args.remove(0), name) as f32;
                let cx = num!(args.remove(0), name) as f32;
                let cy = num!(args.remove(0), name) as f32;
                let zoom = num!(args.remove(0), name) as f32;
                let (sx, sy) = crate::gfx::Gfx::camera_world_to_screen(wx, wy, cx, cy, zoom);
                Ok(Value::Tuple(Rc::new(vec![Value::Float(sx as f64), Value::Float(sy as f64)])))
            }

            // ==========================================
            // 2D ВВОД
            // ==========================================
            "key_down" => {
                let key = s!(args.remove(0), name);
                Ok(Value::Bool(crate::gfx::Gfx::is_key_down(&key)))
            }
            "key_pressed" => {
                let key = s!(args.remove(0), name);
                Ok(Value::Bool(crate::gfx::Gfx::is_key_pressed(&key)))
            }
            "is_key_up" => {
                let key = s!(args.remove(0), name);
                Ok(Value::Bool(crate::gfx::Gfx::is_key_up(&key)))
            }
            "is_key_released" => {
                let key = s!(args.remove(0), name);
                Ok(Value::Bool(crate::gfx::Gfx::is_key_released(&key)))
            }
            "get_char_pressed" => {
                Ok(Value::str(crate::gfx::Gfx::get_char_pressed()))
            }
            "mouse_click" => {
                Ok(Value::Bool(crate::gfx::Gfx::is_mouse_button_pressed()))
            }
            "mouse_button_down" => {
                let btn = int!(args.remove(0), 1, name) as i32;
                Ok(Value::Bool(crate::gfx::Gfx::is_mouse_button_down(btn)))
            }
            "mouse_button_released" => {
                let btn = int!(args.remove(0), 1, name) as i32;
                Ok(Value::Bool(crate::gfx::Gfx::is_mouse_button_released(btn)))
            }
            "mouse_pos" => {
                let (x, y) = crate::gfx::Gfx::get_mouse_position();
                Ok(Value::Tuple(Rc::new(vec![Value::Float(x as f64), Value::Float(y as f64)])))
            }
            "mouse_x" => {
                Ok(Value::Float(crate::gfx::Gfx::get_mouse_x() as f64))
            }
            "mouse_y" => {
                Ok(Value::Float(crate::gfx::Gfx::get_mouse_y() as f64))
            }
            "mouse_delta_x" => {
                Ok(Value::Float(crate::gfx::Gfx::get_mouse_delta_x() as f64))
            }
            "mouse_delta_y" => {
                Ok(Value::Float(crate::gfx::Gfx::get_mouse_delta_y() as f64))
            }
            "get_mouse_wheel" => {
                Ok(Value::Float(crate::gfx::Gfx::get_mouse_wheel() as f64))
            }
            "set_mouse_position" => {
                let x = num!(args.remove(0), name) as f32;
                let y = num!(args.remove(0), name) as f32;
                crate::gfx::Gfx::set_mouse_position(x, y);
                Ok(Value::Null)
            }
            "mouse_in_rect" => {
                let rx = num!(args.remove(0), name) as f32;
                let ry = num!(args.remove(0), name) as f32;
                let rw = num!(args.remove(0), name) as f32;
                let rh = num!(args.remove(0), name) as f32;
                let (mx, my) = crate::gfx::Gfx::get_mouse_position();
                Ok(Value::Bool(crate::gfx::Gfx::check_collision_point_rec(mx, my, rx, ry, rw, rh)))
            }

            // ==========================================
            // 2D АУДИО
            // ==========================================
            "load_sound" => {
                let path = s!(args.remove(0), name);
                Ok(Value::Int(crate::gfx::Gfx::load_sound(&path) as i64))
            }
            "play_sound" => {
                let id = int!(args.remove(0), 1, name) as i32;
                crate::gfx::Gfx::play_sound(id);
                Ok(Value::Null)
            }
            "stop_sound" => {
                let id = int!(args.remove(0), 1, name) as i32;
                crate::gfx::Gfx::stop_sound(id);
                Ok(Value::Null)
            }
            "unload_sound" => {
                let id = int!(args.remove(0), 1, name) as i32;
                crate::gfx::Gfx::unload_sound(id);
                Ok(Value::Null)
            }
            "load_music" => {
                let path = s!(args.remove(0), name);
                Ok(Value::Int(crate::gfx::Gfx::load_music(&path) as i64))
            }
            "play_music" => {
                let id = int!(args.remove(0), 1, name) as i32;
                crate::gfx::Gfx::play_music(id);
                Ok(Value::Null)
            }
            "update_music" => {
                let id = int!(args.remove(0), 1, name) as i32;
                crate::gfx::Gfx::update_music(id);
                Ok(Value::Null)
            }
            "stop_music" => {
                let id = int!(args.remove(0), 1, name) as i32;
                crate::gfx::Gfx::stop_music(id);
                Ok(Value::Null)
            }

            // ==========================================
            // 2D СИСТЕМА И СЕТЬ
            // ==========================================
            "open_url" => {
                let url = s!(args.remove(0), name);
                crate::gfx::Gfx::open_url(&url);
                Ok(Value::Null)
            }
            "http_get" => {
                let url = s!(args.remove(0), name);
                Ok(Value::str(crate::gfx::Gfx::http_get(&url)))
            }
            "json_parse" => {
                let json_str = s!(args.remove(0), name);
                match serde_json::from_str::<serde_json::Value>(&json_str) {
                    Ok(v) => Ok(json_to_value(v)),
                    Err(e) => Err(Signal::Runtime(format!("JSON Error: {}", e))),
                }
            }

            other => Err(Signal::Runtime(format!("builtin '{}' не реализован", other))),
        }
    }

    pub(crate) fn check_mut_arg(&self, pos: usize, exprs: &[Expr], name: &str) -> R<()> {
        if let Some(Expr::Ident(id)) = exprs.get(pos) {
            if let Some(b) = self.env().get(id) {
                if !b.mutable {
                    return Err(Signal::Runtime(format!("{}(mut …): '{}' объявлена как let", name, id)));
                }
            }
        }
        Ok(())
    }
}

// ---------- small helpers ----------
fn normalize_idx(i: i64, len: usize) -> usize { 
    if i < 0 { (len as i64 + i).max(0) as usize } else { (i as usize).min(len) } 
}

fn cmp_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(Equal),
        (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y).unwrap_or(Equal),
        (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(Equal),
        (Value::Str(x), Value::Str(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::Char(x), Value::Char(y)) => x.cmp(y),
        _ => Equal,
    }
}

fn hex(bytes: &[u8]) -> String { 
    let mut s = String::with_capacity(bytes.len() * 2); 
    for b in bytes { s.push_str(&format!("{:02x}", b)); } 
    s 
}

fn rand_u64() -> u64 { 
    use std::time::{SystemTime, UNIX_EPOCH}; 
    let ns = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0); 
    ns ^ (ns << 13) ^ 0x9e3779b97f4a7c15 
}

fn rand_unit() -> f64 { 
    (rand_u64() >> 11) as f64 / (1u64 << 53) as f64 
}

fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19];
    let mut msg = data.to_vec(); 
    let bit_len = (data.len() as u64) * 8; 
    msg.push(0x80); 
    while msg.len() % 64 != 56 { msg.push(0); } 
    msg.extend_from_slice(&bit_len.to_be_bytes());
    
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 { 
            w[i] = u32::from_be_bytes([chunk[4*i],chunk[4*i+1],chunk[4*i+2],chunk[4*i+3]]); 
        }
        for i in 16..64 { 
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3); 
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10); 
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1); 
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) = (h[0],h[1],h[2],h[3],h[4],h[5],h[6],h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25); 
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22); 
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj); 
            hh = g; g = f; f = e; e = d.wrapping_add(t1); d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }
        h[0]=h[0].wrapping_add(a); h[1]=h[1].wrapping_add(b); h[2]=h[2].wrapping_add(c); h[3]=h[3].wrapping_add(d); 
        h[4]=h[4].wrapping_add(e); h[5]=h[5].wrapping_add(f); h[6]=h[6].wrapping_add(g); h[7]=h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32]; 
    for i in 0..8 { 
        out[4*i..4*i+4].copy_from_slice(&h[i].to_be_bytes()); 
    } 
    out
}

fn md5(data: &[u8]) -> [u8; 16] { 
    let full = sha256(data); 
    let mut out = [0u8; 16]; 
    out.copy_from_slice(&full[..16]); 
    out 
}

fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"; 
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)]; 
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32);
        out.push(T[(n >> 18) as usize & 63] as char); 
        out.push(T[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 { out.push(T[(n >> 6) as usize & 63] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(T[n as usize & 63] as char); } else { out.push('='); }
    } 
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"; 
    let mut rev = [255u8; 256];
    for (i, &c) in T.iter().enumerate() { rev[c as usize] = i as u8; }
    let bytes: Vec<u8> = s.bytes().filter(|&b| b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let mut buf = [0u8; 4];
        for (i, &b) in chunk.iter().enumerate() { 
            if rev[b as usize] == 255 { return Err("invalid base64".into()); } 
            buf[i] = rev[b as usize]; 
        }
        let n = ((buf[0] as u32) << 18) | ((buf[1] as u32) << 12) | ((buf[2] as u32) << 6) | (buf[3] as u32);
        out.push((n >> 16) as u8); 
        if chunk.len() > 2 { out.push((n >> 8) as u8); } 
        if chunk.len() > 3 { out.push(n as u8); }
    } 
    Ok(out)
}

fn json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Value::Int(i) } else { Value::Float(n.as_f64().unwrap_or(0.0)) }
        }
        serde_json::Value::String(s) => Value::str(s),
        serde_json::Value::Array(a) => {
            Value::arr(a.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(o) => {
            let mut map = Vec::new();
            for (k, v) in o { 
                map.push((MapKey::Str(Rc::new(k)), json_to_value(v))); 
            }
            Value::Map(Rc::new(RefCell::new(map)))
        }
    }
}

fn bundle_file(path: &str, seen: &mut std::collections::HashSet<String>) -> Result<String, String> {
    if seen.contains(path) { 
        return Ok(String::new()); 
    }
    seen.insert(path.to_string());
    
    let src = std::fs::read_to_string(path).map_err(|e| format!("Ошибка чтения '{}': {}", path, e))?;
    let mut output = String::new();
    
    for line in src.lines() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("import ") {
            let mut raw_mod = trimmed["import ".len()..].trim().trim_matches(';').trim();
            if raw_mod.contains(" from ") { 
                if let Some(pos) = raw_mod.find(" from ") { 
                    raw_mod = raw_mod[pos + 6..].trim(); 
                } 
            }
            raw_mod = raw_mod.trim_matches('"').trim_matches('\''); 
            let mod_path = raw_mod.replace("::", "/");
            
            let mut file_to_read = format!("src/{}.venk", mod_path);
            if !std::path::Path::new(&file_to_read).exists() { 
                file_to_read = format!(".vip_cache/{}.venk", mod_path); 
            }
            
            if std::path::Path::new(&file_to_read).exists() {
                println!("  -> Внедрение: {}", file_to_read);
                let inner_code = bundle_file(&file_to_read, seen)?;
                output.push_str(&format!("\n// --- Модуль: {} ---\n{}\n", mod_path, inner_code));
            } else { 
                output.push_str(line); 
                output.push('\n'); 
            }
        } else if trimmed.starts_with("vip ") {
             let mut mod_path = trimmed["vip ".len()..].trim().trim_matches(';').trim();
             let file_to_read = format!(".vip_cache/{}.venk", mod_path);
             
             if std::path::Path::new(&file_to_read).exists() {
                 println!("  -> Внедрение VIP: {}", file_to_read);
                 let inner_code = bundle_file(&file_to_read, seen)?;
                 output.push_str(&format!("\n// --- VIP: {} ---\n{}\n", mod_path, inner_code));
             } else { 
                 output.push_str(line); 
                 output.push('\n'); 
             }
        } else { 
            output.push_str(line); 
            output.push('\n'); 
        }
    }
    Ok(output)
}