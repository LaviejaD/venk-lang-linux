use crate::interp::{Interp, Signal, R};
use crate::value::*;
use std::cell::RefCell;
use std::rc::Rc;
use serde_json::json;

impl Interp {
    pub(crate) fn call_jvenk(&mut self, name: &str, mut args: Vec<Value>) -> R {
        // Локальные макросы
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
            // JSON СЕРИАЛИЗАЦИЯ (JSON.stringify)
            // ==========================================
            "jvenk_json_stringify" => {
                let val = args.remove(0);
                let json_val = value_to_json(&val);
                match serde_json::to_string(&json_val) {
                    Ok(str_val) => Ok(Value::str(str_val)),
                    Err(e) => Err(Signal::Runtime(format!("JSON Stringify Error: {}", e))),
                }
            }

            // ==========================================
            // HTTP ЗАПРОСЫ (fetch, axios)
            // ==========================================
            "jvenk_http_get" => {
                let url = s!(args.remove(0), name);
                // Если передан второй аргумент (заголовки), пока игнорируем для простоты
                match ureq::get(url.as_str()).call() {
                    Ok(resp) => Ok(Value::res_ok(Value::str(resp.into_string().unwrap_or_default()))),
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))),
                }
            }
            "jvenk_http_post" => {
                let url = s!(args.remove(0), name);
                let body = s!(args.remove(0), name);
                
                // Отправляем POST-запрос с JSON-телом
                let response = ureq::post(url.as_str())
                    .set("Content-Type", "application/json")
                    .send_string(body.as_str());

                match response {
                    Ok(resp) => Ok(Value::res_ok(Value::str(resp.into_string().unwrap_or_default()))),
                    Err(e) => Ok(Value::res_err(Value::str(e.to_string()))),
                }
            }

            // ==========================================
            // URL КОДИРОВАНИЕ (encodeURIComponent)
            // ==========================================
            "jvenk_url_encode" => {
                let text = s!(args.remove(0), name);
                Ok(Value::str(url_encode(&text)))
            }
            "jvenk_url_decode" => {
                let text = s!(args.remove(0), name);
                Ok(Value::str(url_decode(&text)))
            }

            other => Err(Signal::Runtime(format!("jvenk '{}' не реализован", other))),
        }
    }
}

// ==========================================
// ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
// ==========================================

// Превращает объект Vekn в объект serde_json
fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => json!(*b),
        Value::Int(n) => json!(*n),
        Value::Float(f) => json!(*f),
        Value::Str(s) => json!((**s).clone()),
        Value::Array(a) => {
            let arr: Vec<serde_json::Value> = a.borrow().iter().map(value_to_json).collect();
            serde_json::Value::Array(arr)
        }
        Value::Record(r) | Value::Struct(_, r) => {
            let mut map = serde_json::Map::new();
            for (k, v) in r.borrow().iter() {
                map.insert(k.clone(), value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        Value::Map(m) => {
            let mut map = serde_json::Map::new();
            for (k, v) in m.borrow().iter() {
                map.insert(k.to_display(), value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        _ => serde_json::Value::Null,
    }
}

// Простое URL-кодирование (как в JS)
fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

// Простое URL-декодирование
fn url_decode(input: &str) -> String {
    let mut bytes = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();
    while i < chars.len() {
        if chars[i] == '%' && i + 2 < chars.len() {
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", chars[i+1], chars[i+2]), 16) {
                bytes.push(byte);
                i += 3;
                continue;
            }
        }
        if chars[i] == '+' {
            bytes.push(b' ');
        } else {
            let mut buf = [0; 4];
            let c_str = chars[i].encode_utf8(&mut buf);
            bytes.extend_from_slice(c_str.as_bytes());
        }
        i += 1;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}