use crate::gfx::Gfx;
use crate::lexer;
use crate::parser;
use crate::interp;
use std::fs;
use std::env;

pub fn run_ide() {
    Gfx::init_window(1024, 768, "Vekn IDE (Rust)");
    
    // Используем Arial для 100% поддержки кириллицы
    let font = Gfx::load_system_font("arial", 18);
    let font_ui = Gfx::load_system_font("arial", 20);

    // Начальный код
    let mut code = String::from(
        "// Добро пожаловать в Vekn IDE!\n\
         // Напиши код и нажми кнопку RUN.\n\n\
         fn main() {\n\
             print(\"Hello, Vekn!\")\n\
         }"
    );

    // Переменные консоли
    let mut command_input = String::from("");
    let mut console_history: Vec<String> = vec!["Vekn Terminal v0.1".to_string()];
    
    // Фокус: true = редактор кода, false = консоль
    let mut is_code_focused = true; 
    let mut cursor_timer = 0.0;

    while !Gfx::window_should_close() {
        let dt = Gfx::time_delta();
        cursor_timer += dt;
        let show_cursor = (cursor_timer * 2.0) as i32 % 2 == 0;

        // --- ОПРЕДЕЛЕНИЕ КЛИКА (ФОКУС) ---
        if Gfx::is_mouse_button_pressed() {
            let (mx, my) = Gfx::get_mouse_position();
            // Клик по редактору кода
            if mx >= 20.0 && mx <= 720.0 && my >= 80.0 && my <= 530.0 {
                is_code_focused = true;
            }
            // Клик по консоли
            else if mx >= 20.0 && mx <= 720.0 && my >= 550.0 && my <= 740.0 {
                is_code_focused = false;
            }
        }

        // --- БУФЕР ОБМЕНА (CTRL+C / CTRL+V) ---
        let ctrl_pressed = Gfx::is_key_down("CTRL");
        
        if ctrl_pressed && Gfx::is_key_pressed("V") {
            let clipboard = Gfx::get_clipboard();
            if is_code_focused {
                code.push_str(&clipboard);
            } else {
                command_input.push_str(&clipboard);
            }
        }

        if ctrl_pressed && Gfx::is_key_pressed("C") {
            if is_code_focused {
                Gfx::set_clipboard(&code);
            } else {
                Gfx::set_clipboard(&command_input);
            }
        }

        // --- ОБЫЧНЫЙ ВВОД С КЛАВИАТУРЫ ---
        let ch = Gfx::get_char_pressed();
        if !ch.is_empty() && !ctrl_pressed {
            if is_code_focused { code.push_str(&ch); } else { command_input.push_str(&ch); }
        }

        if Gfx::is_key_pressed("BACKSPACE") {
            if is_code_focused { code.pop(); } else { command_input.pop(); }
        }

        // Обработка ENTER
        if Gfx::is_key_pressed("ENTER") {
            if is_code_focused {
                code.push('\n');
            } else {
                // --- ВЫПОЛНЕНИЕ КОМАНДЫ ИЗ КОНСОЛИ ---
                let cmd = command_input.trim().to_string();
                if !cmd.is_empty() {
                    console_history.push(format!("> {}", cmd));
                    
                    if cmd == "clear" {
                        console_history.clear();
                        Gfx::clear_ide_output();
                    } else if cmd.starts_with("open ") {
                        let path = cmd["open ".len()..].trim();
                        if env::set_current_dir(path).is_ok() {
                            console_history.push(format!("Директория: {}", path));
                        } else {
                            console_history.push(format!("Ошибка: папка '{}' не найдена", path));
                        }
                    } else if cmd.starts_with("vekn run ") || cmd.starts_with("run ") {
                        let file_path = if cmd.starts_with("vekn run ") { cmd["vekn run ".len()..].trim() } else { cmd["run ".len()..].trim() };
                        
                        console_history.push(format!("Запуск: {}", file_path));
                        match fs::read_to_string(file_path) {
                            Ok(src) => {
                                match lexer::lex(&src) {
                                    Ok(toks) => match parser::parse(toks) {
                                        Ok(prog) => {
                                            let mut i = interp::Interp::new();
                                            if let Err(e) = i.run(&prog) {
                                                console_history.push(format!("Runtime Error: {}", e));
                                            }
                                        }
                                        Err(e) => console_history.push(format!("Syntax Error: {}", e.msg)),
                                    },
                                    Err(e) => console_history.push(format!("Lexer Error: {}", e.msg)),
                                }
                            }
                            Err(_) => console_history.push("Файл не найден!".to_string()),
                        }
                    } else {
                        console_history.push(format!("Неизвестная команда: {}", cmd));
                    }
                    command_input.clear();
                }
            }
        }

        if Gfx::is_key_pressed("TAB") && is_code_focused {
            code.push_str("    ");
        }

        // --- ОТРИСОВКА ---
        Gfx::begin_drawing();
        Gfx::clear_background(30, 30, 35);

        // Шапка
        Gfx::draw_rectangle(0.0, 0.0, 1024.0, 60.0, 20, 20, 25);
        Gfx::draw_text_ex(font_ui, "Vekn IDE", 20.0, 15.0, 24.0, 1.0, 50, 150, 255);

        // Кнопка RUN
        if Gfx::gui_button_font(920.0, 10.0, 90.0, 40.0, "RUN", font_ui, 20.0) {
            Gfx::clear_ide_output(); // Чистим старый вывод перед запуском
            match lexer::lex(&code) {
                Ok(toks) => match parser::parse(toks) {
                    Ok(prog) => {
                        let mut i = interp::Interp::new();
                        if let Err(e) = i.run(&prog) {
                            Gfx::ide_print(&format!("Runtime Error: {}", e));
                        }
                    }
                    Err(e) => Gfx::ide_print(&format!("Syntax Error: {}", e.msg)),
                },
                Err(e) => Gfx::ide_print(&format!("Lexer Error: {}", e.msg)),
            }
        }

        // Редактор кода
        Gfx::draw_rectangle(20.0, 80.0, 700.0, 450.0, 40, 40, 45);
        if is_code_focused { Gfx::draw_rect_lines(20.0, 80.0, 700.0, 450.0, 2.0, 100, 200, 255); } 
        else { Gfx::draw_rect_lines(20.0, 80.0, 700.0, 450.0, 2.0, 60, 60, 70); }

        let lines: Vec<&str> = code.split('\n').collect();
        let mut line_y = 90.0;
        for (i, line) in lines.iter().enumerate() {
            Gfx::draw_text_ex(font, line, 30.0, line_y, 18.0, 1.0, 220, 220, 220);
            if i == lines.len() - 1 && is_code_focused && show_cursor {
                let tw = Gfx::measure_text(line, 18.0);
                Gfx::draw_rectangle(30.0 + tw as f32, line_y, 2.0, 20.0, 255, 255, 255);
            }
            line_y += 22.0;
        }

        // КОНСОЛЬ (Внизу)
        Gfx::draw_rectangle(20.0, 550.0, 700.0, 190.0, 20, 20, 25);
        if !is_code_focused { Gfx::draw_rect_lines(20.0, 550.0, 700.0, 190.0, 2.0, 100, 200, 255); }
        
        // 1. Вывод print() из самого интерпретатора (Зеленый)
        let ide_out = Gfx::get_ide_output();
        let mut out_y = 560.0;
        if !ide_out.is_empty() {
            for line in ide_out.iter().take(3) { 
                Gfx::draw_text_ex(font, line, 30.0, out_y, 16.0, 1.0, 100, 255, 100);
                out_y += 20.0;
            }
        }

        // 2. История команд (Серый)
        let mut hist_y = out_y + 10.0;
        let start_idx = if console_history.len() > 3 { console_history.len() - 3 } else { 0 };
        for i in start_idx..console_history.len() {
            Gfx::draw_text_ex(font, &console_history[i], 30.0, hist_y, 16.0, 1.0, 150, 150, 150); 
            hist_y += 20.0;
        }

        // Ввод в консоль
        Gfx::draw_text_ex(font_ui, ">", 30.0, 700.0, 20.0, 1.0, 255, 255, 255);
        Gfx::draw_text_ex(font_ui, &command_input, 50.0, 700.0, 20.0, 1.0, 255, 255, 255);
        if !is_code_focused && show_cursor {
            let tw = Gfx::measure_text(&command_input, 20.0);
            Gfx::draw_rectangle(50.0 + tw as f32, 700.0, 2.0, 22.0, 255, 255, 255);
        }

        // Правая панель
        Gfx::draw_rectangle(740.0, 80.0, 264.0, 660.0, 35, 35, 40);
        Gfx::draw_text_ex(font_ui, "Справка", 750.0, 90.0, 20.0, 1.0, 255, 255, 255);
        
        let help = "Ctrl+C - копировать\nCtrl+V - вставить\n\n\nКоманды консоли:\nopen <path>\nrun <file>\nclear";
        Gfx::draw_text_ex(font, help, 750.0, 130.0, 18.0, 1.0, 180, 180, 180);

        Gfx::end_drawing();
    }
    Gfx::close_window();
}