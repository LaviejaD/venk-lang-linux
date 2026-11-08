use crate::interp::{Interp, Signal, R};
use crate::value::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::fs;
use std::path::Path;
use regex::Regex;

impl Interp {
    pub(crate) fn call_pvenk(&mut self, name: &str, mut args: Vec<Value>) -> R {
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

        match name {
            // ==========================================
            // ФАЙЛОВАЯ СИСТЕМА
            // ==========================================
            "pvenk_dir_create" => {
                let path = s!(args.remove(0), name);
                Ok(match fs::create_dir_all(path.as_str()) {
                    Ok(_) => Value::res_ok(Value::Null),
                    Err(e) => Value::res_err(Value::str(e.to_string())),
                })
            }
            "pvenk_dir_list" => {
                let path = s!(args.remove(0), name);
                let mut files = Vec::new();
                if let Ok(entries) = fs::read_dir(path.as_str()) {
                    for entry in entries.flatten() {
                        files.push(Value::str(entry.file_name().to_string_lossy().into_owned()));
                    }
                }
                Ok(Value::arr(files))
            }
            "pvenk_file_copy" => {
                let from = s!(args.remove(0), name);
                let to = s!(args.remove(0), name);
                Ok(match fs::copy(from.as_str(), to.as_str()) {
                    Ok(_) => Value::res_ok(Value::Null),
                    Err(e) => Value::res_err(Value::str(e.to_string())),
                })
            }
            "pvenk_file_move" => {
                let from = s!(args.remove(0), name);
                let to = s!(args.remove(0), name);
                Ok(match fs::rename(from.as_str(), to.as_str()) {
                    Ok(_) => Value::res_ok(Value::Null),
                    Err(e) => Value::res_err(Value::str(e.to_string())),
                })
            }
            "pvenk_file_delete" => {
                let path = s!(args.remove(0), name);
                Ok(match fs::remove_file(path.as_str()) {
                    Ok(_) => Value::res_ok(Value::Null),
                    Err(e) => Value::res_err(Value::str(e.to_string())),
                })
            }
            "pvenk_file_size" => {
                let path = s!(args.remove(0), name);
                Ok(match fs::metadata(path.as_str()) {
                    Ok(m) => Value::Int(m.len() as i64),
                    Err(_) => Value::Int(-1),
                })
            }
            "pvenk_path_join" => {
                let part1 = s!(args.remove(0), name);
                let part2 = s!(args.remove(0), name);
                let path = Path::new(&*part1).join(&*part2);
                Ok(Value::str(path.to_string_lossy().into_owned()))
            }
            "pvenk_path_exists" => {
                let path = s!(args.remove(0), name);
                Ok(Value::Bool(Path::new(path.as_str()).exists()))
            }

            // ==========================================
            // РЕГУЛЯРНЫЕ ВЫРАЖЕНИЯ (ИСПРАВЛЕН ПОРЯДОК)
            // ==========================================
            "pvenk_regex_match" => {
                let text = s!(args.remove(0), name);    // Текст первым!
                let pattern = s!(args.remove(0), name);
                match Regex::new(pattern.as_str()) {
                    Ok(re) => Ok(Value::Bool(re.is_match(text.as_str()))),
                    Err(e) => Err(Signal::Runtime(format!("Regex Error: {}", e))),
                }
            }
            "pvenk_regex_find" => {
                let text = s!(args.remove(0), name);    // Текст первым!
                let pattern = s!(args.remove(0), name);
                match Regex::new(pattern.as_str()) {
                    Ok(re) => {
                        if let Some(m) = re.find(text.as_str()) {
                            Ok(Value::str(m.as_str().to_string()))
                        } else {
                            Ok(Value::Null)
                        }
                    }
                    Err(e) => Err(Signal::Runtime(format!("Regex Error: {}", e))),
                }
            }
            "pvenk_regex_find_all" => {
                let text = s!(args.remove(0), name);    // Текст первым!
                let pattern = s!(args.remove(0), name);
                match Regex::new(pattern.as_str()) {
                    Ok(re) => {
                        let matches: Vec<Value> = re.find_iter(text.as_str())
                            .map(|m| Value::str(m.as_str().to_string()))
                            .collect();
                        Ok(Value::arr(matches))
                    }
                    Err(e) => Err(Signal::Runtime(format!("Regex Error: {}", e))),
                }
            }
            "pvenk_regex_replace" => {
                let text = s!(args.remove(0), name);    // Текст первым!
                let pattern = s!(args.remove(0), name);
                let replacement = s!(args.remove(0), name);
                match Regex::new(pattern.as_str()) {
                    Ok(re) => Ok(Value::str(re.replace_all(text.as_str(), replacement.as_str()).to_string())),
                    Err(e) => Err(Signal::Runtime(format!("Regex Error: {}", e))),
                }
            }
            "pvenk_regex_split" => {
                let text = s!(args.remove(0), name);    // Текст первым!
                let pattern = s!(args.remove(0), name);
                match Regex::new(pattern.as_str()) {
                    Ok(re) => {
                        let parts: Vec<Value> = re.split(text.as_str())
                            .map(|s| Value::str(s.to_string()))
                            .collect();
                        Ok(Value::arr(parts))
                    }
                    Err(e) => Err(Signal::Runtime(format!("Regex Error: {}", e))),
                }
            }

            // ==========================================
            // ВРЕМЯ И ДАТЫ
            // ==========================================
            "pvenk_time_now_str" => {
                let now = chrono::Local::now();
                Ok(Value::str(now.format("%Y-%m-%d %H:%M:%S").to_string()))
            }
            "pvenk_time_format" => {
                let timestamp = int!(args.remove(0), name);
                let format = s!(args.remove(0), name);
                if let Some(dt) = chrono::DateTime::from_timestamp(timestamp, 0) {
                    let local = dt.with_timezone(&chrono::Local);
                    Ok(Value::str(local.format(format.as_str()).to_string()))
                } else {
                    Ok(Value::str("Invalid timestamp".to_string()))
                }
            }

            other => Err(Signal::Runtime(format!("pvenk '{}' не реализован", other))),
        }
    }
}