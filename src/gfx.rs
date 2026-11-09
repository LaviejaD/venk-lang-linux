use raylib::ffi;
use std::ffi::CString;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicI32, Ordering};
use std::cell::RefCell;

static IDE_OUTPUT: Mutex<Vec<String>> = Mutex::new(Vec::new());
static TEXTURES: LazyLock<Mutex<HashMap<i32, ffi::Texture2D>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

struct Particle2D {
    x: f32, y: f32,
    vx: f32, vy: f32,
    life: f32, max_life: f32,
    size: f32,
    r: u8, g: u8, b: u8,
}

thread_local! {
    static SOUNDS: RefCell<HashMap<i32, ffi::Sound>> = RefCell::new(HashMap::new());
    static MUSICS: RefCell<HashMap<i32, ffi::Music>> = RefCell::new(HashMap::new());
    static FONTS: RefCell<HashMap<i32, ffi::Font>> = RefCell::new(HashMap::new());
    static PARTICLES: RefCell<Vec<Particle2D>> = RefCell::new(Vec::new());
}

static NEXT_ID: AtomicI32 = AtomicI32::new(1);
static TIME_SCALE: Mutex<f32> = Mutex::new(1.0);
static mut CAMERA_SHAKE: f32 = 0.0;

pub struct Gfx;

impl Gfx {
    // ==========================================
    // 1. СИСТЕМА, ОКНО И ВРЕМЯ
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
        unsafe { let img = ffi::LoadImage(c_path.as_ptr()); ffi::SetWindowIcon(img); ffi::UnloadImage(img); }
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
    // 2. МАТЕМАТИКА ВЕКТОРОВ
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
    // 3. ПРОДВИНУТАЯ 2D ГРАФИКА
    // ==========================================
    pub fn draw_bezier_curve(x1: f32, y1: f32, cx: f32, cy: f32, x2: f32, y2: f32, thick: f32, r: u8, g: u8, b: u8) {
        unsafe {
            ffi::DrawSplineSegmentBezierQuadratic(ffi::Vector2 { x: x1, y: y1 }, ffi::Vector2 { x: cx, y: cy }, ffi::Vector2 { x: x2, y: y2 }, thick, ffi::Color { r, g, b, a: 255 });
        }
    }
    pub fn draw_spline(points: &[(f32, f32)], thick: f32, r: u8, g: u8, b: u8) {
        if points.len() < 2 { return; }
        let mut pts: Vec<ffi::Vector2> = Vec::new();
        for p in points { pts.push(ffi::Vector2 { x: p.0, y: p.1 }); }
        unsafe { ffi::DrawSplineLinear(pts.as_ptr(), pts.len() as i32, thick, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_polygon_custom(points: &[(f32, f32)], r: u8, g: u8, b: u8) {
        if points.len() < 3 { return; }
        let mut pts: Vec<ffi::Vector2> = Vec::new();
        for p in points { pts.push(ffi::Vector2 { x: p.0, y: p.1 }); }
        unsafe { ffi::DrawTriangleFan(pts.as_ptr(), pts.len() as i32, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_dashed_line(x1: f32, y1: f32, x2: f32, y2: f32, dash_len: f32, r: u8, g: u8, b: u8) {
        let dx = x2 - x1; let dy = y2 - y1;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = dash_len / dist;
        let mut t = 0.0;
        unsafe {
            while t < 1.0 {
                let start_x = x1 + dx * t;
                let start_y = y1 + dy * t;
                let end_x = x1 + dx * (t + step).min(1.0);
                let end_y = y1 + dy * (t + step).min(1.0);
                ffi::DrawLine(start_x as i32, start_y as i32, end_x as i32, end_y as i32, ffi::Color { r, g, b, a: 255 });
                t += step * 2.0;
            }
        }
    }
    pub fn draw_arc(cx: f32, cy: f32, radius: f32, start_angle: f32, end_angle: f32, thick: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawRing(ffi::Vector2 { x: cx, y: cy }, radius - thick, radius, start_angle, end_angle, 32, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_rect_gradient_4(x: f32, y: f32, w: f32, h: f32, c1: (u8,u8,u8,u8), c2: (u8,u8,u8,u8), c3: (u8,u8,u8,u8), c4: (u8,u8,u8,u8)) {
        let colors = [
            ffi::Color { r: c1.0, g: c1.1, b: c1.2, a: c1.3 },
            ffi::Color { r: c2.0, g: c2.1, b: c2.2, a: c2.3 },
            ffi::Color { r: c3.0, g: c3.1, b: c3.2, a: c3.3 },
            ffi::Color { r: c4.0, g: c4.1, b: c4.2, a: c4.3 },
        ];
        unsafe { ffi::DrawRectangleGradientEx(ffi::Rectangle { x, y, width: w, height: h }, colors[0], colors[1], colors[2], colors[3]); }
    }
    pub fn draw_circle_gradient(cx: f32, cy: f32, radius: f32, r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) {
        unsafe { ffi::DrawCircleGradient(cx as i32, cy as i32, radius, ffi::Color { r: r1, g: g1, b: b1, a: 255 }, ffi::Color { r: r2, g: g2, b: b2, a: 255 }); }
    }
    pub fn draw_grid_custom(spacing: f32, r: u8, g: u8, b: u8, a: u8) {
        let w = Self::screen_width() as f32;
        let h = Self::screen_height() as f32;
        let color = ffi::Color { r, g, b, a };
        unsafe {
            let mut x = 0.0;
            while x < w { ffi::DrawLine(x as i32, 0, x as i32, h as i32, color); x += spacing; }
            let mut y = 0.0;
            while y < h { ffi::DrawLine(0, y as i32, w as i32, y as i32, color); y += spacing; }
        }
    }
    pub fn draw_capsule(x: f32, y: f32, w: f32, h: f32, r: u8, g: u8, b: u8) {
        unsafe { ffi::DrawRectangleRounded(ffi::Rectangle { x, y, width: w, height: h }, 0.5, 10, ffi::Color { r, g, b, a: 255 }); }
    }
    pub fn draw_star(cx: f32, cy: f32, inner_rad: f32, outer_rad: f32, points: i32, r: u8, g: u8, b: u8) {
        let mut pts = Vec::with_capacity((points * 2) as usize);
        let mut angle = -std::f32::consts::PI / 2.0;
        let step = std::f32::consts::PI / points as f32;
        unsafe {
            for i in 0..(points * 2) {
                let rad = if i % 2 == 0 { outer_rad } else { inner_rad };
                pts.push(ffi::Vector2 { x: cx + rad * angle.cos(), y: cy + rad * angle.sin() });
                angle += step;
            }
            ffi::DrawTriangleFan(pts.as_ptr(), pts.len() as i32, ffi::Color { r, g, b, a: 255 });
        }
    }

    // ==========================================
    // 4. ПРОДВИНУТЫЕ СПРАЙТЫ
    // ==========================================
    pub fn draw_sprite_nine_slice(id: i32, x: f32, y: f32, w: f32, h: f32, _slice: f32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let src = ffi::Rectangle { x: 0.0, y: 0.0, width: tex.width as f32, height: tex.height as f32 };
                let dest = ffi::Rectangle { x, y, width: w, height: h };
                ffi::DrawTexturePro(*tex, src, dest, ffi::Vector2 { x: 0.0, y: 0.0 }, 0.0, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            }
        }
    }
    pub fn draw_sprite_shadow(id: i32, x: f32, y: f32, shadow_offset: f32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let src = ffi::Rectangle { x: 0.0, y: 0.0, width: tex.width as f32, height: tex.height as f32 };
                let dest_shadow = ffi::Rectangle { x: x + shadow_offset, y: y + shadow_offset, width: tex.width as f32, height: tex.height as f32 };
                ffi::DrawTexturePro(*tex, src, dest_shadow, ffi::Vector2 { x: 0.0, y: 0.0 }, 0.0, ffi::Color { r: 0, g: 0, b: 0, a: 150 });
                let dest = ffi::Rectangle { x, y, width: tex.width as f32, height: tex.height as f32 };
                ffi::DrawTexturePro(*tex, src, dest, ffi::Vector2 { x: 0.0, y: 0.0 }, 0.0, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            }
        }
    }
    pub fn draw_sprite_outline(id: i32, x: f32, y: f32, outline_thick: f32, r: u8, g: u8, b: u8) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let src = ffi::Rectangle { x: 0.0, y: 0.0, width: tex.width as f32, height: tex.height as f32 };
                let tint = ffi::Color { r, g, b, a: 255 };
                let dirs = [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)];
                for (dx, dy) in dirs {
                    let dest = ffi::Rectangle { x: x + dx * outline_thick, y: y + dy * outline_thick, width: tex.width as f32, height: tex.height as f32 };
                    ffi::DrawTexturePro(*tex, src, dest, ffi::Vector2 { x: 0.0, y: 0.0 }, 0.0, tint);
                }
            }
        }
    }
    pub fn draw_sprite_flash(id: i32, x: f32, y: f32, alpha: u8) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let src = ffi::Rectangle { x: 0.0, y: 0.0, width: tex.width as f32, height: tex.height as f32 };
                let dest = ffi::Rectangle { x, y, width: tex.width as f32, height: tex.height as f32 };
                ffi::DrawTexturePro(*tex, src, dest, ffi::Vector2 { x: 0.0, y: 0.0 }, 0.0, ffi::Color { r: 255, g: 255, b: 255, a: alpha });
            }
        }
    }
    pub fn set_texture_filter(id: i32, filter: i32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe { ffi::SetTextureFilter(*tex, filter); }
        }
    }
    pub fn set_texture_wrap(id: i32, wrap: i32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe { ffi::SetTextureWrap(*tex, wrap); }
        }
    }
    pub fn get_pixel_color(id: i32, x: i32, y: i32) -> (u8, u8, u8, u8) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let img = ffi::LoadImageFromTexture(*tex);
                let c = ffi::GetImageColor(img, x, y);
                ffi::UnloadImage(img);
                return (c.r, c.g, c.b, c.a);
            }
        }
        (0, 0, 0, 0)
    }
    pub fn draw_animation_variable(id: i32, x: f32, y: f32, frame_w: f32, frame_h: f32, current_frame: i32, cols: i32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let fx = (current_frame % cols) as f32 * frame_w;
                let fy = (current_frame / cols) as f32 * frame_h;
                let src = ffi::Rectangle { x: fx, y: fy, width: frame_w, height: frame_h };
                let dest = ffi::Rectangle { x, y, width: frame_w, height: frame_h };
                ffi::DrawTexturePro(*tex, src, dest, ffi::Vector2 { x: 0.0, y: 0.0 }, 0.0, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            }
        }
    }
    pub fn draw_sprite_batch(id: i32, positions: &[(f32, f32)], scale: f32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let src = ffi::Rectangle { x: 0.0, y: 0.0, width: tex.width as f32, height: tex.height as f32 };
                for pos in positions {
                    let dest = ffi::Rectangle { x: pos.0, y: pos.1, width: tex.width as f32 * scale, height: tex.height as f32 * scale };
                    ffi::DrawTexturePro(*tex, src, dest, ffi::Vector2 { x: 0.0, y: 0.0 }, 0.0, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
                }
            }
        }
    }
    pub fn draw_texture_rect(id: i32, x: f32, y: f32, src_x: f32, src_y: f32, src_w: f32, src_h: f32) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let src = ffi::Rectangle { x: src_x, y: src_y, width: src_w, height: src_h };
                ffi::DrawTextureRec(*tex, src, ffi::Vector2 { x, y }, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            }
        }
    }

    // ==========================================
    // 5. ТЕКСТ И UI
    // ==========================================
    pub fn draw_text_typewriter(font_id: i32, text: &str, x: f32, y: f32, size: f32, speed: f32, time: f32, r: u8, g: u8, b: u8) {
        let visible_chars = (time * speed) as usize;
        let display: String = text.chars().take(visible_chars).collect();
        Self::draw_text_ex(font_id, &display, x, y, size, 1.0, r, g, b);
    }
    pub fn draw_text_rich(font_id: i32, text: &str, x: f32, y: f32, size: f32, default_r: u8, default_g: u8, default_b: u8) {
        let mut curr_x = x;
        let parts: Vec<&str> = text.split('[').collect();
        for part in parts {
            if part.is_empty() { continue; }
            if let Some(end_idx) = part.find(']') {
                let color_str = &part[..end_idx];
                let txt = &part[end_idx+1..];
                let cols: Vec<&str> = color_str.split(',').collect();
                if cols.len() == 3 {
                    let r = cols[0].parse().unwrap_or(default_r);
                    let g = cols[1].parse().unwrap_or(default_g);
                    let b = cols[2].parse().unwrap_or(default_b);
                    Self::draw_text_ex(font_id, txt, curr_x, y, size, 1.0, r, g, b);
                    curr_x += Self::measure_text_ex(font_id, txt, size).0;
                } else {
                    Self::draw_text_ex(font_id, part, curr_x, y, size, 1.0, default_r, default_g, default_b);
                    curr_x += Self::measure_text_ex(font_id, part, size).0;
                }
            } else {
                Self::draw_text_ex(font_id, part, curr_x, y, size, 1.0, default_r, default_g, default_b);
                curr_x += Self::measure_text_ex(font_id, part, size).0;
            }
        }
    }
    pub fn draw_text_aligned(font_id: i32, text: &str, y: f32, size: f32, alignment: i32, r: u8, g: u8, b: u8) {
        let w = Self::screen_width() as f32;
        let tw = Self::measure_text_ex(font_id, text, size).0;
        let x = match alignment { 1 => (w - tw) / 2.0, 2 => w - tw - 20.0, _ => 20.0 };
        Self::draw_text_ex(font_id, text, x, y, size, 1.0, r, g, b);
    }
    pub fn gui_text_input(x: f32, y: f32, w: f32, h: f32, text: &str) -> String {
        let mut buf = [0i8; 256];
        let bytes = text.as_bytes();
        let len = bytes.len().min(255);
        for i in 0..len { buf[i] = bytes[i] as i8; }
        unsafe {
            ffi::GuiTextBox(ffi::Rectangle { x, y, width: w, height: h }, buf.as_mut_ptr(), 256, true);
            let c_str = std::ffi::CStr::from_ptr(buf.as_ptr());
            c_str.to_string_lossy().into_owned()
        }
    }
    pub fn gui_checkbox(x: f32, y: f32, w: f32, h: f32, text: &str, checked: bool) -> bool {
        let c_text = CString::new(text).unwrap();
        let mut val = checked;
        unsafe { ffi::GuiCheckBox(ffi::Rectangle { x, y, width: w, height: h }, c_text.as_ptr(), &mut val); }
        val
    }
    pub fn gui_progress_bar(x: f32, y: f32, w: f32, h: f32, text: &str, value: f32, min: f32, max: f32) {
        let c_text = CString::new(text).unwrap();
        let c_empty = CString::new("").unwrap();
        let mut val = value;
        unsafe { ffi::GuiProgressBar(ffi::Rectangle { x, y, width: w, height: h }, c_text.as_ptr(), c_empty.as_ptr(), &mut val, min, max); }
    }
    pub fn gui_panel(x: f32, y: f32, w: f32, h: f32, text: &str) {
        let c_text = CString::new(text).unwrap();
        unsafe { ffi::GuiPanel(ffi::Rectangle { x, y, width: w, height: h }, c_text.as_ptr()); }
    }
    
    // ИСПРАВЛЕНИЕ 1: Добавлен 5-й аргумент для GuiScrollPanel
    pub fn gui_scrollview(x: f32, y: f32, w: f32, h: f32, text: &str) {
        let c_text = CString::new(text).unwrap();
        let mut scroll = ffi::Vector2 { x: 0.0, y: 0.0 };
        let mut view = ffi::Rectangle { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };
        unsafe { 
            ffi::GuiScrollPanel(
                ffi::Rectangle { x, y, width: w, height: h }, 
                c_text.as_ptr(), 
                ffi::Rectangle { x: 0.0, y: 0.0, width: w - 20.0, height: h * 2.0 }, 
                &mut scroll, 
                &mut view
            ); 
        }
    }
    
    pub fn gui_tooltip(text: &str, x: f32, y: f32) {
        let c_text = CString::new(text).unwrap();
        unsafe { ffi::GuiLabelButton(ffi::Rectangle { x, y, width: 200.0, height: 30.0 }, c_text.as_ptr()); }
    }
    pub fn draw_text_3d_billboard(font_id: i32, text: &str, x: f32, y: f32, size: f32, r: u8, g: u8, b: u8) {
        Self::draw_text_ex(font_id, text, x, y, size, 1.0, r, g, b);
    }

    // ==========================================
    // 6. КАМЕРА, ФИЗИКА И МАТЕМАТИКА
    // ==========================================
    pub fn camera_follow_smooth(cam_x: f32, cam_y: f32, target_x: f32, target_y: f32, smoothness: f32) -> (f32, f32) {
        let t = smoothness.min(1.0).max(0.0);
        (cam_x + (target_x - cam_x) * t, cam_y + (target_y - cam_y) * t)
    }
    pub fn camera_set_bounds(cam_x: f32, cam_y: f32, min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> (f32, f32) {
        (cam_x.max(min_x).min(max_x), cam_y.max(min_y).min(max_y))
    }
    pub fn camera_shake_perlin(intensity: f32) -> (f32, f32) {
        let t = Self::get_time() as f32 * 10.0;
        let dx = (t.sin() * 12.9898).fract() * intensity;
        let dy = (t.cos() * 78.233).fract() * intensity;
        (dx, dy)
    }
    pub fn collision_point_poly(px: f32, py: f32, points: &[(f32, f32)]) -> bool {
        let mut pts = Vec::new();
        for p in points { pts.push(ffi::Vector2 { x: p.0, y: p.1 }); }
        unsafe { ffi::CheckCollisionPointPoly(ffi::Vector2 { x: px, y: py }, pts.as_ptr(), pts.len() as i32) }
    }
    
    // ИСПРАВЛЕНИЕ 2: Переписана логика CheckCollisionLines (теперь передается 5 аргументов, проверяются 4 стороны)
    pub fn collision_line_rect(x1: f32, y1: f32, x2: f32, y2: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
        let rec = ffi::Rectangle { x: rx, y: ry, width: rw, height: rh };
        let start = ffi::Vector2 { x: x1, y: y1 };
        let end = ffi::Vector2 { x: x2, y: y2 };
        unsafe {
            if ffi::CheckCollisionPointRec(start, rec) || ffi::CheckCollisionPointRec(end, rec) {
                return true;
            }
            let mut pt = ffi::Vector2 { x: 0.0, y: 0.0 };
            if ffi::CheckCollisionLines(start, end, ffi::Vector2 { x: rx, y: ry }, ffi::Vector2 { x: rx + rw, y: ry }, &mut pt) { return true; }
            if ffi::CheckCollisionLines(start, end, ffi::Vector2 { x: rx + rw, y: ry }, ffi::Vector2 { x: rx + rw, y: ry + rh }, &mut pt) { return true; }
            if ffi::CheckCollisionLines(start, end, ffi::Vector2 { x: rx + rw, y: ry + rh }, ffi::Vector2 { x: rx, y: ry + rh }, &mut pt) { return true; }
            if ffi::CheckCollisionLines(start, end, ffi::Vector2 { x: rx, y: ry + rh }, ffi::Vector2 { x: rx, y: ry }, &mut pt) { return true; }
        }
        false
    }
    
    // ИСПРАВЛЕНИЕ 3: Исправлен raycast_2d (убраны лишние аргументы у CheckCollisionLines, проверяются 4 ребра стены)
    pub fn raycast_2d(x1: f32, y1: f32, x2: f32, y2: f32, walls: &[(f32, f32, f32, f32)]) -> (f32, f32) {
        let mut closest_point = (x2, y2);
        let mut min_dist = f32::MAX;
        unsafe {
            for w in walls {
                let mut collision_point = ffi::Vector2 { x: 0.0, y: 0.0 };
                let edges = [
                    (ffi::Vector2 { x: w.0, y: w.1 }, ffi::Vector2 { x: w.0 + w.2, y: w.1 }),
                    (ffi::Vector2 { x: w.0 + w.2, y: w.1 }, ffi::Vector2 { x: w.0 + w.2, y: w.1 + w.3 }),
                    (ffi::Vector2 { x: w.0 + w.2, y: w.1 + w.3 }, ffi::Vector2 { x: w.0, y: w.1 + w.3 }),
                    (ffi::Vector2 { x: w.0, y: w.1 + w.3 }, ffi::Vector2 { x: w.0, y: w.1 }),
                ];
                for (edge_start, edge_end) in edges {
                    if ffi::CheckCollisionLines(ffi::Vector2 { x: x1, y: y1 }, ffi::Vector2 { x: x2, y: y2 }, edge_start, edge_end, &mut collision_point) {
                        let real_dist = ((collision_point.x - x1).powi(2) + (collision_point.y - y1).powi(2)).sqrt();
                        if real_dist < min_dist {
                            min_dist = real_dist;
                            closest_point = (collision_point.x, collision_point.y);
                        }
                    }
                }
            }
        }
        closest_point
    }
    
    pub fn get_overlap_rect(x1: f32, y1: f32, w1: f32, h1: f32, x2: f32, y2: f32, w2: f32, h2: f32) -> (f32, f32, f32, f32) {
        let r = unsafe { ffi::GetCollisionRec(ffi::Rectangle { x: x1, y: y1, width: w1, height: h1 }, ffi::Rectangle { x: x2, y: y2, width: w2, height: h2 }) };
        (r.x, r.y, r.width, r.height)
    }
    pub fn vec2_slerp(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> (f32, f32) {
        let dot = (x1 * x2 + y1 * y2).clamp(-1.0, 1.0);
        let theta = dot.acos() * t;
        let _sin_theta = theta.sin();
        let (rx, ry) = if dot > 0.9995 {
            (x1 + (x2 - x1) * t, y1 + (y2 - y1) * t)
        } else {
            let (s1, s2) = ((1.0 - t) * theta.cos(), t * theta.cos());
            (s1 * x1 + s2 * x2, s1 * y1 + s2 * y2)
        };
        (rx, ry)
    }
    pub fn point_in_circle(px: f32, py: f32, cx: f32, cy: f32, radius: f32) -> bool {
        unsafe { ffi::CheckCollisionPointCircle(ffi::Vector2 { x: px, y: py }, ffi::Vector2 { x: cx, y: cy }, radius) }
    }
    pub fn get_mouse_world_pos(mouse_x: f32, mouse_y: f32, cam_x: f32, cam_y: f32, zoom: f32) -> (f32, f32) {
        let screen_w = unsafe { ffi::GetScreenWidth() } as f32;
        let screen_h = unsafe { ffi::GetScreenHeight() } as f32;
        let wx = (mouse_x - screen_w / 2.0) / zoom + cam_x;
        let wy = (mouse_y - screen_h / 2.0) / zoom + cam_y;
        (wx, wy)
    }

    // ==========================================
    // 7. ЧАСТИЦЫ, ЗВУК И ЭФФЕКТЫ
    // ==========================================
    pub fn particles_create_emitter(x: f32, y: f32, count: i32, r: u8, g: u8, b: u8) {
        PARTICLES.with(|p| {
            let mut particles = p.borrow_mut();
            for _ in 0..count {
                let vx = (rand_unit() as f32 - 0.5) * 200.0;
                let vy = (rand_unit() as f32 - 0.5) * 200.0;
                particles.push(Particle2D { x, y, vx, vy, life: 1.0, max_life: 1.0, size: 5.0, r, g, b });
            }
        });
    }
    pub fn particles_set_direction(dx: f32, dy: f32, spread: f32) {
        PARTICLES.with(|p| {
            for particle in p.borrow_mut().iter_mut() {
                let angle = dy.atan2(dx) + (rand_unit() as f32 - 0.5) * spread;
                let speed = (particle.vx * particle.vx + particle.vy * particle.vy).sqrt();
                particle.vx = angle.cos() * speed;
                particle.vy = angle.sin() * speed;
            }
        });
    }
    pub fn particles_set_color_gradient(r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) {
        PARTICLES.with(|p| {
            for particle in p.borrow_mut().iter_mut() {
                let t = 1.0 - (particle.life / particle.max_life);
                particle.r = (r1 as f32 * (1.0 - t) + r2 as f32 * t) as u8;
                particle.g = (g1 as f32 * (1.0 - t) + g2 as f32 * t) as u8;
                particle.b = (b1 as f32 * (1.0 - t) + b2 as f32 * t) as u8;
            }
        });
    }
    pub fn particles_attach_to_sprite(x: f32, y: f32) {
        PARTICLES.with(|p| {
            if let Some(last) = p.borrow_mut().last_mut() {
                last.x = x;
                last.y = y;
            }
        });
    }
    pub fn particles_update_2d(dt: f32) {
        PARTICLES.with(|p| {
            let mut particles = p.borrow_mut();
            for particle in particles.iter_mut() {
                particle.x += particle.vx * dt;
                particle.y += particle.vy * dt;
                particle.life -= dt;
            }
            particles.retain(|p| p.life > 0.0);
        });
    }
    pub fn particles_draw_2d() {
        PARTICLES.with(|p| {
            let particles = p.borrow();
            for particle in particles.iter() {
                unsafe {
                    ffi::DrawCircle(particle.x as i32, particle.y as i32, particle.size, ffi::Color { r: particle.r, g: particle.g, b: particle.b, a: (particle.life * 255.0) as u8 });
                }
            }
        });
    }
    pub fn play_sound_pitched(id: i32, pitch: f32) {
        SOUNDS.with(|s| {
            if let Some(snd) = s.borrow().get(&id) {
                unsafe { ffi::SetSoundPitch(*snd, pitch); ffi::PlaySound(*snd); }
            }
        });
    }
    pub fn play_sound_positional(id: i32, x: f32, y: f32, cam_x: f32, cam_y: f32) {
        let dist = ((x - cam_x).powi(2) + (y - cam_y).powi(2)).sqrt();
        let vol = (1.0 - (dist / 1000.0)).max(0.0);
        SOUNDS.with(|s| {
            if let Some(snd) = s.borrow().get(&id) {
                unsafe { ffi::SetSoundVolume(*snd, vol); ffi::PlaySound(*snd); }
            }
        });
    }
    pub fn music_set_volume(id: i32, vol: f32) {
        MUSICS.with(|m| { if let Some(mus) = m.borrow().get(&id) { unsafe { ffi::SetMusicVolume(*mus, vol); } } });
    }
    pub fn sound_set_volume(id: i32, vol: f32) {
        SOUNDS.with(|s| { if let Some(snd) = s.borrow().get(&id) { unsafe { ffi::SetSoundVolume(*snd, vol); } } });
    }
    pub fn screen_flash(r: u8, g: u8, b: u8, alpha: u8) {
        unsafe { ffi::DrawRectangle(0, 0, ffi::GetScreenWidth(), ffi::GetScreenHeight(), ffi::Color { r, g, b, a: alpha }); }
    }
    pub fn screen_fade(alpha: u8) {
        unsafe { ffi::DrawRectangle(0, 0, ffi::GetScreenWidth(), ffi::GetScreenHeight(), ffi::Color { r: 0, g: 0, b: 0, a: alpha }); }
    }
    pub fn draw_vignette(intensity: f32) {
        let w = unsafe { ffi::GetScreenWidth() } as f32;
        let h = unsafe { ffi::GetScreenHeight() } as f32;
        let alpha = (intensity * 255.0) as u8;
        unsafe {
            ffi::DrawRectangleGradientV(0, 0, w as i32, 100, ffi::Color { r: 0, g: 0, b: 0, a: alpha }, ffi::Color { r: 0, g: 0, b: 0, a: 0 });
            ffi::DrawRectangleGradientV(0, (h - 100.0) as i32, w as i32, 100, ffi::Color { r: 0, g: 0, b: 0, a: 0 }, ffi::Color { r: 0, g: 0, b: 0, a: alpha });
        }
    }

    // ==========================================
    // 8. СТАРЫЕ ФУНКЦИИ (ТЕКСТУРЫ, ШРИФТЫ, ТЕКСТ)
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
    pub fn get_texture_width(id: i32) -> i32 { let texs = TEXTURES.lock().unwrap(); if let Some(tex) = texs.get(&id) { tex.width } else { 0 } }
    pub fn get_texture_height(id: i32) -> i32 { let texs = TEXTURES.lock().unwrap(); if let Some(tex) = texs.get(&id) { tex.height } else { 0 } }
    pub fn draw_sprite_ex(id: i32, x: f32, y: f32, rot: f32, scale_x: f32, scale_y: f32, flip_x: bool, flip_y: bool) {
        let texs = TEXTURES.lock().unwrap();
        if let Some(tex) = texs.get(&id) {
            unsafe {
                let source = ffi::Rectangle { x: 0.0, y: 0.0, width: tex.width as f32, height: tex.height as f32 };
                let dest = ffi::Rectangle { x, y, width: tex.width as f32 * scale_x, height: tex.height as f32 * scale_y };
                let origin = ffi::Vector2 { x: dest.width / 2.0, y: dest.height / 2.0 };
                let mut final_rot = rot; if flip_x { final_rot = -final_rot; }
                ffi::DrawTexturePro(*tex, source, dest, origin, final_rot, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
            }
        }
    }
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
                        ffi::DrawTextureRec(*tex, source, pos, ffi::Color { r: 255, g: 255, b: 255, a: 255 });
                        dx += step_x;
                    }
                    dy += step_y;
                }
            }
        }
    }
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
            unsafe { ffi::DrawTextEx(font, c_text.as_ptr(), ffi::Vector2 { x, y }, size, spacing, ffi::Color { r, g, b, a: 255 }); }
        });
    }
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
                    } else { line = test_line; }
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
    // 9. СТАРЫЕ ФИГУРЫ И UI
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
        unsafe { let rec = ffi::Rectangle { x, y, width: w, height: h }; ffi::DrawRectangleLinesEx(rec, thick, ffi::Color { r, g, b, a: 255 }); }
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
        unsafe { ffi::DrawTriangle(ffi::Vector2 { x: x1, y: y1 }, ffi::Vector2 { x: x2, y: y2 }, ffi::Vector2 { x: x3, y: y3 }, ffi::Color { r, g, b, a: 255 }); }
    }
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
        Self::draw_rectangle(x, y, w, h, 40, 40, 40);
        Self::draw_rectangle(x, y, w * ratio, h, r, g, b);
        Self::draw_rect_lines(x, y, w, h, 1.0, 0, 0, 0);
    }
    pub fn gui_button(x: f32, y: f32, w: f32, h: f32, text: &str) -> bool {
        let mx = unsafe { ffi::GetMouseX() } as f32; let my = unsafe { ffi::GetMouseY() } as f32;
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
        let mx = unsafe { ffi::GetMouseX() } as f32; let my = unsafe { ffi::GetMouseY() } as f32;
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
    // 10. СТАРЫЕ КОЛЛИЗИИ, КАМЕРА И ВВОД
    // ==========================================
    pub fn collision_rect(x1: f32, y1: f32, w1: f32, h1: f32, x2: f32, y2: f32, w2: f32, h2: f32) -> bool {
        unsafe { ffi::CheckCollisionRecs(ffi::Rectangle { x: x1, y: y1, width: w1, height: h1 }, ffi::Rectangle { x: x2, y: y2, width: w2, height: h2 }) }
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
    pub fn begin_camera(x: f32, y: f32, zoom: f32) {
        unsafe {
            let screen_w = ffi::GetScreenWidth() as f32;
            let screen_h = ffi::GetScreenHeight() as f32;
            let mut shake_x = 0.0; let mut shake_y = 0.0;
            if CAMERA_SHAKE > 0.0 {
                shake_x = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as f32 % 100.0 - 50.0) * CAMERA_SHAKE;
                shake_y = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() as f32 % 100.0 - 50.0) * CAMERA_SHAKE;
                CAMERA_SHAKE -= 0.05; if CAMERA_SHAKE < 0.0 { CAMERA_SHAKE = 0.0; }
            }
            let camera = ffi::Camera2D { offset: ffi::Vector2 { x: screen_w / 2.0 + shake_x, y: screen_h / 2.0 + shake_y }, target: ffi::Vector2 { x, y }, rotation: 0.0, zoom };
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
    pub fn is_key_down(key_name: &str) -> bool {
        let key = match key_name {
            "W" | "UP" => ffi::KeyboardKey::KEY_W as i32, "S" | "DOWN" => ffi::KeyboardKey::KEY_S as i32,
            "A" | "LEFT" => ffi::KeyboardKey::KEY_A as i32, "D" | "RIGHT" => ffi::KeyboardKey::KEY_D as i32,
            "SPACE" => ffi::KeyboardKey::KEY_SPACE as i32, "ESC" => ffi::KeyboardKey::KEY_ESCAPE as i32,
            "CTRL" | "LEFT_CONTROL" => ffi::KeyboardKey::KEY_LEFT_CONTROL as i32, "SHIFT" | "LEFT_SHIFT" => ffi::KeyboardKey::KEY_LEFT_SHIFT as i32,
            "V" => ffi::KeyboardKey::KEY_V as i32, "C" => ffi::KeyboardKey::KEY_C as i32, _ => 0,
        };
        unsafe { ffi::IsKeyDown(key) }
    }
    pub fn is_key_pressed(key_name: &str) -> bool {
        let key = match key_name {
            "SPACE" => ffi::KeyboardKey::KEY_SPACE as i32, "ENTER" => ffi::KeyboardKey::KEY_ENTER as i32,
            "ESC" => ffi::KeyboardKey::KEY_ESCAPE as i32, "BACKSPACE" => ffi::KeyboardKey::KEY_BACKSPACE as i32,
            "TAB" => ffi::KeyboardKey::KEY_TAB as i32, "CTRL" | "LEFT_CONTROL" => ffi::KeyboardKey::KEY_LEFT_CONTROL as i32,
            "V" => ffi::KeyboardKey::KEY_V as i32, "C" => ffi::KeyboardKey::KEY_C as i32, _ => 0,
        };
        unsafe { ffi::IsKeyPressed(key) }
    }
    pub fn is_key_released(key_name: &str) -> bool {
        let key = match key_name { "SPACE" => ffi::KeyboardKey::KEY_SPACE as i32, "ESC" => ffi::KeyboardKey::KEY_ESCAPE as i32, _ => 0 };
        unsafe { ffi::IsKeyReleased(key) }
    }
    pub fn is_key_up(key_name: &str) -> bool {
        let key = match key_name {
            "W" | "UP" => ffi::KeyboardKey::KEY_W as i32, "S" | "DOWN" => ffi::KeyboardKey::KEY_S as i32,
            "A" | "LEFT" => ffi::KeyboardKey::KEY_A as i32, "D" | "RIGHT" => ffi::KeyboardKey::KEY_D as i32, _ => 0,
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
    // 11. СТАРЫЕ ЗВУКИ И СИСТЕМА
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
    pub fn play_sound(id: i32) { SOUNDS.with(|s| { if let Some(snd) = s.borrow().get(&id) { unsafe { ffi::PlaySound(*snd); } } }); }
    pub fn stop_sound(id: i32) { SOUNDS.with(|s| { if let Some(snd) = s.borrow().get(&id) { unsafe { ffi::StopSound(*snd); } } }); }
    pub fn unload_sound(id: i32) { SOUNDS.with(|s| { if let Some(snd) = s.borrow_mut().remove(&id) { unsafe { ffi::UnloadSound(snd); } } }); }
    pub fn load_music(path: &str) -> i32 {
        let c_path = CString::new(path).unwrap();
        unsafe {
            let mus = ffi::LoadMusicStream(c_path.as_ptr());
            let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
            MUSICS.with(|m| { m.borrow_mut().insert(id, mus); });
            id
        }
    }
    pub fn play_music(id: i32) { MUSICS.with(|m| { if let Some(mus) = m.borrow().get(&id) { unsafe { ffi::PlayMusicStream(*mus); } } }); }
    pub fn update_music(id: i32) { MUSICS.with(|m| { if let Some(mus) = m.borrow().get(&id) { unsafe { ffi::UpdateMusicStream(*mus); } } }); }
    pub fn stop_music(id: i32) { MUSICS.with(|m| { if let Some(mus) = m.borrow().get(&id) { unsafe { ffi::StopMusicStream(*mus); } } }); }
    
    pub fn open_url(url: &str) { let c_url = CString::new(url).unwrap(); unsafe { ffi::OpenURL(c_url.as_ptr()); } }
    pub fn get_clipboard() -> String {
        unsafe { let c_str = ffi::GetClipboardText(); if c_str.is_null() { return String::new(); } std::ffi::CStr::from_ptr(c_str).to_string_lossy().into_owned() }
    }
    pub fn set_clipboard(text: &str) { let c_text = std::ffi::CString::new(text).unwrap(); unsafe { ffi::SetClipboardText(c_text.as_ptr()); } }
    pub fn ide_print(text: &str) { IDE_OUTPUT.lock().unwrap().push(text.to_string()); println!("{}", text); }
    pub fn get_ide_output() -> Vec<String> { IDE_OUTPUT.lock().unwrap().clone() }
    pub fn clear_ide_output() { IDE_OUTPUT.lock().unwrap().clear(); }
    pub fn http_get(url: &str) -> String {
        match ureq::get(url).call() { Ok(resp) => resp.into_string().unwrap_or_default(), Err(_) => "ERROR".to_string() }
    }
    pub fn draw_tilemap(tiles: &[i64], map_width: usize, start_x: usize, start_y: usize, end_x: usize, end_y: usize, tile_size: f32) {
        for y in start_y..end_y {
            for x in start_x..end_x {
                let idx = y * map_width + x;
                if let Some(&tile) = tiles.get(idx) {
                    let draw_x = x as f32 * tile_size; let draw_y = y as f32 * tile_size;
                    if tile == 1 {
                        unsafe { ffi::DrawRectangle(draw_x as i32, draw_y as i32, tile_size as i32, tile_size as i32, ffi::Color { r: 50, g: 50, b: 60, a: 255 }); }
                    } else {
                        unsafe { ffi::DrawRectangle(draw_x as i32, draw_y as i32, tile_size as i32, tile_size as i32, ffi::Color { r: 80, g: 140, b: 80, a: 255 }); }
                    }
                }
            }
        }
    }
}

fn rand_unit() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let x = ns ^ (ns << 13) ^ 0x9e3779b97f4a7c15;
    (x >> 11) as f64 / (1u64 << 53) as f64
}