use crate::interp::{Interp, Signal, R};
use crate::value::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashMap, VecDeque};
use std::process::Command;

// Глобальное состояние для системных функций
thread_local! {
    // Имитация кучи (Heap) для управления памятью
    static HEAP: RefCell<HashMap<usize, Vec<u8>>> = RefCell::new(HashMap::new());
    // Указатель на следующую свободную "ячейку" памяти
    static NEXT_PTR: RefCell<usize> = RefCell::new(0x1000); 
    
    // Контейнеры C++ (Очереди)
    static QUEUES: RefCell<HashMap<i32, VecDeque<Value>>> = RefCell::new(HashMap::new());
    static NEXT_ID: RefCell<i32> = RefCell::new(1);
}

impl Interp {
    pub(crate) fn call_cppvenk(&mut self, name: &str, mut args: Vec<Value>) -> R {
        macro_rules! int {
            ($v:expr, $name:expr) => {
                match $v {
                    Value::Int(n) => n,
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
            // 1. УПРАВЛЕНИЕ ПАМЯТЬЮ (C-style malloc/free)
            // ==========================================
            "cppvenk_mem_alloc" => {
                let size = int!(args.remove(0), name) as usize;
                let ptr = NEXT_PTR.with(|p| {
                    let mut ptr_val = p.borrow_mut();
                    let current = *ptr_val;
                    *ptr_val += size + 1; // Сдвигаем указатель (условно)
                    current
                });
                HEAP.with(|h| {
                    h.borrow_mut().insert(ptr, vec![0u8; size]);
                });
                Ok(Value::Int(ptr as i64))
            }
            "cppvenk_mem_free" => {
                let ptr = int!(args.remove(0), name) as usize;
                HEAP.with(|h| {
                    h.borrow_mut().remove(&ptr);
                });
                Ok(Value::Null)
            }
            "cppvenk_mem_write" => {
                let ptr = int!(args.remove(0), name) as usize;
                let offset = int!(args.remove(0), name) as usize;
                let byte = int!(args.remove(0), name) as u8;
                
                HEAP.with(|h| {
                    if let Some(block) = h.borrow_mut().get_mut(&ptr) {
                        if offset < block.len() {
                            block[offset] = byte;
                            return Ok(Value::Null);
                        }
                    }
                    Err(Signal::Runtime("Ошибка сегментации: выход за границы памяти".into()))
                })
            }
            "cppvenk_mem_read" => {
                let ptr = int!(args.remove(0), name) as usize;
                let offset = int!(args.remove(0), name) as usize;
                
                HEAP.with(|h| {
                    if let Some(block) = h.borrow().get(&ptr) {
                        if offset < block.len() {
                            return Ok(Value::Int(block[offset] as i64));
                        }
                    }
                    Err(Signal::Runtime("Ошибка чтения памяти".into()))
                })
            }
            "cppvenk_mem_dump" => {
                let ptr = int!(args.remove(0), name) as usize;
                HEAP.with(|h| {
                    if let Some(block) = h.borrow().get(&ptr) {
                        let str_data = String::from_utf8_lossy(block).into_owned();
                        Ok(Value::str(str_data))
                    } else {
                        Err(Signal::Runtime("Указатель не найден".into()))
                    }
                })
            }

            // ==========================================
            // 2. СИСТЕМНЫЕ ВЫЗОВЫ (OS Syscalls)
            // ==========================================
            "cppvenk_os_exec" => {
                let cmd = s!(args.remove(0), name);
                
                #[cfg(target_os = "windows")]
                let output = Command::new("cmd").args(&["/C", &cmd]).output();
                
                #[cfg(not(target_os = "windows"))]
                let output = Command::new("sh").args(&["-c", &cmd]).output();

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
                        Ok(Value::res_ok(Value::str(stdout)))
                    }
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))),
                }
            }
            "cppvenk_os_getpid" => {
                let pid = std::process::id();
                Ok(Value::Int(pid as i64))
            }
            "cppvenk_os_getenv" => {
                let key = s!(args.remove(0), name);
                match std::env::var(key.as_str()) {
                    Ok(val) => Ok(Value::opt_some(Value::str(val))),
                    Err(_) => Ok(Value::opt_none()),
                }
            }
            "cppvenk_os_setenv" => {
                let key = s!(args.remove(0), name);
                let val = s!(args.remove(0), name);
                std::env::set_var(key.as_str(), val.as_str());
                Ok(Value::Null)
            }

            // ==========================================
            // 3. ПОТОКИ (Multithreading / Processes)
            // ==========================================
            "cppvenk_thread_spawn" => {
                let script_path = s!(args.remove(0), name);
                
                // Мы запускаем новый процесс vekn (текущий исполняемый файл)
                let exe_path = std::env::current_exe().unwrap_or_default();
                let child = Command::new(exe_path)
                    .arg(script_path.as_str())
                    .spawn();

                match child {
                    Ok(c) => Ok(Value::res_ok(Value::Int(c.id() as i64))),
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))),
                }
            }
            "cppvenk_thread_sleep" => {
                let ms = int!(args.remove(0), name) as u64;
                std::thread::sleep(std::time::Duration::from_millis(ms));
                Ok(Value::Null)
            }

            // ==========================================
            // 4. АЛГОРИТМЫ C++ (STL - Queues)
            // ==========================================
            "cppvenk_queue_create" => {
                let id = NEXT_ID.with(|n| {
                    let mut id = n.borrow_mut();
                    let current = *id;
                    *id += 1;
                    current
                });
                QUEUES.with(|q| {
                    q.borrow_mut().insert(id, VecDeque::new());
                });
                Ok(Value::Int(id as i64))
            }
            "cppvenk_queue_push" => {
                let q_id = int!(args.remove(0), name) as i32;
                let item = args.remove(0);
                QUEUES.with(|q| {
                    if let Some(queue) = q.borrow_mut().get_mut(&q_id) {
                        queue.push_back(item);
                        Ok(Value::Null)
                    } else {
                        Err(Signal::Runtime("Очередь не найдена".into()))
                    }
                })
            }
            "cppvenk_queue_pop" => {
                let q_id = int!(args.remove(0), name) as i32;
                QUEUES.with(|q| {
                    if let Some(queue) = q.borrow_mut().get_mut(&q_id) {
                        if let Some(item) = queue.pop_front() {
                            Ok(Value::opt_some(item))
                        } else {
                            Ok(Value::opt_none())
                        }
                    } else {
                        Err(Signal::Runtime("Очередь не найдена".into()))
                    }
                })
            }
            "cppvenk_queue_size" => {
                let q_id = int!(args.remove(0), name) as i32;
                QUEUES.with(|q| {
                    if let Some(queue) = q.borrow().get(&q_id) {
                        Ok(Value::Int(queue.len() as i64))
                    } else {
                        Err(Signal::Runtime("Очередь не найдена".into()))
                    }
                })
            }

            other => Err(Signal::Runtime(format!("cppvenk '{}' не реализован", other))),
        }
    }
}