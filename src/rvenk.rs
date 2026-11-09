use raylib::ffi;
use std::ffi::CString;
use crate::interp::{Interp, Signal, R};
use crate::value::*;
use std::rc::Rc;

// ---------- математические хелперы (т.к. их нет в ffi) ----------
fn math_clamp(value: f32, min: f32, max: f32) -> f32 {
    if value < min { min } else if value > max { max } else { value }
}

fn math_lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + amount * (end - start)
}

// ---------- хелперы: структуры <-> i64 ----------
fn box_texture(t: ffi::Texture2D) -> i64 { Box::into_raw(Box::new(t)) as i64 }
fn box_font(f: ffi::Font)         -> i64 { Box::into_raw(Box::new(f)) as i64 }
fn box_sound(s: ffi::Sound)       -> i64 { Box::into_raw(Box::new(s)) as i64 }
fn box_music(m: ffi::Music)       -> i64 { Box::into_raw(Box::new(m)) as i64 }
fn box_model(m: ffi::Model)       -> i64 { Box::into_raw(Box::new(m)) as i64 }
fn box_shader(s: ffi::Shader)     -> i64 { Box::into_raw(Box::new(s)) as i64 }

unsafe fn take_texture(id: i64) -> ffi::Texture2D { std::ptr::read(id as *const ffi::Texture2D) }
unsafe fn take_font(id: i64)    -> ffi::Font      { std::ptr::read(id as *const ffi::Font) }
unsafe fn take_sound(id: i64)   -> ffi::Sound     { std::ptr::read(id as *const ffi::Sound) }
unsafe fn take_music(id: i64)   -> ffi::Music     { std::ptr::read(id as *const ffi::Music) }
unsafe fn take_model(id: i64)   -> ffi::Model     { std::ptr::read(id as *const ffi::Model) }
unsafe fn take_shader(id: i64)  -> ffi::Shader    { std::ptr::read(id as *const ffi::Shader) }

// ---------- маленькие хелперы для извлечения чисел ----------
macro_rules! arg_i32 { ($a:expr, $n:expr) => { match $a.remove(0) {
    Value::Int(n) => n as i32, Value::Float(f) => f as i32,
    other => return Err(Signal::Runtime(format!(concat!($n, ": ожидалось число, получен {}"), other.type_name()))),
}}}
macro_rules! arg_f32 { ($a:expr, $n:expr) => { match $a.remove(0) {
    Value::Int(n) => n as f32, Value::Float(f) => f as f32,
    other => return Err(Signal::Runtime(format!(concat!($n, ": ожидалось число, получен {}"), other.type_name()))),
}}}
macro_rules! arg_u8 { ($a:expr, $n:expr) => { match $a.remove(0) {
    Value::Int(n) => n as u8, Value::Float(f) => f as u8,
    other => return Err(Signal::Runtime(format!(concat!($n, ": ожидалось число, получен {}"), other.type_name()))),
}}}
macro_rules! arg_str { ($a:expr, $n:expr) => { match $a.remove(0) {
    Value::Str(s) => s,
    other => return Err(Signal::Runtime(format!(concat!($n, ": ожидалась строка, получен {}"), other.type_name()))),
}}}
macro_rules! arg_id { ($a:expr, $n:expr) => { match $a.remove(0) {
    Value::Int(n) => n,
    other => return Err(Signal::Runtime(format!(concat!($n, ": ожидался id, получен {}"), other.type_name()))),
}}}

impl Interp {
    pub(crate) fn call_rvenk(&mut self, name: &str, mut args: Vec<Value>) -> R {
        match name {
            // ===== СИСТЕМА И ОКНО =====
            "rvenk_init_window" => {
                let w = arg_i32!(args, "rvenk_init_window(width)");
                let h = arg_i32!(args, "rvenk_init_window(height)");
                let title = arg_str!(args, "rvenk_init_window(title)");
                let c_title = CString::new(title.as_str()).unwrap();
                unsafe { ffi::InitWindow(w, h, c_title.as_ptr()); }
                Ok(Value::Null)
            }
            "rvenk_close_window" => { unsafe { ffi::CloseWindow(); } Ok(Value::Null) }
            "rvenk_window_should_close" => Ok(Value::Bool(unsafe { ffi::WindowShouldClose() })),
            "rvenk_begin_drawing" => { unsafe { ffi::BeginDrawing(); } Ok(Value::Null) }
            "rvenk_end_drawing"   => { unsafe { ffi::EndDrawing(); }   Ok(Value::Null) }

            "rvenk_clear_background" => {
                let r = arg_u8!(args, "clear_background(r)");
                let g = arg_u8!(args, "clear_background(g)");
                let b = arg_u8!(args, "clear_background(b)");
                let a = arg_u8!(args, "clear_background(a)");
                unsafe { ffi::ClearBackground(ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_set_target_fps" => { let fps = arg_i32!(args, "set_target_fps"); unsafe { ffi::SetTargetFPS(fps); } Ok(Value::Null) }
            "rvenk_get_fps" => Ok(Value::Int(unsafe { ffi::GetFPS() } as i64)),
            "rvenk_get_screen_width"  => Ok(Value::Int(unsafe { ffi::GetScreenWidth()  } as i64)),
            "rvenk_get_screen_height" => Ok(Value::Int(unsafe { ffi::GetScreenHeight() } as i64)),

            // ===== 2D ФИГУРЫ =====
            "rvenk_draw_pixel" => {
                let x = arg_i32!(args, "draw_pixel(x)");
                let y = arg_i32!(args, "draw_pixel(y)");
                let r = arg_u8!(args, "draw_pixel(r)");
                let g = arg_u8!(args, "draw_pixel(g)");
                let b = arg_u8!(args, "draw_pixel(b)");
                let a = arg_u8!(args, "draw_pixel(a)");
                unsafe { ffi::DrawPixel(x, y, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_line" => {
                let x1 = arg_i32!(args, "draw_line(x1)");
                let y1 = arg_i32!(args, "draw_line(y1)");
                let x2 = arg_i32!(args, "draw_line(x2)");
                let y2 = arg_i32!(args, "draw_line(y2)");
                let r = arg_u8!(args, "draw_line(r)");
                let g = arg_u8!(args, "draw_line(g)");
                let b = arg_u8!(args, "draw_line(b)");
                let a = arg_u8!(args, "draw_line(a)");
                unsafe { ffi::DrawLine(x1, y1, x2, y2, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_circle" => {
                let x = arg_i32!(args, "draw_circle(x)");
                let y = arg_i32!(args, "draw_circle(y)");
                let rad = arg_f32!(args, "draw_circle(radius)");
                let r = arg_u8!(args, "draw_circle(r)");
                let g = arg_u8!(args, "draw_circle(g)");
                let b = arg_u8!(args, "draw_circle(b)");
                let a = arg_u8!(args, "draw_circle(a)");
                unsafe { ffi::DrawCircle(x, y, rad, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_rectangle" => {
                let x = arg_i32!(args, "draw_rectangle(x)");
                let y = arg_i32!(args, "draw_rectangle(y)");
                let w = arg_i32!(args, "draw_rectangle(w)");
                let h = arg_i32!(args, "draw_rectangle(h)");
                let r = arg_u8!(args, "draw_rectangle(r)");
                let g = arg_u8!(args, "draw_rectangle(g)");
                let b = arg_u8!(args, "draw_rectangle(b)");
                let a = arg_u8!(args, "draw_rectangle(a)");
                unsafe { ffi::DrawRectangle(x, y, w, h, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_circle_lines" => {
                let x = arg_i32!(args, "circle_lines(x)");
                let y = arg_i32!(args, "circle_lines(y)");
                let rad = arg_f32!(args, "circle_lines(r)");
                let r = arg_u8!(args, "circle_lines(r)");
                let g = arg_u8!(args, "circle_lines(g)");
                let b = arg_u8!(args, "circle_lines(b)");
                let a = arg_u8!(args, "circle_lines(a)");
                unsafe { ffi::DrawCircleLines(x, y, rad, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_rectangle_lines" => {
                let x = arg_i32!(args, "rect_lines(x)");
                let y = arg_i32!(args, "rect_lines(y)");
                let w = arg_i32!(args, "rect_lines(w)");
                let h = arg_i32!(args, "rect_lines(h)");
                let r = arg_u8!(args, "rect_lines(r)");
                let g = arg_u8!(args, "rect_lines(g)");
                let b = arg_u8!(args, "rect_lines(b)");
                let a = arg_u8!(args, "rect_lines(a)");
                unsafe { ffi::DrawRectangleLines(x, y, w, h, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_triangle" => {
                let x1 = arg_f32!(args, "triangle(x1)");
                let y1 = arg_f32!(args, "triangle(y1)");
                let x2 = arg_f32!(args, "triangle(x2)");
                let y2 = arg_f32!(args, "triangle(y2)");
                let x3 = arg_f32!(args, "triangle(x3)");
                let y3 = arg_f32!(args, "triangle(y3)");
                let r = arg_u8!(args, "triangle(r)");
                let g = arg_u8!(args, "triangle(g)");
                let b = arg_u8!(args, "triangle(b)");
                let a = arg_u8!(args, "triangle(a)");
                unsafe {
                    ffi::DrawTriangle(
                        ffi::Vector2 { x: x1, y: y1 },
                        ffi::Vector2 { x: x2, y: y2 },
                        ffi::Vector2 { x: x3, y: y3 },
                        ffi::Color { r, g, b, a },
                    );
                }
                Ok(Value::Null)
            }

            // ===== 3D =====
            "rvenk_begin_mode_3d" => {
                let px = arg_f32!(args, "mode3d(px)");
                let py = arg_f32!(args, "mode3d(py)");
                let pz = arg_f32!(args, "mode3d(pz)");
                let tx = arg_f32!(args, "mode3d(tx)");
                let ty = arg_f32!(args, "mode3d(ty)");
                let tz = arg_f32!(args, "mode3d(tz)");
                let fovy = arg_f32!(args, "mode3d(fovy)");
                unsafe {
                    let camera = ffi::Camera3D {
                        position: ffi::Vector3 { x: px, y: py, z: pz },
                        target:   ffi::Vector3 { x: tx, y: ty, z: tz },
                        up:       ffi::Vector3 { x: 0.0, y: 1.0, z: 0.0 },
                        fovy,
                        projection: ffi::CameraProjection::CAMERA_PERSPECTIVE as i32,
                    };
                    ffi::BeginMode3D(camera);
                }
                Ok(Value::Null)
            }
            "rvenk_end_mode_3d" => { unsafe { ffi::EndMode3D(); } Ok(Value::Null) }

            "rvenk_draw_cube" => {
                let x = arg_f32!(args, "cube(x)");
                let y = arg_f32!(args, "cube(y)");
                let z = arg_f32!(args, "cube(z)");
                let w = arg_f32!(args, "cube(w)");
                let h = arg_f32!(args, "cube(h)");
                let l = arg_f32!(args, "cube(l)");
                let r = arg_u8!(args, "cube(r)");
                let g = arg_u8!(args, "cube(g)");
                let b = arg_u8!(args, "cube(b)");
                let a = arg_u8!(args, "cube(a)");
                unsafe { ffi::DrawCube(ffi::Vector3 { x, y, z }, w, h, l, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_cube_wires" => {
                let x = arg_f32!(args, "cube_wires(x)");
                let y = arg_f32!(args, "cube_wires(y)");
                let z = arg_f32!(args, "cube_wires(z)");
                let w = arg_f32!(args, "cube_wires(w)");
                let h = arg_f32!(args, "cube_wires(h)");
                let l = arg_f32!(args, "cube_wires(l)");
                let r = arg_u8!(args, "cube_wires(r)");
                let g = arg_u8!(args, "cube_wires(g)");
                let b = arg_u8!(args, "cube_wires(b)");
                let a = arg_u8!(args, "cube_wires(a)");
                unsafe { ffi::DrawCubeWires(ffi::Vector3 { x, y, z }, w, h, l, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_sphere" => {
                let x = arg_f32!(args, "sphere(x)");
                let y = arg_f32!(args, "sphere(y)");
                let z = arg_f32!(args, "sphere(z)");
                let rad = arg_f32!(args, "sphere(radius)");
                let r = arg_u8!(args, "sphere(r)");
                let g = arg_u8!(args, "sphere(g)");
                let b = arg_u8!(args, "sphere(b)");
                let a = arg_u8!(args, "sphere(a)");
                unsafe { ffi::DrawSphere(ffi::Vector3 { x, y, z }, rad, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_cylinder" => {
                let x = arg_f32!(args, "cylinder(x)");
                let y = arg_f32!(args, "cylinder(y)");
                let z = arg_f32!(args, "cylinder(z)");
                let r_top = arg_f32!(args, "cylinder(r_top)");
                let r_bot = arg_f32!(args, "cylinder(r_bot)");
                let h = arg_f32!(args, "cylinder(h)");
                let slices = arg_i32!(args, "cylinder(slices)");
                let r = arg_u8!(args, "cylinder(r)");
                let g = arg_u8!(args, "cylinder(g)");
                let b = arg_u8!(args, "cylinder(b)");
                let a = arg_u8!(args, "cylinder(a)");
                unsafe { ffi::DrawCylinder(ffi::Vector3 { x, y, z }, r_top, r_bot, h, slices, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_plane" => {
                let x = arg_f32!(args, "plane(x)");
                let y = arg_f32!(args, "plane(y)");
                let z = arg_f32!(args, "plane(z)");
                let w = arg_f32!(args, "plane(w)");
                let l = arg_f32!(args, "plane(l)");
                let r = arg_u8!(args, "plane(r)");
                let g = arg_u8!(args, "plane(g)");
                let b = arg_u8!(args, "plane(b)");
                let a = arg_u8!(args, "plane(a)");
                unsafe { ffi::DrawPlane(ffi::Vector3 { x, y, z }, ffi::Vector2 { x: w, y: l }, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_draw_line_3d" => {
                let x1 = arg_f32!(args, "line3d(x1)");
                let y1 = arg_f32!(args, "line3d(y1)");
                let z1 = arg_f32!(args, "line3d(z1)");
                let x2 = arg_f32!(args, "line3d(x2)");
                let y2 = arg_f32!(args, "line3d(y2)");
                let z2 = arg_f32!(args, "line3d(z2)");
                let r = arg_u8!(args, "line3d(r)");
                let g = arg_u8!(args, "line3d(g)");
                let b = arg_u8!(args, "line3d(b)");
                let a = arg_u8!(args, "line3d(a)");
                unsafe { ffi::DrawLine3D(ffi::Vector3 { x: x1, y: y1, z: z1 }, ffi::Vector3 { x: x2, y: y2, z: z2 }, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }

            // ===== ВВОД =====
            "rvenk_is_key_down"     => { let k = arg_i32!(args, "is_key_down"); Ok(Value::Bool(unsafe { ffi::IsKeyDown(k) })) }
            "rvenk_is_key_pressed"  => { let k = arg_i32!(args, "is_key_pressed"); Ok(Value::Bool(unsafe { ffi::IsKeyPressed(k) })) }
            "rvenk_is_key_released" => { let k = arg_i32!(args, "is_key_released"); Ok(Value::Bool(unsafe { ffi::IsKeyReleased(k) })) }
            "rvenk_is_key_up"       => { let k = arg_i32!(args, "is_key_up"); Ok(Value::Bool(unsafe { ffi::IsKeyUp(k) })) }
            "rvenk_get_mouse_x" => Ok(Value::Int(unsafe { ffi::GetMouseX() } as i64)),
            "rvenk_get_mouse_y" => Ok(Value::Int(unsafe { ffi::GetMouseY() } as i64)),
            "rvenk_get_mouse_position" => {
                let p = unsafe { ffi::GetMousePosition() };
                Ok(Value::Tuple(Rc::new(vec![Value::Float(p.x as f64), Value::Float(p.y as f64)])))
            }
            "rvenk_is_mouse_button_pressed" => { let b = arg_i32!(args, "mb_pressed"); Ok(Value::Bool(unsafe { ffi::IsMouseButtonPressed(b) })) }
            "rvenk_is_mouse_button_down"    => { let b = arg_i32!(args, "mb_down");    Ok(Value::Bool(unsafe { ffi::IsMouseButtonDown(b) })) }
            "rvenk_get_mouse_wheel_move"    => Ok(Value::Float(unsafe { ffi::GetMouseWheelMove() } as f64)),

            // ===== ТЕКСТУРЫ =====
            "rvenk_load_texture" => {
                let path = arg_str!(args, "load_texture(path)");
                let c = CString::new(path.as_str()).unwrap();
                let tex = unsafe { ffi::LoadTexture(c.as_ptr()) };
                Ok(Value::Int(box_texture(tex)))
            }
            "rvenk_draw_texture" => {
                let id = arg_id!(args, "draw_texture(id)");
                let x = arg_i32!(args, "draw_texture(x)");
                let y = arg_i32!(args, "draw_texture(y)");
                let r = arg_u8!(args, "draw_texture(r)");
                let g = arg_u8!(args, "draw_texture(g)");
                let b = arg_u8!(args, "draw_texture(b)");
                let a = arg_u8!(args, "draw_texture(a)");
                unsafe { ffi::DrawTexture(take_texture(id), x, y, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }

            // ===== ТЕКСТ / ШРИФТЫ =====
            "rvenk_draw_text" => {
                let text = arg_str!(args, "draw_text(text)");
                let c = CString::new(text.as_str()).unwrap();
                let x = arg_i32!(args, "draw_text(x)");
                let y = arg_i32!(args, "draw_text(y)");
                let fs = arg_i32!(args, "draw_text(font_size)");
                let r = arg_u8!(args, "draw_text(r)");
                let g = arg_u8!(args, "draw_text(g)");
                let b = arg_u8!(args, "draw_text(b)");
                let a = arg_u8!(args, "draw_text(a)");
                unsafe { ffi::DrawText(c.as_ptr(), x, y, fs, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_measure_text" => {
                let text = arg_str!(args, "measure_text(text)");
                let c = CString::new(text.as_str()).unwrap();
                let fs = arg_i32!(args, "measure_text(font_size)");
                Ok(Value::Int(unsafe { ffi::MeasureText(c.as_ptr(), fs) } as i64))
            }
            "rvenk_load_font" => {
                let path = arg_str!(args, "load_font(path)");
                let size = arg_i32!(args, "load_font(size)");
                let c = CString::new(path.as_str()).unwrap();
                let f = unsafe { ffi::LoadFontEx(c.as_ptr(), size, std::ptr::null_mut(), 0) };
                Ok(Value::Int(box_font(f)))
            }
            "rvenk_draw_text_ex" => {
                let fid = arg_id!(args, "draw_text_ex(font)");
                let text = arg_str!(args, "draw_text_ex(text)");
                let c = CString::new(text.as_str()).unwrap();
                let x = arg_f32!(args, "draw_text_ex(x)");
                let y = arg_f32!(args, "draw_text_ex(y)");
                let fs = arg_f32!(args, "draw_text_ex(font_size)");
                let sp = arg_f32!(args, "draw_text_ex(spacing)");
                let r = arg_u8!(args, "draw_text_ex(r)");
                let g = arg_u8!(args, "draw_text_ex(g)");
                let b = arg_u8!(args, "draw_text_ex(b)");
                let a = arg_u8!(args, "draw_text_ex(a)");
                unsafe {
                    ffi::DrawTextEx(take_font(fid), c.as_ptr(),
                        ffi::Vector2 { x, y }, fs, sp, ffi::Color { r, g, b, a });
                }
                Ok(Value::Null)
            }

            // ===== АУДИО =====
            "rvenk_init_audio_device"  => { unsafe { ffi::InitAudioDevice();  } Ok(Value::Null) }
            "rvenk_close_audio_device" => { unsafe { ffi::CloseAudioDevice(); } Ok(Value::Null) }
            "rvenk_load_sound" => {
                let path = arg_str!(args, "load_sound(path)");
                let c = CString::new(path.as_str()).unwrap();
                let s = unsafe { ffi::LoadSound(c.as_ptr()) };
                Ok(Value::Int(box_sound(s)))
            }
            "rvenk_play_sound" => {
                let id = arg_id!(args, "play_sound(id)");
                unsafe { ffi::PlaySound(take_sound(id)); }
                Ok(Value::Null)
            }
            "rvenk_stop_sound" => {
                let id = arg_id!(args, "stop_sound(id)");
                unsafe { ffi::StopSound(take_sound(id)); }
                Ok(Value::Null)
            }
            "rvenk_unload_sound" => {
                let id = arg_id!(args, "unload_sound(id)");
                unsafe {
                    let s = take_sound(id);
                    ffi::UnloadSound(s);
                    drop(Box::from_raw(id as *mut ffi::Sound));
                }
                Ok(Value::Null)
            }
            "rvenk_load_music_stream" => {
                let path = arg_str!(args, "load_music_stream(path)");
                let c = CString::new(path.as_str()).unwrap();
                let m = unsafe { ffi::LoadMusicStream(c.as_ptr()) };
                Ok(Value::Int(box_music(m)))
            }
            "rvenk_play_music_stream"   => { let id = arg_id!(args, "play_music");   unsafe { ffi::PlayMusicStream(take_music(id)); }   Ok(Value::Null) }
            "rvenk_update_music_stream" => { let id = arg_id!(args, "update_music"); unsafe { ffi::UpdateMusicStream(take_music(id)); } Ok(Value::Null) }
            "rvenk_stop_music_stream"   => { let id = arg_id!(args, "stop_music");   unsafe { ffi::StopMusicStream(take_music(id)); }   Ok(Value::Null) }

            // ===== 3D МОДЕЛИ =====
            "rvenk_load_model" => {
                let path = arg_str!(args, "load_model(path)");
                let c = CString::new(path.as_str()).unwrap();
                let m = unsafe { ffi::LoadModel(c.as_ptr()) };
                Ok(Value::Int(box_model(m)))
            }
            "rvenk_draw_model" => {
                let id = arg_id!(args, "draw_model(id)");
                let x = arg_f32!(args, "draw_model(x)");
                let y = arg_f32!(args, "draw_model(y)");
                let z = arg_f32!(args, "draw_model(z)");
                let sc = arg_f32!(args, "draw_model(scale)");
                let r = arg_u8!(args, "draw_model(r)");
                let g = arg_u8!(args, "draw_model(g)");
                let b = arg_u8!(args, "draw_model(b)");
                let a = arg_u8!(args, "draw_model(a)");
                unsafe { ffi::DrawModel(take_model(id), ffi::Vector3 { x, y, z }, sc, ffi::Color { r, g, b, a }); }
                Ok(Value::Null)
            }
            "rvenk_unload_model" => {
                let id = arg_id!(args, "unload_model(id)");
                unsafe {
                    let m = take_model(id);
                    ffi::UnloadModel(m);
                    drop(Box::from_raw(id as *mut ffi::Model));
                }
                Ok(Value::Null)
            }

            // ===== ШЕЙДЕРЫ =====
            "rvenk_load_shader" => {
                let vs = arg_str!(args, "load_shader(vs)");
                let fs = arg_str!(args, "load_shader(fs)");
                let c_vs = CString::new(vs.as_str()).unwrap();
                let c_fs = CString::new(fs.as_str()).unwrap();
                let s = unsafe { ffi::LoadShader(c_vs.as_ptr(), c_fs.as_ptr()) };
                Ok(Value::Int(box_shader(s)))
            }
            "rvenk_begin_shader_mode" => {
                let id = arg_id!(args, "begin_shader_mode(id)");
                unsafe { ffi::BeginShaderMode(take_shader(id)); }
                Ok(Value::Null)
            }
            "rvenk_end_shader_mode" => { unsafe { ffi::EndShaderMode(); } Ok(Value::Null) }

            // ===== МАТЕМАТИКА =====
            "rvenk_clamp" => {
                let v = arg_f32!(args, "clamp(v)");
                let mn = arg_f32!(args, "clamp(min)");
                let mx = arg_f32!(args, "clamp(max)");
                Ok(Value::Float(math_clamp(v, mn, mx) as f64))
            }
            "rvenk_lerp" => {
                let s = arg_f32!(args, "lerp(start)");
                let e = arg_f32!(args, "lerp(end)");
                let t = arg_f32!(args, "lerp(amount)");
                Ok(Value::Float(math_lerp(s, e, t) as f64))
            }

            // ===== ПРОЧЕЕ =====
            "rvenk_set_window_title" => {
                let t = arg_str!(args, "set_window_title");
                let c = CString::new(t.as_str()).unwrap();
                unsafe { ffi::SetWindowTitle(c.as_ptr()); }
                Ok(Value::Null)
            }
            "rvenk_open_url" => {
                let u = arg_str!(args, "open_url");
                let c = CString::new(u.as_str()).unwrap();
                unsafe { ffi::OpenURL(c.as_ptr()); }
                Ok(Value::Null)
            }
            "rvenk_get_clipboard_text" => {
                let s = unsafe {
                    let p = ffi::GetClipboardText();
                    if p.is_null() { String::new() }
                    else { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
                };
                Ok(Value::Str(Rc::from(s)))
            }
            "rvenk_set_clipboard_text" => {
                let t = arg_str!(args, "set_clipboard_text");
                let c = CString::new(t.as_str()).unwrap();
                unsafe { ffi::SetClipboardText(c.as_ptr()); }
                Ok(Value::Null)
            }
            "rvenk_is_window_ready"      => Ok(Value::Bool(unsafe { ffi::IsWindowReady()      })),
            "rvenk_is_window_fullscreen" => Ok(Value::Bool(unsafe { ffi::IsWindowFullscreen() })),
            "rvenk_set_window_state" => {
                let f = arg_i32!(args, "set_window_state");
                unsafe { ffi::SetWindowState(f as u32); }
                Ok(Value::Null)
            }
            "rvenk_clear_window_state" => {
                let f = arg_i32!(args, "clear_window_state");
                unsafe { ffi::ClearWindowState(f as u32); }
                Ok(Value::Null)
            }
            "rvenk_take_screenshot" => {
                let n = arg_str!(args, "take_screenshot");
                let c = CString::new(n.as_str()).unwrap();
                unsafe { ffi::TakeScreenshot(c.as_ptr()); }
                Ok(Value::Null)
            }

            // ===== ЕДИНСТВЕННЫЙ catch-all в самом конце =====
            other => Err(Signal::Runtime(format!("rvenk '{}' не реализован", other))),
        }
    }
}