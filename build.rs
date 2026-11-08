use std::fs;
use std::path::Path;
use std::env;

fn main() {
    // 1. Указываем, где искать наши .lib файлы для линковки
    println!("cargo:rustc-link-search=native=lib");
    
    // 2. Указываем линковщику подключить raylib
    println!("cargo:rustc-link-lib=dylib=raylib");

    // 3. Копируем raylib.dll в папку target/debug (или target/release)
    if let Ok(out_dir) = env::var("OUT_DIR") {
        // OUT_DIR выглядит примерно так: target\debug\build\venk-xxxx\out
        // ancestors().nth(3) поднимается на 3 уровня вверх до папки target\debug
        if let Some(target_dir) = Path::new(&out_dir).ancestors().nth(3) {
            let dll_dest = target_dir.join("raylib.dll");
            
            if Path::new("raylib.dll").exists() {
                if let Err(e) = fs::copy("raylib.dll", &dll_dest) {
                    println!("cargo:warning=Не удалось скопировать raylib.dll: {}", e);
                }
            } else {
                println!("cargo:warning=Файл raylib.dll не найден в корне проекта!");
            }
        }
    }

    // 4. Подключаем иконку (ресурсы Windows)
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        // Если у тебя есть иконка, раскомментируй строчку ниже:
        // res.set_icon("icon.ico"); 
        
        if let Err(e) = res.compile() {
            println!("cargo:warning=Не удалось скомпилировать ресурсы: {}", e);
        }
    }
}