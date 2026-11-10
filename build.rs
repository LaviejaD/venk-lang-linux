use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // [English] Get the target OS. This is safer than using `cfg!` for cross-compilation.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // 1. Указываем, где искать наши .lib файлы для линковки
    // [English] Tell the linker to look inside the specific OS folder.
    if target_os == "windows" {
        println!("cargo:rustc-link-search=native=libs/windows");
    } else if target_os == "linux" {
        println!("cargo:rustc-link-search=native=libs/linux");
    }

    // 2. Указываем линковщику подключить raylib
    println!("cargo:rustc-link-lib=dylib=raylib");

    // [English] Linux specific: Tell the executable to look for .so files in its own directory at runtime.
    if target_os == "linux" {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");

        // [English] Fallback: If by any chance Linux links statically (libraylib.a),
        // these dependencies prevent the X11 undefined symbol errors.
        println!("cargo:rustc-link-lib=X11");
        println!("cargo:rustc-link-lib=GL");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
        println!("cargo:rustc-link-lib=rt");
    }

    // 3. Копируем raylib.dll в папку target/debug (или target/release)
    if let Ok(out_dir) = env::var("OUT_DIR") {
        if let Some(target_dir) = Path::new(&out_dir).ancestors().nth(3) {
            // [English] Windows: Copy the DLL from libs/windows/ to the output directory.
            if target_os == "windows" {
                let dll_path = Path::new("libs/windows/raylib.dll");
                let dll_dest = target_dir.join("raylib.dll");

                if dll_path.exists() {
                    if let Err(e) = fs::copy(dll_path, &dll_dest) {
                        println!("cargo:warning=Не удалось скопировать raylib.dll: {}", e);
                    }
                } else {
                    println!("cargo:warning=Файл raylib.dll не найден в libs/windows/!");
                }
            }

            // [English] Linux: Copy ALL files starting with "libraylib.so" (including versioned ones)
            // from libs/linux/ to the output directory.
            if target_os == "linux" {
                if let Ok(entries) = fs::read_dir("libs/linux") {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            if filename.starts_with("libraylib.so") {
                                let dest = target_dir.join(filename);
                                if let Err(e) = fs::copy(&path, &dest) {
                                    println!("cargo:warning=Failed to copy {}: {}", filename, e);
                                }
                            }
                        }
                    }
                } else {
                    println!("cargo:warning=Could not read libs/linux/ directory!");
                }
            }
        }
    }

    // 4. Подключаем иконку (ресурсы Windows)
    // [English] Only compile Windows resources if the target OS is Windows.
    if target_os == "windows" {
        let mut res = winres::WindowsResource::new();
        // Если у тебя есть иконка, раскомментируй строчку ниже:
        res.set_icon("icon.ico");

        if let Err(e) = res.compile() {
            println!("cargo:warning=Не удалось скомпилировать ресурсы: {}", e);
        }
    }
}
