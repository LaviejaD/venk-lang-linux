use crate::ast::Program;
use crate::lexer;
use crate::parser;
use std::fs;

pub struct ImportManager;

impl ImportManager {
    pub fn load_module(path: &str) -> Result<Program, String> {
        // Если путь не содержит слешей и не заканчивается на .vex, 
        // значит это модуль из VIP (например `import my_lib`)
        let real_path = if !path.contains('/') && !path.ends_with(".venk") {
            // Ищем в src/ (локальный) или в .vip_cache/ (пакет)
            if std::path::Path::new(&format!("src/{}.venk", path)).exists() {
                format!("src/{}.venk", path)
            } else {
                crate::vip::VipManager::load_package(path)?
            }
        } else {
            path.to_string()
        };

        let src = fs::read_to_string(&real_path)
            .map_err(|e| format!("Ошибка чтения '{}': {}", real_path, e))?;
            
        let toks = lexer::lex(&src)
            .map_err(|e| format!("Синтаксис '{}': {}", real_path, e.msg))?;
            
        let prog = parser::parse(toks)
            .map_err(|e| format!("Парсинг '{}': {}", real_path, e.msg))?;
            
        Ok(prog)
    }
}