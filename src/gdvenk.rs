use crate::interp::{Interp, Signal, R};
use crate::value::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::time::Instant;

// Глобальные таймеры и сигналы (привязаны к главному потоку)
thread_local! {
    static TIMERS: RefCell<HashMap<i32, Instant>> = RefCell::new(HashMap::new());
    static NEXT_TIMER_ID: RefCell<i32> = RefCell::new(1);
    static SIGNALS: RefCell<HashMap<String, Vec<Value>>> = RefCell::new(HashMap::new());
}

impl Interp {
    pub(crate) fn call_gdvenk(&mut self, name: &str, mut args: Vec<Value>) -> R {
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

        match name {
            // ==========================================
            // ВЕКТОРЫ (Vector2)
            // ==========================================
            "gdvenk_vec2" => {
                let x = num!(args.remove(0), name);
                let y = num!(args.remove(0), name);
                Ok(create_vec2(x, y))
            }
            "gdvenk_vec2_add" => {
                let (x1, y1) = extract_vec2(&args.remove(0));
                let (x2, y2) = extract_vec2(&args.remove(0));
                Ok(create_vec2(x1 + x2, y1 + y2))
            }
            "gdvenk_vec2_sub" => {
                let (x1, y1) = extract_vec2(&args.remove(0));
                let (x2, y2) = extract_vec2(&args.remove(0));
                Ok(create_vec2(x1 - x2, y1 - y2))
            }
            "gdvenk_vec2_mul" => {
                let (x, y) = extract_vec2(&args.remove(0));
                let scalar = num!(args.remove(0), name);
                Ok(create_vec2(x * scalar, y * scalar))
            }
            "gdvenk_vec2_length" => {
                let (x, y) = extract_vec2(&args.remove(0));
                Ok(Value::Float((x * x + y * y).sqrt()))
            }
            "gdvenk_vec2_normalize" => {
                let (x, y) = extract_vec2(&args.remove(0));
                let len = (x * x + y * y).sqrt();
                if len == 0.0 { return Ok(create_vec2(0.0, 0.0)); }
                Ok(create_vec2(x / len, y / len))
            }
            "gdvenk_vec2_distance" => {
                let (x1, y1) = extract_vec2(&args.remove(0));
                let (x2, y2) = extract_vec2(&args.remove(0));
                let dx = x2 - x1;
                let dy = y2 - y1;
                Ok(Value::Float((dx * dx + dy * dy).sqrt()))
            }

            // ==========================================
            // ТАЙМЕРЫ (Timer)
            // ==========================================
            "gdvenk_timer_start" => {
                let id = NEXT_TIMER_ID.with(|n| {
                    let mut id = n.borrow_mut();
                    let current = *id;
                    *id += 1;
                    current
                });
                TIMERS.with(|t| {
                    t.borrow_mut().insert(id, Instant::now());
                });
                Ok(Value::Int(id as i64))
            }
            "gdvenk_timer_elapsed" => {
                let id = match args.remove(0) {
                    Value::Int(n) => n as i32,
                    _ => return Err(Signal::Runtime("timer_elapsed: ожидался ID (int)".into())),
                };
                TIMERS.with(|t| {
                    if let Some(start) = t.borrow().get(&id) {
                        let elapsed = start.elapsed().as_secs_f64();
                        Ok(Value::Float(elapsed))
                    } else {
                        Ok(Value::Float(-1.0))
                    }
                })
            }
            "gdvenk_timer_done" => {
                let id = match args.remove(0) {
                    Value::Int(n) => n as i32,
                    _ => return Err(Signal::Runtime("timer_done: ожидался ID (int)".into())),
                };
                let duration = num!(args.remove(0), name);
                TIMERS.with(|t| {
                    if let Some(start) = t.borrow().get(&id) {
                        Ok(Value::Bool(start.elapsed().as_secs_f64() >= duration))
                    } else {
                        Ok(Value::Bool(false))
                    }
                })
            }

            // ==========================================
            // СИГНАЛЫ (Signals / Event Bus)
            // ==========================================
            "gdvenk_signal_connect" => {
                let signal_name = s!(args.remove(0), name);
                let func = args.remove(0);
                
                // Если это не функция — ошибка
                if !matches!(func, Value::Fn(_) | Value::Builtin(_)) {
                    return Err(Signal::Runtime("signal_connect: ожидалась функция".into()));
                }

                SIGNALS.with(|s| {
                    s.borrow_mut().entry((*signal_name).clone()).or_default().push(func);
                });
                Ok(Value::Null)
            }
            "gdvenk_signal_emit" => {
                let signal_name = s!(args.remove(0), name);
                
                // Забираем копию функций, чтобы не держать borrow во время вызова
                let funcs = SIGNALS.with(|s| {
                    s.borrow().get(&*signal_name).cloned().unwrap_or_default()
                });

                for f in funcs {
                    // Вызываем функцию без аргументов
                    self.call_value(f, vec![], &[])?;
                }
                Ok(Value::Null)
            }

            other => Err(Signal::Runtime(format!("gdvenk '{}' не реализован", other))),
        }
    }
}

// Хелпер: создает структуру Vec2 (как объект Vekn)
fn create_vec2(x: f64, y: f64) -> Value {
    let fields = vec![
        ("x".to_string(), Value::Float(x)),
        ("y".to_string(), Value::Float(y)),
    ];
    Value::Struct(Rc::new("Vec2".to_string()), Rc::new(RefCell::new(fields)))
}

// Хелпер: достает x и y из объекта Vec2
fn extract_vec2(v: &Value) -> (f64, f64) {
    if let Value::Struct(_, fields) = v {
        let f = fields.borrow();
        let x = f.iter().find(|(k, _)| k == "x").map(|(_, v)| match v { Value::Float(n) => *n, Value::Int(n) => *n as f64, _ => 0.0 }).unwrap_or(0.0);
        let y = f.iter().find(|(k, _)| k == "y").map(|(_, v)| match v { Value::Float(n) => *n, Value::Int(n) => *n as f64, _ => 0.0 }).unwrap_or(0.0);
        (x, y)
    } else {
        (0.0, 0.0)
    }
}