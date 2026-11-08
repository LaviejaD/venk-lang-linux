use raylib::ffi;
use std::ffi::CString;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicI32, Ordering};
use std::cell::RefCell;

// Глобальный буфер для вывода в IDE
static IDE_OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());

// Текстуры можно шарить между потоками
static TEXTURES: LazyLock<Mutex<HashMap<i32, ffi::Texture2D>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

// Звуки, музыка и шрифты привязаны к главному потоку (thread_local)
thread_local! {
    static SOUNDS: RefCell<HashMap<i32, ffi::Sound>> = RefCell::new(HashMap::new());
    static MUSICS: RefCell<HashMap<i32, ffi::Music>> = RefCell::new(HashMap::new());
    static FONTS: RefCell<HashMap<i32, ffi::Font>> = RefCell::new(HashMap::new());
}

static NEXT_ID: AtomicI32 = AtomicI32::new(1);
static TIME_SCALE: Mutex<f32> = Mutex::new(1.0);
static mut CAMERA_SHAKE: f32 = 0.0;

pub struct Gfx;

impl Gfx {
    // ==========================================
    // 1. ОКНО И ВРЕМЯ (Оригинальные + Новые)
    // ==========================================
    pub fn init_window(w: i32, h: i32, title: &str) {
        let c_title = CString::new(title).unwrap();
        unsafe {
            ffi::InitWindow(w, h, c_title.as_ptr());
            ffi::InitAudioDevice();
            ffi::SetTargetFPS(60);
        }
    }
    pub fn window_should_close() -> bool { unsafe { ffi::WindowShouldClose() } }
    pub fn close_window() { unsafe { ffi::CloseAudioDevice(); ffi::CloseWindow(); } }
    pub fn begin_drawing() { unsafe { ffi::BeginDrawing() } }
    pub fn end_drawing() { unsafe { ffi::EndDrawing() } }
    pub fn clear_background(r: u8, g: u8, b: u8) { unsafe { ffi::ClearBackground(ffi::Color { r, g, b, a: 255 }); } }
    pub fn clear_background_alpha(r: u8, g: u8, b: u8, a: u8) { unsafe { ffi::ClearBackground(ffi::Color { r, g, b, a }); } }
    
    pub fn toggle_fullscreen() { unsafe { ffi::ToggleFullscreen(); } }
    pub fn set_window_title(title: &str) {
        let c_title = CString::new(title).unwrap();
        unsafe { ffi::SetWindowTitle(c_title.as_ptr()); }
    }
    pub fn set_window_icon(path: &str) {
        let c_path = CString::new(path).unwrap();
        unsafe { 
            let img = ffi::LoadImage(c_path.as_ptr()); 
            ffi::SetWindowIcon(img); 
            ffi::UnloadImage(img); 
        }
    }
    pub fn set_window_state_resizable() { unsafe { ffi::SetWindowState(ffi::ConfigFlags::FLAG_WINDOW_RESIZABLE as u32); } }
    pub fn set_vsync(enabled: bool) { 
        unsafe { 
            if enabled { ffi::SetWindowState(ffi::ConfigFlags::FLAG_VSYNC_HINT as u32); } 
            else { ffi::ClearWindowState(ffi::ConfigFlags::FLAG_VSYNC_HINT as u32); } 
        } 
    }
    pub fn get_fps() -> i32 { unsafe { ffi::GetFPS() } }
    pub fn get_frame_time() -> f32 { unsafe { ffi::GetFrameTime() } }
    pub fn screen_width() -> i32 { unsafe { ffi::GetScreenWidth() } }
    pub fn screen_height() -> i32 { unsafe { ffi::GetScreenHeight() } }
    pub fn set_fps(fps: i32) { unsafe { ffi::SetTargetFPS(fps); } }
    
    pub fn time_delta() -> f32 {
        let real_delta = unsafe { ffi::GetFrameTime() };
        let scale = *TIME_SCALE.lock().unwrap();
        real_delta * scale
    }
    pub fn set_time(speed: f32) { *TIME_SCALE.lock().unwrap() = speed.max(0.0); }
    pub fn get_time_speed() -> f32 { *TIME_SCALE.lock().unwrap() }
    pub fn get_time() -> f64 { unsafe { ffi::GetTime() } }

    // ==========================================
    // 2. МАТЕМАТИКА И ВЕКТОРЫ 2D (Новое)
    // ==========================================
    pub fn vec2_add(x1: f32, y1: f32, x2: f32, y2: f32) -> (f32, f32) { (x1 + x2, y1 + y2) }
    pub fn vec2_sub(x1: f32, y1: f32, x2: f32, y2: f32) -> (f32, f32) { (x1 - x2, y1 - y2) }
    pub fn vec2_mul(x: f32, y: f32, scalar: f32) -> (f32, f32) { (x * scalar, y * scalar) }
    pub fn vec2_div(x: f32, y: f32, scalar: f32) -> (f32, f32) { if scalar == 0.0 { (0.0, 0.0) } else { (x / scalar, y / scalar) } }
    pub fn vec2_length(x: f32, y: f32) -> f32 { (x * x + y * y).sqrt() }
    pub fn vec2_normalize(x: f32, y: f32) -> (f32, f32) {
        let len = (x * x + y * y).sqrt();
        if len == 0.0 { (0.0, 0.0) } else { (x / len, y / len) }
    }
    pub fn vec2_dot(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 { x1 * x2 + y1 * y2 }
    pub fn vec2_distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let dx = x2 - x1; let dy = y2 - y1; (dx * dx + dy * dy).sqrt()
    }
    pub fn vec2_lerp(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> (f32, f32) {
        (x1 + (x2 - x1) * t, y1 + (y2 - y1) * t)
    }
    pub fn vec2_rotate(x: f32, y: f32, angle_degrees: f32) -> (f32, f32) {
        let rad = angle_degrees.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();
        (x * cos_a - y * sin_a, x * sin_a + y * cos_a)
    }
    pub fn angle_from_vec(x: f32, y: f32) -> f32 { y.atan2(x).to_degrees() }
    pub fn vec_from_angle(angle_degrees: f32) -> (f32, f32) {
        let rad = angle_degrees.to_radians();
        (rad.cos(), rad.sin())
    }

    // ==========================================
    // 3. СПРАЙТЫ И ТЕКСТУРЫ (Расширено)
    // ==========================================
    pub fn load_texture(path: &str) -> i32 {
        let c_path = CString::new(path).unwrap();
        unsafe {
            let tex = ffi::LoadTexture(c_path.as_ptr());
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            TEXTURES.lock().unwrap().insert(id, tex);
            id
        }
    }
    pub fn unload_texture(id: i32) {
        let mut texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.remove(&id) { unsafe { ffi::UnloadTexture(tex); } }
    }
    pub fn get_texture_width(id: i32) -> i32 {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) { tex.width } else { 0 }
    }
    pub fn get_texture_height(id: i32) -> i32 {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) { tex.height } else { 0 }
    }
    
    // Оригинальная функция
    pub fn draw_sprite_ex(id: i32, x: f32, y: f32, rot: f32, scale_x: f32, scale_y: f32, flip_x: bool, flip_y: bool) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let source = ffi::Rectangle { x: 0.0, y: 0.0, width: tex.width as f32, height: tex.height as f32 };
                let dest = ffi::Rectangle { x, y, width: tex.width as f32 * scale_x, height: tex.height as f32 * scale_y };
                let origin = ffi::Vector2 { x: dest.width / 2.0, y: dest.height / 2.0 };
                let mut final_rot = rot;
                if flip_x { final_rot = -final_rot; }
                ffi::DrawTexturePro(*tex, source, dest, origin, final_rot, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            }
        }
    }
    
    // Оригинальная функция
    pub fn draw_animation_frame(id: i32, x: f32, y: f32, frame_x: f32, frame_y: f32, frame_w: f32, frame_h: f32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let source = ffi::Rectangle { x: frame_x, y: frame_y, width: frame_w, height: frame_h };
                let pos = ffi::Vector2 { x, y };
                ffi::DrawTextureRec(*tex, source, pos, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            }
        }
    }

    // Новые функции
    pub fn draw_texture_pro(id: i32, src_x: f32, src_y: f32, src_w: f32, src_h: f32, dest_x: f32, dest_y: f32, scale_x: f32, scale_y: f32, rot: f32, flip_x: bool, flip_y: bool) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let source = ffi::Rectangle { x: src_x, y: src_y, width: if flip_x { -src_w } else { src_w }, height: if flip_y { -src_h } else { src_h } };
                let dest = ffi::Rectangle { x: dest_x, y: dest_y, width: src_w * scale_x, height: src_h * scale_y };
                let origin = ffi::Vector2 { x: dest.width / 2.0, y: dest.height / 2.0 };
                ffi::DrawTexturePro(*tex, source, dest, origin, rot, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            }
        }
    }

    pub fn draw_texture_tiled(id: i32, x: f32, y: f32, w: f32, h: f32, scale: f32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let source = ffi::Rectangle { x: 0.0, y: 0.0, width: tex.width as f32, height: tex.height as f32 };
                let step_x = tex.width as f32 * scale;
                let step_y = tex.height as f32 * scale;
                
                let mut dy = 0.0;
                while dy < h {
                    let mut dx = 0.0;
                    while dx < w {
                        let pos = ffi::Vector2 { x: x + dx, y: y + dy };
                        // Используем DrawTextureRec для рисования каждого кусочка
                        ffi::DrawTextureRec(*tex, source, pos, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
                        dx += step_x;
                    }
                    dy += step_y;
                }
            }
        }
    }

    // ==========================================
    // 4. ШРИФТЫ И ТЕКСТ (Расширено)
    // ==========================================
    pub fn load_font(path: &str, size: i32) -> i32 {
        let c_path = CString::new(path).unwrap();
        unsafe {
            let mut codepoints = Vec::new();
            for i in 32..=126 { codepoints.push(i); }
            for i in 1024..=1279 { codepoints.push(i); }
            let font = ffi::LoadFontEx(c_path.as_ptr(), size, codepoints.as_mut_ptr(), codepoints.len() as i32);
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            FONTS.with(|f| { f.borrow_mut().insert(id, font); });
            id
        }
    }
    pub fn load_system_font(font_name: &str, size: i32) -> i32 {
        let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let fonts_dir = format!("{}\\Fonts", win_dir);
        let mut path_to_load = None;
        for ext in &["ttf", "otf"] {
            let path = format!("{}\\{}.{}", fonts_dir, font_name, ext);
            if std::path::Path::new(&path).exists() { path_to_load = Some(path); break; }
        }
        if let Some(path) = path_to_load {
            let c_path = CString::new(path).unwrap();
            unsafe {
                let mut codepoints = Vec::new();
                for i in 32..=126 { codepoints.push(i); }
                for i in 1024..=1279 { codepoints.push(i); }
                let font = ffi::LoadFontEx(c_path.as_ptr(), size, codepoints.as_mut_ptr(), codepoints.len() as i32);
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                FONTS.with(|f| { f.borrow_mut().insert(id, font); });
                id
            }
        } else {
            unsafe {
                let font = ffi::GetFontDefault();
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                FONTS.with(|f| { f.borrow_mut().insert(id, font); });
                id
            }
        }
    }
    pub fn draw_text_ex(font_id: i32, text: &str, x: f32, y: f32, size: f32, spacing: f32, r: u8, g: u8, b: u8) {
        FONTS.with(|f| {
            let fonts = f.borrow();
            let font = if let Some(fnt) = fonts.get(&font_id) { *fnt } else { unsafe { ffi::GetFontDefault() } };
            let c_text = CString::new(text).unwrap();
            unsafe {
                ffi::DrawTextEx(font, c_text.as_ptr(), ffi::Vector2 { x, y }, size, spacing, ffi::Color { r, g, b, a: 255 });
            }
        });
    }
    
    // Новые функции
    pub fn draw_text_centered(font_id: i32, text: &str, y: f32, size: f32, r: u8, g: u8, b: u8) {
        let screen_w = unsafe { ffi::GetScreenWidth() } as f32;
        FONTS.with(|f| {
            let fonts = f.borrow();
            let font = if let Some(fnt) = fonts.get(&font_id) { *fnt } else { unsafe { ffi::GetFontDefault() } };
            let c_text = CString::new(text).unwrap();
            unsafe {
                let vec = ffi::MeasureTextEx(font, c_text.as_ptr(), size, 1.0);
                ffi::DrawTextEx(font, c_text.as_ptr(), ffi::Vector2 { x: (screen_w - vec.x) / 2.0, y }, size, 1.0, ffi::Color { r, g, b, a: 255 });
            }
        });
    }

    pub fn draw_text_shadow(font_id: i32, text: &str, x: f32, y: f32, size: f32, r: u8, g: u8, b: u8) {
        Self::draw_text_ex(font_id, text, x + 2.0, y + 2.0, size, 1.0, 0, 0, 0);
        Self::draw_text_ex(font_id, text, x, y, size, 1.0, r, g, b);
    }

    pub fn draw_text_wrapped(font_id: i32, text: &str, x: f32, y: f32, width: f32, size: f32, r: u8, g: u8, b: u8) {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut line = String::new();
        let mut curr_y = y;
        FONTS.with(|f| {
            let fonts = f.borrow();
            let font = if let Some(fnt) = fonts.get(&font_id) { *fnt } else { unsafe { ffi::GetFontDefault() } };
            for word in words {
                let test_line = if line.is_empty() { word.to_string() } else { format!("{} {}", line, word) };
                let c_test = CString::new(test_line.as_str()).unwrap();
                unsafe {
                    let text_size = ffi::MeasureTextEx(font, c_test.as_ptr(), size, 1.0);
                    if text_size.x > width {
                        let c_line = CString::new(line.as_str()).unwrap();
                        ffi::DrawTextEx(font, c_line.as_ptr(), ffi::Vector2 { x, y: curr_y }, size, 1.0, ffi::Color { r, g, b, a: 255 });
                        curr_y += size + 4.0;
                        line = word.to_string();
                    } else {
                        line = test_line;
                    }
                }
            }
            if !line.is_empty() {
                let c_line = CString::new(line.as_str()).unwrap();
                unsafe { ffi::DrawTextEx(font, c_line.as_ptr(), ffi::Vector2 { x, y: curr_y }, size, 1.0, ffi::Color { r, g, b, a: 255 }); }
            }
        });
    }

    pub fn measure_text(text: &str, size: f32) -> i32 {
        let c_text = CString::new(text).unwrap();
        unsafe { ffi::MeasureText(c_text.as_ptr(), size as i32) }
    }
    
    pub fn measure_text_ex(font_id: i32, text: &str, size: f32) -> (f32, f32) {
        FONTS.with(|f| {
            let fonts = f.borrow();
            let font = if let Some(fnt) = fonts.get(&font_id) { *fnt } else { unsafe { ffi::GetFontDefault() } };
            let c_text = CString::new(text).unwrap();
            unsafe {
                let vec = ffi::MeasureTextEx(font, c_text.as_ptr(), size, 1.0);
                (vec.x, vec.y)
            }
        })
    }

    // ==========================================
    // 5. ПРИМИТИВЫ И ПРОДВИНУТАЯ 2D ГРАФИКА (Расширено)
    // ==========================================
    pub fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, thick: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawLineEx(ffi::Vector2 { x: x1, y: y1 }, ffi::Vector2 { x: x2, y: y2 }, thick, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_rectangle(x: f32, y: f32, w: f32, h: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawRectangle(x as i32, y as i32, w as i32, h as i32, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_circle(x: f32, y: f32, radius: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawCircle(x as i32, y as i32, radius, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_text(text: &str, x: f32, y: f32, size: f32, r: u8, g: u8, b: u8) {
        let c_text = CString::new(text).unwrap();
        unsafe { ffi::DrawText(c_text.as_ptr(), x as i32, y as i32, size as i32, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_rect_lines(x: f32, y: f32, w: f32, h: f32, thick: f32, r: u8, g: u8, b: u8) {
        unsafe {
            let rec = ffi::Rectangle { x, y, width: w, height: h };
            ffi::DrawRectangleLinesEx(rec, thick, ffi::Color { r, g, b, a: 255 });
        }
    }
    pub fn draw_circle_lines(x: f32, y: f32, radius: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawCircleLines(x as i32, y as i32, radius, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_poly(cx: f32, cy: f32, sides: i32, radius: f32, rot: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawPoly(ffi::Vector2 { x: cx, y: cy }, sides, radius, rot, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_ellipse(cx: f32, cy: f32, rh: f32, rv: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawEllipse(cx as i32, cy as i32, rh, rv, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_circle_sector(cx: f32, cy: f32, rad: f32, start_angle: f32, end_angle: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawCircleSector(ffi::Vector2 { x: cx, y: cy }, rad, start_angle, end_angle, 36, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_ring(cx: f32, cy: f32, inner: f32, outer: f32, start_angle: f32, end_angle: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawRing(ffi::Vector2 { x: cx, y: cy }, inner, outer, start_angle, end_angle, 36, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_triangle(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, r: u8, g: u8, b: u8) {
        unsafe { 
            ffi::DrawTriangle(
                ffi::Vector2 { x: x1, y: y1 }, 
                ffi::Vector2 { x: x2, y: y2 }, 
                ffi::Vector2 { x: x3, y: y3 }, 
                ffi::Color { r, g, b, a: 255 }
            ); 
        }
    }

    // Новые функции
    pub fn draw_rect_alpha(x: f32, y: f32, w: f32, h: f32, r: u8, g: u8, b: u8, a: u8) {
        unsafe { ffi::DrawRectangle(x as i32, y as i32, w as i32, h as i32, ffi::Color { r, g, b, a }); }
    }
    pub fn draw_rect_rounded(x: f32, y: f32, w: f32, h: f32, radius: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawRectangleRounded(ffi::Rectangle { x, y, width: w, height: h }, radius, 10, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_rect_gradient_v(x: f32, y: f32, w: f32, h: f32, r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) {
        unsafe { ffi::DrawRectangleGradientV(x as i32, y as i32, w as i32, h as i32, ffi::Color { r: r1, g: g1, b: b1, a: 255 }, ffi::Color { r: r2, g: g2, b: b2, a: 255 }); }
    }
    pub fn draw_rect_gradient_h(x: f32, y: f32, w: f32, h: f32, r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) {
        unsafe { ffi::DrawRectangleGradientH(x as i32, y as i32, w as i32, h as i32, ffi::Color { r: r1, g: g1, b: b1, a: 255 }, ffi::Color { r: r2, g: g2, b: b2, a: 255 }); }
    }
    pub fn draw_triangle_lines(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawTriangleLines(ffi::Vector2 { x: x1, y: y1 }, ffi::Vector2 { x: x2, y: y2 }, ffi::Vector2 { x: x3, y: y3 }, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_pixel(x: f32, y: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawPixel(x as i32, y as i32, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_health_bar(x: f32, y: f32, w: f32, h: f32, current: f32, max: f32, r: u8, g: u8, b: u8) {
        let ratio = (current / max).clamp(0.0, 1.0);
        Self::draw_rectangle(x, y, w, h, 40, 40, 40); // Фон
        Self::draw_rectangle(x, y, w * ratio, h, r, g, b); // Заполнение
        Self::draw_rect_lines(x, y, w, h, 1.0, 0, 0, 0); // Рамка
    }
    
    // ==========================================
    // 6. UI ЭЛЕМЕНТЫ
    // ==========================================
    pub fn gui_button(x: f32, y: f32, w: f32, h: f32, text: &str) -> bool {
        let mx = unsafe { ffi::GetMouseX() } as f32;
        let my = unsafe { ffi::GetMouseY() } as f32;
        let hover = mx >= x && mx <= x + w && my >= y && my <= y + h;
        
        let bg = if hover { ffi::Color { r: 100, g: 100, b: 120, a: 255 } } else { ffi::Color { r: 60, g: 60, b: 80, a: 255 } };
        unsafe {
            ffi::DrawRectangle(x as i32, y as i32, w as i32, h as i32, bg);
            ffi::DrawRectangleLines(x as i32, y as i32, w as i32, h as i32, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            let c_text = CString::new(text).unwrap();
            let tw = ffi::MeasureText(c_text.as_ptr(), 20);
            ffi::DrawText(c_text.as_ptr(), (x + w/2.0) as i32 - tw/2, (y + h/2.0) as i32 - 10, 20, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
        }
        hover && unsafe { ffi::IsMouseButtonPressed(ffi::MouseButton::MOUSE_BUTTON_LEFT as i32) }
    }

    pub fn gui_button_font(x: f32, y: f32, w: f32, h: f32, text: &str, font_id: i32, font_size: f32) -> bool {
        let mx = unsafe { ffi::GetMouseX() } as f32;
        let my = unsafe { ffi::GetMouseY() } as f32;
        let hover = mx >= x && mx <= x + w && my >= y && my <= y + h;
        
        let bg = if hover { ffi::Color { r: 100, g: 100, b: 120, a: 255 } } else { ffi::Color { r: 60, g: 60, b: 80, a: 255 } };
        unsafe {
            ffi::DrawRectangle(x as i32, y as i32, w as i32, h as i32, bg);
            ffi::DrawRectangleLines(x as i32, y as i32, w as i32, h as i32, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            
            let c_text = CString::new(text).unwrap();
            let tw = ffi::MeasureText(c_text.as_ptr(), font_size as i32);
            Self::draw_text_ex(font_id, text, x + (w - tw as f32) / 2.0, y + (h - font_size) / 2.0, font_size, 1.0, 255, 255, 255);
        }
        hover && unsafe { ffi::IsMouseButtonPressed(ffi::MouseButton::MOUSE_BUTTON_LEFT as i32) }
    }

    pub fn gui_slider(x: f32, y: f32, w: f32, h: f32, value: f32) -> f32 {
        unsafe {
            ffi::DrawRectangle(x as i32, y as i32, w as i32, h as i32, ffi::Color { r: 50, g: 50, b: 50, a: 255 });
            ffi::DrawRectangle(x as i32, y as i32, (w * value) as i32, h as i32, ffi::Color { r: 50, g: 200, b: 50, a: 255 });
        }
        value
    }

    // ==========================================
    // 7. АУДИО (Звуки и Музыка)
    // ==========================================
    pub fn load_sound(path: &str) -> i32 {
        let c_path = CString::new(path).unwrap();
        unsafe {
            let snd = ffi::LoadSound(c_path.as_ptr());
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            SOUNDS.with(|s| { s.borrow_mut().insert(id, snd); });
            id
        }
    }
    pub fn play_sound(id: i32) {
        SOUNDS.with(|s| { if let Some(snd) = s.borrow().get(&id) { unsafe { ffi::PlaySound(*snd); } } });
    }
    pub fn stop_sound(id: i32) {
        SOUNDS.with(|s| { if let Some(snd) = s.borrow().get(&id) { unsafe { ffi::StopSound(*snd); } } });
    }
    pub fn unload_sound(id: i32) {
        SOUNDS.with(|s| { if let Some(snd) = s.borrow_mut().remove(&id) { unsafe { ffi::UnloadSound(snd); } } });
    }

    pub fn load_music(path: &str) -> i32 {
        let c_path = CString::new(path).unwrap();
        unsafe {
            let mus = ffi::LoadMusicStream(c_path.as_ptr());
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            MUSICS.with(|m| { m.borrow_mut().insert(id, mus); });
            id
        }
    }
    pub fn play_music(id: i32) {
        MUSICS.with(|m| { if let Some(mus) = m.borrow().get(&id) { unsafe { ffi::PlayMusicStream(*mus); } } });
    }
    pub fn update_music(id: i32) {
        MUSICS.with(|m| { if let Some(mus) = m.borrow().get(&id) { unsafe { ffi::UpdateMusicStream(*mus); } } });
    }
    pub fn stop_music(id: i32) {
        MUSICS.with(|m| { if let Some(mus) = m.borrow().get(&id) { unsafe { ffi::StopMusicStream(*mus); } } });
    }

    // ==========================================
    // 8. КОЛЛИЗИИ И МАТЕМАТИКА (Оригинальные + Новые)
    // ==========================================
    pub fn collision_rect(x1: f32, y1: f32, w1: f32, h1: f32, x2: f32, y2: f32, w2: f32, h2: f32) -> bool {
        unsafe {
            ffi::CheckCollisionRecs(
                ffi::Rectangle { x: x1, y: y1, width: w1, height: h1 },
                ffi::Rectangle { x: x2, y: y2, width: w2, height: h2 }
            )
        }
    }
    pub fn collision_circle(x1: f32, y1: f32, r1: f32, x2: f32, y2: f32, r2: f32) -> bool {
        unsafe { ffi::CheckCollisionCircles(ffi::Vector2 { x: x1, y: y1 }, r1, ffi::Vector2 { x: x2, y: y2 }, r2) }
    }
    pub fn collision_point_circle(px: f32, py: f32, cx: f32, cy: f32, cr: f32) -> bool {
        unsafe { ffi::CheckCollisionPointCircle(ffi::Vector2 { x: px, y: py }, ffi::Vector2 { x: cx, y: cy }, cr) }
    }
    pub fn collision_circle_rect(cx: f32, cy: f32, rad: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
        unsafe { ffi::CheckCollisionCircleRec(ffi::Vector2 { x: cx, y: cy }, rad, ffi::Rectangle { x: rx, y: ry, width: rw, height: rh }) }
    }
    pub fn get_distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let dx = x2 - x1; let dy = y2 - y1; (dx * dx + dy * dy).sqrt()
    }
    pub fn get_angle(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
        let dx = x2 - x1; let dy = y2 - y1; dy.atan2(dx) * 180.0 / std::f32::consts::PI
    }
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
    pub fn apply_gravity(y: f32, vy: f32, gravity: f32, dt: f32) -> (f32, f32) {
        let new_vy = vy + gravity * dt;
        let new_y = y + new_vy * dt;
        (new_y, new_vy)
    }

    // ==========================================
    // 9. КАМЕРА
    // ==========================================
    pub fn begin_camera(x: f32, y: f32, zoom: f32) {
        unsafe {
            let screen_w = ffi::GetScreenWidth() as f32;
            let screen_h = ffi::GetScreenHeight() as f32;
            let mut shake_x = 0.0;
            let mut shake_y = 0.0;
            if CAMERA_SHAKE > 0.0 {
                shake_x = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as f32 % 100.0 - 50.0) * CAMERA_SHAKE;
                shake_y = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as f32 % 100.0 - 50.0) * CAMERA_SHAKE;
                CAMERA_SHAKE -= 0.05;
                if CAMERA_SHAKE < 0.0 { CAMERA_SHAKE = 0.0; }
            }
            let camera = ffi::Camera2D {
                offset: ffi::Vector2 { x: screen_w / 2.0 + shake_x, y: screen_h / 2.0 + shake_y },
                target: ffi::Vector2 { x, y },
                rotation: 0.0,
                zoom,
            };
            ffi::BeginMode2D(camera);
        }
    }
    pub fn end_camera() { unsafe { ffi::EndMode2D(); } }
    pub fn shake_camera(intensity: f32) { unsafe { CAMERA_SHAKE = intensity; } }
    
    pub fn camera_screen_to_world(mx: f32, my: f32, cam_x: f32, cam_y: f32, zoom: f32) -> (f32, f32) {
        let screen_w = unsafe { ffi::GetScreenWidth() } as f32;
        let screen_h = unsafe { ffi::GetScreenHeight() } as f32;
        let wx = (mx - screen_w / 2.0) / zoom + cam_x;
        let wy = (my - screen_h / 2.0) / zoom + cam_y;
        (wx, wy)
    }
    pub fn camera_world_to_screen(wx: f32, wy: f32, cam_x: f32, cam_y: f32, zoom: f32) -> (f32, f32) {
        let screen_w = unsafe { ffi::GetScreenWidth() } as f32;
        let screen_h = unsafe { ffi::GetScreenHeight() } as f32;
        let sx = (wx - cam_x) * zoom + screen_w / 2.0;
        let sy = (wy - cam_y) * zoom + screen_h / 2.0;
        (sx, sy)
    }

    // ==========================================
    // 10. ВВОД И СИСТЕМА (Оригинальные + Новые)
    // ==========================================
    pub fn is_key_down(key_name: &str) -> bool {
        let key = match key_name {
            "W" | "UP" => ffi::KeyboardKey::KEY_W as i32,
            "S" | "DOWN" => ffi::KeyboardKey::KEY_S as i32,
            "A" | "LEFT" => ffi::KeyboardKey::KEY_A as i32,
            "D" | "RIGHT" => ffi::KeyboardKey::KEY_D as i32,
            "SPACE" => ffi::KeyboardKey::KEY_SPACE as i32,
            "ESC" => ffi::KeyboardKey::KEY_ESCAPE as i32,
            "CTRL" | "LEFT_CONTROL" => ffi::KeyboardKey::KEY_LEFT_CONTROL as i32,
            "SHIFT" | "LEFT_SHIFT" => ffi::KeyboardKey::KEY_LEFT_SHIFT as i32,
            "V" => ffi::KeyboardKey::KEY_V as i32,
            "C" => ffi::KeyboardKey::KEY_C as i32,
            _ => 0,
        };
        unsafe { ffi::IsKeyDown(key) }
    }
    pub fn is_key_pressed(key_name: &str) -> bool {
        let key = match key_name {
            "SPACE" => ffi::KeyboardKey::KEY_SPACE as i32,
            "ENTER" => ffi::KeyboardKey::KEY_ENTER as i32,
            "ESC" => ffi::KeyboardKey::KEY_ESCAPE as i32,
            "BACKSPACE" => ffi::KeyboardKey::KEY_BACKSPACE as i32,
            "TAB" => ffi::KeyboardKey::KEY_TAB as i32,
            "CTRL" | "LEFT_CONTROL" => ffi::KeyboardKey::KEY_LEFT_CONTROL as i32,
            "V" => ffi::KeyboardKey::KEY_V as i32,
            "C" => ffi::KeyboardKey::KEY_C as i32,
            _ => 0,
        };
        unsafe { ffi::IsKeyPressed(key) }
    }
    pub fn is_key_released(key_name: &str) -> bool {
        let key = match key_name {
            "SPACE" => ffi::KeyboardKey::KEY_SPACE as i32,
            "ESC" => ffi::KeyboardKey::KEY_ESCAPE as i32,
            _ => 0,
        };
        unsafe { ffi::IsKeyReleased(key) }
    }
    pub fn is_key_up(key_name: &str) -> bool {
        let key = match key_name {
            "W" | "UP" => ffi::KeyboardKey::KEY_W as i32,
            "S" | "DOWN" => ffi::KeyboardKey::KEY_S as i32,
            "A" | "LEFT" => ffi::KeyboardKey::KEY_A as i32,
            "D" | "RIGHT" => ffi::KeyboardKey::KEY_D as i32,
            _ => 0,
        };
        unsafe { ffi::IsKeyUp(key) }
    }
    pub fn get_char_pressed() -> String {
        let c = unsafe { ffi::GetCharPressed() };
        if c > 0 { std::char::from_u32(c as u32).map(|ch| ch.to_string()).unwrap_or_default() } else { String::new() }
    }
    pub fn is_mouse_button_pressed() -> bool { unsafe { ffi::IsMouseButtonPressed(ffi::MouseButton::MOUSE_BUTTON_LEFT as i32) } }
    pub fn is_mouse_button_down(button: i32) -> bool { unsafe { ffi::IsMouseButtonDown(button) } }
    pub fn is_mouse_button_released(button: i32) -> bool { unsafe { ffi::IsMouseButtonReleased(button) } }
    pub fn get_mouse_position() -> (f32, f32) { unsafe { let pos = ffi::GetMousePosition(); (pos.x, pos.y) } }
    pub fn get_mouse_x() -> f32 { unsafe { ffi::GetMouseX() as f32 } }
    pub fn get_mouse_y() -> f32 { unsafe { ffi::GetMouseY() as f32 } }
    pub fn get_mouse_delta_x() -> f32 { unsafe { ffi::GetMouseDelta().x } }
    pub fn get_mouse_delta_y() -> f32 { unsafe { ffi::GetMouseDelta().y } }
    pub fn get_mouse_wheel() -> f32 { unsafe { ffi::GetMouseWheelMove() } }
    pub fn set_mouse_position(x: f32, y: f32) { unsafe { ffi::SetMousePosition(x as i32, y as i32); } }
    pub fn check_collision_point_rec(px: f32, py: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
        unsafe { ffi::CheckCollisionPointRec(ffi::Vector2 { x: px, y: py }, ffi::Rectangle { x: rx, y: ry, width: rw, height: rh }) }
    }

    // ==========================================
    // 11. СИСТЕМНЫЕ ФУНКЦИИ И СЕТЬ
    // ==========================================
    pub fn open_url(url: &str) {
        let c_url = CString::new(url).unwrap();
        unsafe { ffi::OpenURL(c_url.as_ptr()); }
    }
    pub fn get_clipboard() -> String {
        unsafe {
            let c_str = ffi::GetClipboardText();
            if c_str.is_null() { return String::new(); }
            std::ffi::CStr::from_ptr(c_str).to_string_lossy().into_owned()
        }
    }
    pub fn set_clipboard(text: &str) {
        let c_text = std::ffi::CString::new(text).unwrap();
        unsafe { ffi::SetClipboardText(c_text.as_ptr()); }
    }
    pub fn ide_print(text: &str) {
        IDE_OUTPUT.lock().unwrap().push(text.to_string());
        println!("{}", text);
    }
    pub fn get_ide_output() -> Vec<String> { IDE_OUTPUT.lock().unwrap().clone() }
    pub fn clear_ide_output() { IDE_OUTPUT.lock().unwrap().clear(); }
    pub fn http_get(url: &str) -> String {
        match ureq::get(url).call() {
            Ok(resp) => resp.into_string().unwrap_or_default(),
            Err(_) => "ERROR".to_string(),
        }
    }
    pub fn draw_tilemap(tiles: &[i64], map_width: usize, start_x: usize, start_y: usize, end_x: usize, end_y: usize, tile_size: f32) {
        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = y * map_width + x;
                if let Some(&tile) = tiles.get(idx) {
                    let draw_x = x as f32 * tile_size;
                    let draw_y = y as f32 * tile_size;
                    
                    if tile == 1 {
                        unsafe { 
                            ffi::DrawRectangle(draw_x as i32, draw_y as i32, tile_size as i32, tile_size as i32, ffi::Color { r: 50, g: 50, b: 60, a: 255 });
                            ffi::DrawRectangle(draw_x as i32, (draw_y + tile_size - 8.0) as i32, tile_size as i32, 8, ffi::Color { r: 30, g: 30, b: 40, a: 255 });
                        }
                    } else {
                        unsafe { ffi::DrawRectangle(draw_x as i32, draw_y as i32, tile_size as i32, tile_size as i32, ffi::Color { r: 80, g: 140, b: 80, a: 255 }); }
                    }
                }
            }
        }
    }
}