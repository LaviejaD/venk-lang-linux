mod ast;
mod lexer;
mod parser;
mod value;
mod interp;
mod builtins;
mod error;
mod import;
mod vip;
mod gfx;
mod ide;
mod pvenk;
mod jvenk;
mod gdvenk;
mod lvenk;
mod cppvenk;
mod vek3d;
mod worldgen;

use error::Diagnostics;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use std::fs;
use std::process;
use std::path::Path;

// Заголовок для упаковки игр (ОБЯЗАТЕЛЬНО ДЛЯ СБОРКИ)
pub const MAGIC_HEADER: &[u8] = b"---VENK_BUNDLED_GAME_v1---";

fn main() {
    let args: Vec<String> = env::args().collect();

    // 1. Если аргументов нет — запускаем REPL (консоль)
    if args.len() < 2 {
        start_repl();
        return;
    }

    // 2. ЗАПУСК IDE
    if args[1] == "ide" {
        ide::run_ide();
        return;
    }

    // 3. Менеджер пакетов VIP
    if args[1] == "vipc" {
        if args.len() < 3 {
            eprintln!("\x1b[31mИспользование: venk vipc <имя_библиотеки>\x1b[0m");
            process::exit(1);
        }
        if let Err(e) = vip::VipManager::create_library(&args[2]) {
            eprintln!("\x1b[31mОшибка: {}\x1b[0m", e);
        }
        return;
    }

    if args[1] == "vip" && args.get(2).map(|s| s.as_str()) == Some("install") {
        if let Err(e) = vip::VipManager::install_dependencies() {
            eprintln!("\x1b[31mОшибка: {}\x1b[0m", e);
        }
        return;
    }

    // 4. Обычный запуск файла
    let path = &args[1];
    run_file(path);
}

fn run_file(path: &str) {
    let src = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("\x1b[1;31mОшибка чтения файла '{}': {}\x1b[0m", path, e);
            return;
        }
    };
    run_source(&src);
}

fn run_source(src: &str) {
    let mut diags = Diagnostics::new();
    let toks = match lexer::lex(src) {
        Ok(t) => t,
        Err(e) => { 
            diags.push_error(e.span, e.msg); 
            diags.print_all(); 
            return; 
        }
    };

    let prog = match parser::parse(toks) {
        Ok(p) => p,
        Err(e) => { 
            diags.push_error(e.span, e.msg); 
            diags.print_all(); 
            return; 
        }
    };

    let mut interpreter = interp::Interp::new();
    if let Err(err_msg) = interpreter.run(&prog) {
        eprintln!("\x1b[1;31mRuntime Error:\x1b[0m {}", err_msg);
    }
}

fn start_repl() {
    println!("\x1b[1;36mVenk 0.1.0 (default, May 12 2024) [Rust]\x1b[0m");
    println!("Type \"help\" for commands, \"exit()\" or \"quit\" to exit.");
    
    print!("\x1b[1;34m");
    println!(r" __     __                   ");
    println!(r" \ \   / / ___  _ __  | | __ ");
    println!(r"  \ \ / / / _ \| '_ \ | |/ / ");
    println!(r"   \ V / |  __/| | | ||   <  ");
    println!(r"    \_/   \___||_| |_||_|\_\ ");
    println!(r"     Scripting Language");
    print!("\x1b[0m\n");

    let mut rl = DefaultEditor::new().unwrap();
    let mut interpreter = interp::Interp::new();
    let mut buffer = String::new();
    let mut linked_scripts: Vec<String> = Vec::new();

    loop {
        let prompt = if buffer.is_empty() { "\x1b[1;32m>>> \x1b[0m" } else { "\x1b[1;32m... \x1b[0m" };
        let readline = rl.readline(prompt);
        
        match readline {
            Ok(line) => {
                let line_trimmed = line.trim();
                if line_trimmed.is_empty() && buffer.is_empty() { continue; }
                
                // --- Обработка команд оболочки ---
                if line_trimmed == "exit()" || line_trimmed == "quit" { break; }
                if line_trimmed == "help" {
                    println!("Venk Commands:");
                    println!("  open <path>             - Открыть папку");
                    println!("  create_script <name>    - Создать новый скрипт (.venk)");
                    println!("  run <name>              - Запустить скрипт");
                    println!("  Script <name>           - Связать скрипт с текущей сессией");
                    println!("  venk run <name>         - Запустить скрипт и все привязки");
                    println!("  ide                     - Запустить графическую IDE");
                    println!("  exit() / quit           - Выйти");
                    println!("  (Все остальное выполняется как код Venk)");
                    continue;
                }
                if line_trimmed == "ide" {
                    ide::run_ide();
                    continue;
                }
                if line_trimmed.starts_with("open ") {
                    let path_str = line_trimmed["open ".len()..].trim();
                    if Path::new(path_str).exists() {
                        if env::set_current_dir(path_str).is_ok() {
                            println!("\x1b[32mДиректория изменена на: {}\x1b[0m", path_str);
                        } else {
                            eprintln!("\x1b[31mНе удалось войти в {}\x1b[0m", path_str);
                        }
                    } else {
                        eprintln!("\x1b[31mПапка не найдена: {}\x1b[0m", path_str);
                    }
                    continue;
                }
                if line_trimmed.starts_with("create_script ") {
                    let name = line_trimmed["create_script ".len()..].trim();
                    if let Ok(_) = fs::File::create(name) {
                        println!("\x1b[32mСоздан файл: {}\x1b[0m", name);
                    } else {
                        eprintln!("\x1b[31mОшибка создания файла {}\x1b[0m", name);
                    }
                    continue;
                }
                if line_trimmed.starts_with("Script ") {
                    let name = line_trimmed["Script ".len()..].trim();
                    linked_scripts.push(name.to_string());
                    println!("\x1b[32mСкрипт '{}' привязан к текущей сессии.\x1b[0m", name);
                    continue;
                }
                if line_trimmed.starts_with("run ") {
                    let name = line_trimmed["run ".len()..].trim();
                    run_file(name);
                    continue;
                }
                if line_trimmed.starts_with("venk run ") {
                    let name = line_trimmed["venk run ".len()..].trim();
                    run_file(name);
                    for link in &linked_scripts {
                        run_file(link);
                    }
                    continue;
                }

                let _ = rl.add_history_entry(line_trimmed);
                buffer.push_str(&line);
                buffer.push('\n');

                match lexer::lex(&buffer) {
                    Ok(toks) => {
                        let mut p = parser::Parser::new(toks);
                        match p.parse_repl_line() {
                            Ok(parser::ReplInput::Item(item)) => {
                                if let Err(e) = interpreter.register_repl(&item) {
                                    eprintln!("\x1b[1;31mError: {:?}\x1b[0m", e);
                                }
                                buffer.clear();
                            }
                            Ok(parser::ReplInput::Stmt(stmt)) => {
                                if let Err(e) = interpreter.exec_repl_stmt(&stmt) {
                                    eprintln!("\x1b[1;31mError: {:?}\x1b[0m", e);
                                }
                                buffer.clear();
                            }
                            Ok(parser::ReplInput::Expr(expr)) => {
                                match interpreter.eval_repl_expr(&expr) {
                                    Ok(val) => {
                                        if !matches!(val, value::Value::Null) {
                                            println!("\x1b[36m{}\x1b[0m", val.to_display());
                                        }
                                        buffer.clear();
                                    }
                                    Err(e) => {
                                        eprintln!("\x1b[1;31mError: {:?}\x1b[0m", e);
                                        buffer.clear();
                                    }
                                }
                            }
                            Err(e) => {
                                if e.msg.contains("Eof") || e.msg.contains("EOF") || e.msg.contains("ожидалось") {
                                    // Ждем дальше (незакрытые скобки и т.д.)
                                } else {
                                    eprintln!("\x1b[1;31mSyntax Error: {}\x1b[0m", e);
                                    buffer.clear();
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31mLexer Error: {}\x1b[0m", e);
                        buffer.clear();
                    }
                }
            }
            Err(ReadlineError::Interrupted) => { println!("Ctrl-C"); buffer.clear(); }
            Err(ReadlineError::Eof) => { println!("Ctrl-D"); break; }
            Err(err) => { println!("Error: {:?}", err); break; }
        }
    }
}