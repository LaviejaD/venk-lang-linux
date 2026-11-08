use std::fs;
use std::path::Path;

pub struct VipManager;

impl VipManager {
    /// Аналог `cargo new --lib`, создает структуру библиотеки
    pub fn create_library(name: &str) -> Result<(), String> {
        let path = Path::new(name);
        if path.exists() {
            return Err(format!("Папка '{}' уже существует", name));
        }

        // Создаем папки
        fs::create_dir_all(path.join("src")).map_err(|e| e.to_string())?;

        // Создаем манифест vip.json
        let manifest = format!(
            "{{\n  \"name\": \"{}\",\n  \"version\": \"0.1.0\",\n  \"author\": \"you\",\n  \"dependencies\": {{}}\n}}", 
            name
        );
        fs::write(path.join("vip.json"), manifest).map_err(|e| e.to_string())?;

        // Создаем файл библиотеки
        let lib_code = format!("// Библиотека {}\npub fn hello() {{\n    print(\"Привет из библиотеки {}!\");\n}}\n", name, name);
        fs::write(path.join("src").join("lib.venk"), lib_code).map_err(|e| e.to_string())?;

        println!("\x1b[32m✔ Создана библиотека '{}'\x1b[0m", name);
        println!("  ├── vip.json");
        println!("  └── src/lib.venk");
        Ok(())
    }

    /// Аналог `cargo build` / `npm install`. Читает манифест, скачивает пакеты в кэш.
    pub fn install_dependencies() -> Result<(), String> {
        let manifest_path = "vip.json";
        if !Path::new(manifest_path).exists() {
            return Err("Файл vip.json не найден. Запустите из корня проекта.".into());
        }

        let content = fs::read_to_string(manifest_path).map_err(|e| e.to_string())?;
        
        // Простейший парсинг зависимостей (в будущем заменим на реальный JSON-парсер)
        // Ищем строку "dependencies": { ... }
        let mut deps = Vec::new();
        if let Some(deps_start) = content.find("\"dependencies\"") {
            if let Some(brace_start) = content[deps_start..].find('{') {
                if let Some(brace_end) = content[deps_start..].find('}') {
                    let deps_str = &content[deps_start + brace_start + 1..deps_start + brace_end];
                    for pair in deps_str.split(',') {
                        if let Some((k, _v)) = pair.split_once(':') {
                            let pkg_name = k.trim().trim_matches('"');
                            if !pkg_name.is_empty() {
                                deps.push(pkg_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        if deps.is_empty() {
            println!("\x1b[32m✔ Зависимости отсутствуют или уже установлены.\x1b[0m");
            return Ok(());
        }

        // Создаем локальный кэш пакетов (симуляция ~/.vip/cache)
        let cache_dir = Path::new(".vip_cache");
        fs::create_dir_all(cache_dir).map_err(|e| e.to_string())?;

        let mut lock_content = String::from("# vip.lock\n# Автоматически сгенерированный файл\n\n");

        for dep in deps {
            println!("\x1b[33m  Загрузка {}...\x1b[0m", dep);
            
            // Симуляция скачивания из глобального реестра
            let pkg_file = cache_dir.join(format!("{}.venk", dep));
            if !pkg_file.exists() {
                let code = format!("pub fn {}_version() -> string {{\n    return \"1.0.0\"\n}}\n", dep);
                fs::write(&pkg_file, code).map_err(|e| e.to_string())?;
            }

            lock_content.push_str(&format!("[[package]]\nname = \"{}\"\nversion = \"1.0.0\"\nsource = \"registry.venk-lang.org\"\n\n", dep));
        }

        fs::write("vip.lock", lock_content).map_err(|e| e.to_string())?;
        println!("\x1b[32m✔ Установка завершена. Создан vip.lock\x1b[0m");
        Ok(())
    }

    /// Загружает пакет из локального кэша в AST
    pub fn load_package(pkg_name: &str) -> Result<String, String> {
        let pkg_path = format!(".vip_cache/{}.venk", pkg_name);
        if !Path::new(&pkg_path).exists() {
            return Err(format!("Пакет '{}' не найден в кэше. Запустите 'vexc vip install'.", pkg_name));
        }
        fs::read_to_string(pkg_path).map_err(|e| e.to_string())
    }
}