use raylib::ffi;
use std::ffi::CString;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::cell::RefCell;
use crate::interp::{Interp, Signal, R};
use crate::value::*;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

// Структура для хранения моделей с анимациями
pub struct VeknModel {
    pub model: ffi::Model,
    pub anims: Vec<ffi::ModelAnimation>,
    pub current_anim: usize,
    pub anim_frame: f32,
}

// Структура для системы частиц
struct ParticleBatch {
    mesh: ffi::Mesh,
    material: ffi::Material,
    positions: Vec<ffi::Vector3>,
    speeds: Vec<f32>,
    y_max: f32,
}

// Используем thread_local! для 3D-ассетов (они не Send)
thread_local! {
    static MESHES: RefCell<HashMap<i32, ffi::Mesh>> = RefCell::new(HashMap::new());
    static MODELS: RefCell<HashMap<i32, VeknModel>> = RefCell::new(HashMap::new());
    static CAMERAS: RefCell<HashMap<i32, ffi::Camera3D>> = RefCell::new(HashMap::new());
    static BATCHES: RefCell<HashMap<i32, ParticleBatch>> = RefCell::new(HashMap::new());
}

static NEXT_ID: AtomicI32 = AtomicI32::new(1);

// Простой генератор случайных чисел для Rust (без внешних крейтов)
fn local_rand() -> f32 {
    let ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_nanos();
    (ns % 10000) as f32 / 10000.0
}

impl Interp {
    pub(crate) fn call_vek3d(&mut self, name: &str, mut args: Vec<Value>) -> R {
        macro_rules! num {
            ($v:expr, $name:expr) => {
                match $v {
                    Value::Int(n) => n as f32,
                    Value::Float(f) => f as f32,
                    other => return Err(Signal::Runtime(format!("{}: ожидалось число, получен {}", $name, other.type_name()))),
                }
            };
        }
        macro_rules! int {
            ($v:expr, $name:expr) => {
                match $v {
                    Value::Int(n) => n as i32,
                    other => return Err(Signal::Runtime(format!("{}: ожидалось число, получен {}", $name, other.type_name()))),
                }
            };
        }
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
            // МАТЕМАТИКА ВЕКТОРОВ (Godot-style)
            // ==========================================
            "vek3d_vec3" => {
                let x = num!(args.remove(0), name);
                let y = num!(args.remove(0), name);
                let z = num!(args.remove(0), name);
                Ok(make_vec3(x, y, z))
            }
            "vek3d_vec3_add" => {
                let (x1, y1, z1) = extract_vec3(&args.remove(0));
                let (x2, y2, z2) = extract_vec3(&args.remove(0));
                Ok(make_vec3(x1 + x2, y1 + y2, z1 + z2))
            }
            "vek3d_vec3_sub" => {
                let (x1, y1, z1) = extract_vec3(&args.remove(0));
                let (x2, y2, z2) = extract_vec3(&args.remove(0));
                Ok(make_vec3(x1 - x2, y1 - y2, z1 - z2))
            }
            "vek3d_vec3_mul" => {
                let (x, y, z) = extract_vec3(&args.remove(0));
                let scalar = num!(args.remove(0), name);
                Ok(make_vec3(x * scalar, y * scalar, z * scalar))
            }
            "vek3d_vec3_div" => {
                let (x, y, z) = extract_vec3(&args.remove(0));
                let scalar = num!(args.remove(0), name);
                if scalar == 0.0 { return Ok(make_vec3(0.0, 0.0, 0.0)); }
                Ok(make_vec3(x / scalar, y / scalar, z / scalar))
            }
            "vek3d_vec3_length" => {
                let (x, y, z) = extract_vec3(&args.remove(0));
                Ok(Value::Float(((x * x + y * y + z * z) as f64).sqrt()))
            }
            "vek3d_vec3_normalize" => {
                let (x, y, z) = extract_vec3(&args.remove(0));
                let len = (x * x + y * y + z * z).sqrt();
                if len == 0.0 { return Ok(make_vec3(0.0, 0.0, 0.0)); }
                Ok(make_vec3(x / len, y / len, z / len))
            }
            "vek3d_vec3_dot" => {
                let (x1, y1, z1) = extract_vec3(&args.remove(0));
                let (x2, y2, z2) = extract_vec3(&args.remove(0));
                Ok(Value::Float((x1 * x2 + y1 * y2 + z1 * z2) as f64))
            }
            "vek3d_vec3_cross" => {
                let (x1, y1, z1) = extract_vec3(&args.remove(0));
                let (x2, y2, z2) = extract_vec3(&args.remove(0));
                Ok(make_vec3(
                    y1 * z2 - z1 * y2,
                    z1 * x2 - x1 * z2,
                    x1 * y2 - y1 * x2,
                ))
            }
            "vek3d_vec3_distance" => {
                let (x1, y1, z1) = extract_vec3(&args.remove(0));
                let (x2, y2, z2) = extract_vec3(&args.remove(0));
                Ok(Value::Float((((x1 - x2).powi(2) + (y1 - y2).powi(2) + (z1 - z2).powi(2)) as f64).sqrt()))
            }
            "vek3d_vec3_lerp" => {
                let (x1, y1, z1) = extract_vec3(&args.remove(0));
                let (x2, y2, z2) = extract_vec3(&args.remove(0));
                let t = num!(args.remove(0), name);
                Ok(make_vec3(x1 + (x2 - x1) * t, y1 + (y2 - y1) * t, z1 + (z2 - z1) * t))
            }
            "vek3d_vec3_rotate_y" => {
                let (x, y, z) = extract_vec3(&args.remove(0));
                let angle = num!(args.remove(0), name).to_radians();
                let nx = x * angle.cos() - z * angle.sin();
                let nz = x * angle.sin() + z * angle.cos();
                Ok(make_vec3(nx, y, nz))
            }

            // ==========================================
            // ИИ И ГЕНЕРАЦИЯ МИРА
            // ==========================================
            "vek3d_npc_move_towards" => {
                let (x1, y1, z1) = extract_vec3(&args.remove(0));
                let (x2, y2, z2) = extract_vec3(&args.remove(0));
                let max_dist = num!(args.remove(0), name);
                let dx = x2 - x1; let dy = y2 - y1; let dz = z2 - z1;
                let dist = (dx*dx + dy*dy + dz*dz).sqrt();
                if dist <= max_dist || dist == 0.0 {
                    Ok(make_vec3(x2, y2, z2))
                } else {
                    let ratio = max_dist / dist;
                    Ok(make_vec3(x1 + dx * ratio, y1 + dy * ratio, z1 + dz * ratio))
                }
            }
            "vek3d_world_noise" => {
                let x = num!(args.remove(0), name) as i32;
                let y = num!(args.remove(0), name) as i32;
                let mut n = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
                n = (n ^ (n >> 13)).wrapping_mul(1274126177);
                let f = (n & 0x7fffffff) as f32 / 0x7fffffff as f32;
                Ok(Value::Float(f as f64))
            }

            // ==========================================
            // КАМЕРА (Godot-style)
            // ==========================================
            "vek3d_camera_create" => {
                let px = num!(args.remove(0), name); let py = num!(args.remove(0), name); let pz = num!(args.remove(0), name);
                let tx = num!(args.remove(0), name); let ty = num!(args.remove(0), name); let tz = num!(args.remove(0), name);
                let fovy = num!(args.remove(0), name);
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                CAMERAS.with(|c| {
                    c.borrow_mut().insert(id, ffi::Camera3D {
                        position: ffi::Vector3 { x: px, y: py, z: pz },
                        target: ffi::Vector3 { x: tx, y: ty, z: tz },
                        up: ffi::Vector3 { x: 0.0, y: 1.0, z: 0.0 },
                        fovy, projection: ffi::CameraProjection::CAMERA_PERSPECTIVE as i32,
                    });
                });
                Ok(Value::Int(id as i64))
            }
            "vek3d_camera_update_pos" => {
                let id = int!(args.remove(0), name);
                let px = num!(args.remove(0), name); let py = num!(args.remove(0), name); let pz = num!(args.remove(0), name);
                let tx = num!(args.remove(0), name); let ty = num!(args.remove(0), name); let tz = num!(args.remove(0), name);
                
                CAMERAS.with(|c| {
                    let mut cams = c.borrow_mut();
                    if let Some(cam) = cams.get_mut(&id) {
                        cam.position = ffi::Vector3 { x: px, y: py, z: pz };
                        cam.target = ffi::Vector3 { x: tx, y: ty, z: tz };
                    }
                });
                Ok(Value::Null)
            }
            "vek3d_camera_set_fov" => {
                let id = int!(args.remove(0), name);
                let fov = num!(args.remove(0), name);
                CAMERAS.with(|c| {
                    if let Some(cam) = c.borrow_mut().get_mut(&id) {
                        cam.fovy = fov;
                    }
                });
                Ok(Value::Null)
            }
            "vek3d_camera_get_forward" => {
                let id = int!(args.remove(0), name);
                let mut res = make_vec3(0.0, 0.0, 0.0);
                CAMERAS.with(|c| {
                    if let Some(cam) = c.borrow().get(&id) {
                        let dx = cam.target.x - cam.position.x;
                        let dy = cam.target.y - cam.position.y;
                        let dz = cam.target.z - cam.position.z;
                        let len = (dx*dx + dy*dy + dz*dz).sqrt();
                        if len > 0.0 { res = make_vec3(dx/len, dy/len, dz/len); }
                    }
                });
                Ok(res)
            }
            "vek3d_camera_update_fps" => {
                let id = int!(args.remove(0), name);
                let dt = num!(args.remove(0), name);
                let speed = num!(args.remove(0), name);
                let sensitivity = num!(args.remove(0), name);
                
                CAMERAS.with(|c| {
                    let mut cams = c.borrow_mut();
                    if let Some(cam) = cams.get_mut(&id) {
                        unsafe {
                            if ffi::IsMouseButtonDown(ffi::MouseButton::MOUSE_BUTTON_RIGHT as i32) {
                                let mouse_delta = ffi::GetMouseDelta();
                                let yaw = -mouse_delta.x * sensitivity;
                                let pitch = -mouse_delta.y * sensitivity;
                                
                                let target_diff = vec3_sub(cam.target, cam.position);
                                let rotated = vec3_rotate_axis(target_diff, ffi::Vector3 { x: 0.0, y: 1.0, z: 0.0 }, yaw * 0.0174533);
                                let right = vec3_normalize(vec3_cross(rotated, cam.up));
                                let rotated2 = vec3_rotate_axis(rotated, right, pitch * 0.0174533);
                                cam.target = vec3_add(cam.position, rotated2);
                            }
                            
                            let forward = vec3_normalize(vec3_sub(cam.target, cam.position));
                            let right = vec3_normalize(vec3_cross(forward, cam.up));
                            let mut move_vec = ffi::Vector3 { x: 0.0, y: 0.0, z: 0.0 };

                            if ffi::IsKeyDown(ffi::KeyboardKey::KEY_W as i32) { move_vec = vec3_add(move_vec, forward); }
                            if ffi::IsKeyDown(ffi::KeyboardKey::KEY_S as i32) { move_vec = vec3_sub(move_vec, forward); }
                            if ffi::IsKeyDown(ffi::KeyboardKey::KEY_D as i32) { move_vec = vec3_add(move_vec, right); }
                            if ffi::IsKeyDown(ffi::KeyboardKey::KEY_A as i32) { move_vec = vec3_sub(move_vec, right); }
                            
                            let delta = vec3_scale(move_vec, speed * dt);
                            cam.position = vec3_add(cam.position, delta);
                            cam.target = vec3_add(cam.target, delta);
                        }
                    }
                });
                Ok(Value::Null)
            }
            "vek3d_camera_begin" => {
                let id = int!(args.remove(0), name);
                CAMERAS.with(|c| {
                    if let Some(cam) = c.borrow().get(&id) {
                        unsafe { ffi::BeginMode3D(*cam); }
                    }
                });
                Ok(Value::Null)
            }
            "vek3d_camera_end" => {
                unsafe { ffi::EndMode3D(); }
                Ok(Value::Null)
            }

            // ==========================================
            // СИСТЕМА ЧАСТИЦ
            // ==========================================
            "vek3d_particles_create_cube" => {
                let size = num!(args.remove(0), name);
                let r = int!(args.remove(0), name) as u8;
                let g = int!(args.remove(0), name) as u8;
                let b = int!(args.remove(0), name) as u8;
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                unsafe {
                    let mesh = ffi::GenMeshCube(size, size, size);
                    let mut material = ffi::LoadMaterialDefault();
                    if !material.maps.is_null() {
                        (*material.maps).color = ffi::Color { r, g, b, a: 255 };
                    }
                    BATCHES.with(|bat| {
                        bat.borrow_mut().insert(id, ParticleBatch {
                            mesh,
                            material,
                            positions: Vec::new(),
                            speeds: Vec::new(),
                            y_max: 40.0,
                        });
                    });
                }
                Ok(Value::Int(id as i64))
            }
            "vek3d_particles_spawn" => {
                let id = int!(args.remove(0), name);
                let count = int!(args.remove(0), name) as usize;
                let w = num!(args.remove(0), name);
                let h = num!(args.remove(0), name);
                let d = num!(args.remove(0), name);
                
                BATCHES.with(|bat| {
                    if let Some(batch) = bat.borrow_mut().get_mut(&id) {
                        batch.y_max = h;
                        batch.positions.reserve(count);
                        batch.speeds.reserve(count);
                        for _ in 0..count {
                            batch.positions.push(ffi::Vector3 {
                                x: (local_rand() - 0.5) * w,
                                y: local_rand() * h,
                                z: (local_rand() - 0.5) * d,
                            });
                            batch.speeds.push(2.0 + local_rand() * 8.0);
                        }
                    }
                });
                Ok(Value::Null)
            }
            "vek3d_particles_update" => {
                let id = int!(args.remove(0), name);
                let dt = num!(args.remove(0), name);
                BATCHES.with(|bat| {
                    if let Some(batch) = bat.borrow_mut().get_mut(&id) {
                        for i in 0..batch.positions.len() {
                            batch.positions[i].y -= batch.speeds[i] * dt;
                            if batch.positions[i].y < 0.0 {
                                batch.positions[i].y = batch.y_max;
                            }
                        }
                    }
                });
                Ok(Value::Null)
            }
            "vek3d_particles_draw" => {
                let id = int!(args.remove(0), name);
                BATCHES.with(|bat| {
                    if let Some(batch) = bat.borrow().get(&id) {
                        unsafe {
                            let mut transforms: Vec<ffi::Matrix> = Vec::with_capacity(batch.positions.len());
                            for pos in &batch.positions {
                                transforms.push(ffi::Matrix {
                                    m0: 1.0, m1: 0.0, m2: 0.0, m3: 0.0,
                                    m4: 0.0, m5: 1.0, m6: 0.0, m7: 0.0,
                                    m8: 0.0, m9: 0.0, m10: 1.0, m11: 0.0,
                                    m12: pos.x, m13: pos.y, m14: pos.z, m15: 1.0,
                                });
                            }
                            for transform in transforms {
                                ffi::DrawMesh(batch.mesh, batch.material, transform);
                            }
                        }
                    }
                });
                Ok(Value::Null)
            }

            // ==========================================
            // МОДЕЛИ И АНИМАЦИИ (Godot/Unity-style)
            // ==========================================
            "vek3d_model_load" => {
                let path = s!(args.remove(0), name);
                let c_path = CString::new(path.as_str()).unwrap();
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                unsafe {
                    let model = ffi::LoadModel(c_path.as_ptr());
                    let anims = Vec::new();
                    MODELS.with(|m| { 
                        m.borrow_mut().insert(id, VeknModel { model, anims, current_anim: 0, anim_frame: 0.0 }); 
                    });
                }
                Ok(Value::Int(id as i64))
            }
            "vek3d_model_load_anim" => {
                let path = s!(args.remove(0), name);
                let anim_path = s!(args.remove(0), name);
                let c_path = CString::new(path.as_str()).unwrap();
                let c_anim = CString::new(anim_path.as_str()).unwrap();
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                unsafe {
                    let model = ffi::LoadModel(c_path.as_ptr());
                    
                    // ИСПРАВЛЕНИЕ: Ожидается *mut i32, а не u32
                    let mut anims_count: i32 = 0; 
                    let anims_ptr = ffi::LoadModelAnimations(c_anim.as_ptr(), &mut anims_count);
                    
                    let anims = if anims_ptr.is_null() { 
                        Vec::new() 
                    } else { 
                        std::slice::from_raw_parts(anims_ptr, anims_count as usize).to_vec() 
                    };
                    
                    MODELS.with(|m| { 
                        m.borrow_mut().insert(id, VeknModel { model, anims, current_anim: 0, anim_frame: 0.0 }); 
                    });
                }
                Ok(Value::Int(id as i64))
            }
            "vek3d_model_update_anim" => {
                let id = int!(args.remove(0), name);
                let dt = num!(args.remove(0), name);
                MODELS.with(|m| {
                    let mut models = m.borrow_mut();
                    if let Some(vm) = models.get_mut(&id) {
                        if !vm.anims.is_empty() {
                            vm.anim_frame += dt * 30.0;
                            unsafe {
                                let anim = &vm.anims[vm.current_anim];
                                // ИСПРАВЛЕНИЕ: приведение к i32
                                let frame = (vm.anim_frame as i32) % (anim.frameCount as i32);
                                ffi::UpdateModelAnimation(vm.model, *anim, frame as i32); 
                            }
                        }
                    }
                });
                Ok(Value::Null)
            }
            "vek3d_model_draw" => {
                let id = int!(args.remove(0), name);
                let x = num!(args.remove(0), name); let y = num!(args.remove(0), name); let z = num!(args.remove(0), name);
                let rot = num!(args.remove(0), name);
                let scale = num!(args.remove(0), name);
                let r = int!(args.remove(0), name) as u8; let g = int!(args.remove(0), name) as u8; let b = int!(args.remove(0), name) as u8;
                
                MODELS.with(|m| {
                    if let Some(vm) = m.borrow().get(&id) {
                        unsafe {
                            let pos = ffi::Vector3 { x, y, z };
                            let scale_v = ffi::Vector3 { x: scale, y: scale, z: scale };
                            let axis = ffi::Vector3 { x: 0.0, y: 1.0, z: 0.0 };
                            ffi::DrawModelEx(vm.model, pos, axis, rot, scale_v, ffi::Color { r, g, b, a: 255 });
                        }
                    }
                });
                Ok(Value::Null)
            }
            "vek3d_model_unload" => {
                let id = int!(args.remove(0), name);
                MODELS.with(|m| {
                    if let Some(vm) = m.borrow_mut().remove(&id) {
                        unsafe { ffi::UnloadModel(vm.model); }
                    }
                });
                Ok(Value::Null)
            }

            // ==========================================
            // МЕШИ (Примитивы)
            // ==========================================
            "vek3d_mesh_cube" => {
                let w = num!(args.remove(0), name); let h = num!(args.remove(0), name); let l = num!(args.remove(0), name);
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                unsafe {
                    let mesh = ffi::GenMeshCube(w, h, l);
                    MESHES.with(|m| { m.borrow_mut().insert(id, mesh); });
                }
                Ok(Value::Int(id as i64))
            }
            "vek3d_mesh_sphere" => {
                let r = num!(args.remove(0), name); let rings = int!(args.remove(0), name) as i32; let slices = int!(args.remove(0), name) as i32;
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                unsafe {
                    let mesh = ffi::GenMeshSphere(r, rings, slices);
                    MESHES.with(|m| { m.borrow_mut().insert(id, mesh); });
                }
                Ok(Value::Int(id as i64))
            }
            "vek3d_mesh_cylinder" => {
                let r_top = num!(args.remove(0), name); 
                let _r_bot = num!(args.remove(0), name); 
                let h = num!(args.remove(0), name); 
                let slices = int!(args.remove(0), name) as i32;
                let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
                unsafe {
                    // ИСПРАВЛЕНИЕ: передаем ровно 3 аргумента
                    let mesh = ffi::GenMeshCylinder(r_top, h, slices); 
                    MESHES.with(|m| { m.borrow_mut().insert(id, mesh); });
                }
                Ok(Value::Int(id as i64))
            }
            "vek3d_mesh_unload" => {
                let id = int!(args.remove(0), name);
                MESHES.with(|m| { m.borrow_mut().remove(&id); });
                Ok(Value::Null)
            }
            "vek3d_mesh_draw" => {
                let id = int!(args.remove(0), name);
                let x = num!(args.remove(0), name); let y = num!(args.remove(0), name); let z = num!(args.remove(0), name);
                let r = int!(args.remove(0), name) as u8; let g = int!(args.remove(0), name) as u8; let b = int!(args.remove(0), name) as u8;
                
                MESHES.with(|m| {
                    if let Some(mesh) = m.borrow().get(&id) {
                        unsafe {
                            let mut mat = ffi::LoadMaterialDefault();
                            if !mat.maps.is_null() { (*mat.maps).color = ffi::Color { r, g, b, a: 255 }; }
                            let transform = ffi::Matrix {
                                m0: 1.0, m1: 0.0, m2: 0.0, m3: 0.0,
                                m4: 0.0, m5: 1.0, m6: 0.0, m7: 0.0,
                                m8: 0.0, m9: 0.0, m10: 1.0, m11: 0.0,
                                m12: x, m13: y, m14: z, m15: 1.0,
                            };
                            ffi::DrawMesh(*mesh, mat, transform);
                            ffi::UnloadMaterial(mat);
                        }
                    }
                });
                Ok(Value::Null)
            }

            // ==========================================
            // ПРИМИТИВЫ И СТРУКТУРЫ
            // ==========================================
            "vek3d_draw_cube" => {
                let x = num!(args.remove(0), name); let y = num!(args.remove(0), name); let z = num!(args.remove(0), name);
                let w = num!(args.remove(0), name); let h = num!(args.remove(0), name); let l = num!(args.remove(0), name);
                let r = int!(args.remove(0), name) as u8; let g = int!(args.remove(0), name) as u8; let b = int!(args.remove(0), name) as u8;
                unsafe { ffi::DrawCube(ffi::Vector3 { x, y, z }, w, h, l, ffi::Color { r, g, b, a: 255 }); }
                Ok(Value::Null)
            }
            "vek3d_draw_cube_wires" => {
                let x = num!(args.remove(0), name); let y = num!(args.remove(0), name); let z = num!(args.remove(0), name);
                let w = num!(args.remove(0), name); let h = num!(args.remove(0), name); let l = num!(args.remove(0), name);
                let r = int!(args.remove(0), name) as u8; let g = int!(args.remove(0), name) as u8; let b = int!(args.remove(0), name) as u8;
                unsafe { ffi::DrawCubeWires(ffi::Vector3 { x, y, z }, w, h, l, ffi::Color { r, g, b, a: 255 }); }
                Ok(Value::Null)
            }
            "vek3d_draw_sphere" => {
                let x = num!(args.remove(0), name); let y = num!(args.remove(0), name); let z = num!(args.remove(0), name);
                let rad = num!(args.remove(0), name);
                let r = int!(args.remove(0), name) as u8; let g = int!(args.remove(0), name) as u8; let b = int!(args.remove(0), name) as u8;
                unsafe { ffi::DrawSphere(ffi::Vector3 { x, y, z }, rad, ffi::Color { r, g, b, a: 255 }); }
                Ok(Value::Null)
            }
            "vek3d_draw_grid" => {
                let slices = int!(args.remove(0), name); let spacing = num!(args.remove(0), name);
                unsafe { ffi::DrawGrid(slices, spacing); }
                Ok(Value::Null)
            }
            "vek3d_draw_tree" => {
                let x = num!(args.remove(0), name); let y = num!(args.remove(0), name); let z = num!(args.remove(0), name);
                unsafe {
                    ffi::DrawCylinder(ffi::Vector3 { x, y: y + 1.0, z }, 0.2, 0.2, 2.0, 8, ffi::Color { r: 139, g: 69, b: 19, a: 255 });
                    ffi::DrawSphere(ffi::Vector3 { x, y: y + 2.5, z }, 1.5, ffi::Color { r: 34, g: 139, b: 34, a: 255 });
                }
                Ok(Value::Null)
            }
            "vek3d_draw_house" => {
                let x = num!(args.remove(0), name); let y = num!(args.remove(0), name); let z = num!(args.remove(0), name);
                unsafe {
                    ffi::DrawCube(ffi::Vector3 { x, y: y + 1.0, z }, 2.0, 2.0, 2.0, ffi::Color { r: 200, g: 200, b: 200, a: 255 });
                    ffi::DrawCylinder(ffi::Vector3 { x, y: y + 2.0, z }, 0.0, 1.5, 1.5, 4, ffi::Color { r: 200, g: 50, b: 50, a: 255 });
                }
                Ok(Value::Null)
            }
            "vek3d_draw_npc" => {
                let x = num!(args.remove(0), name); let y = num!(args.remove(0), name); let z = num!(args.remove(0), name);
                let r = int!(args.remove(0), name) as u8; let g = int!(args.remove(0), name) as u8; let b = int!(args.remove(0), name) as u8;
                unsafe {
                    ffi::DrawCube(ffi::Vector3 { x, y: y + 0.5, z }, 0.8, 1.0, 0.4, ffi::Color { r, g, b, a: 255 });
                    ffi::DrawSphere(ffi::Vector3 { x, y: y + 1.3, z }, 0.3, ffi::Color { r: 255, g: 220, b: 177, a: 255 });
                }
                Ok(Value::Null)
            }

            // ==========================================
            // ФИЗИКА И КОЛЛИЗИИ (Godot-style)
            // ==========================================
            "vek3d_check_collision_spheres" => {
                let (x1, y1, z1) = extract_vec3(&args.remove(0));
                let r1 = num!(args.remove(0), name);
                let (x2, y2, z2) = extract_vec3(&args.remove(0));
                let r2 = num!(args.remove(0), name);
                unsafe {
                    let res = ffi::CheckCollisionSpheres(ffi::Vector3 { x: x1, y: y1, z: z1 }, r1, ffi::Vector3 { x: x2, y: y2, z: z2 }, r2);
                    Ok(Value::Bool(res))
                }
            }
            "vek3d_check_collision_boxes" => {
                let (min1x, min1y, min1z) = extract_vec3(&args.remove(0));
                let (max1x, max1y, max1z) = extract_vec3(&args.remove(0));
                let (min2x, min2y, min2z) = extract_vec3(&args.remove(0));
                let (max2x, max2y, max2z) = extract_vec3(&args.remove(0));
                unsafe {
                    let box1 = ffi::BoundingBox { min: ffi::Vector3 { x: min1x, y: min1y, z: min1z }, max: ffi::Vector3 { x: max1x, y: max1y, z: max1z } };
                    let box2 = ffi::BoundingBox { min: ffi::Vector3 { x: min2x, y: min2y, z: min2z }, max: ffi::Vector3 { x: max2x, y: max2y, z: max2z } };
                    let res = ffi::CheckCollisionBoxes(box1, box2);
                    Ok(Value::Bool(res))
                }
            }
            "vek3d_ray_sphere" => {
                let ox = num!(args.remove(0), name); let oy = num!(args.remove(0), name); let oz = num!(args.remove(0), name);
                let dx = num!(args.remove(0), name); let dy = num!(args.remove(0), name); let dz = num!(args.remove(0), name);
                let cx = num!(args.remove(0), name); let cy = num!(args.remove(0), name); let cz = num!(args.remove(0), name);
                let rad = num!(args.remove(0), name);
                unsafe {
                    let ray = ffi::Ray { position: ffi::Vector3 { x: ox, y: oy, z: oz }, direction: ffi::Vector3 { x: dx, y: dy, z: dz } };
                    let coll = ffi::GetRayCollisionSphere(ray, ffi::Vector3 { x: cx, y: cy, z: cz }, rad);
                    Ok(Value::Bool(coll.hit))
                }
            }
            "vek3d_ray_box" => {
                let ox = num!(args.remove(0), name); let oy = num!(args.remove(0), name); let oz = num!(args.remove(0), name);
                let dx = num!(args.remove(0), name); let dy = num!(args.remove(0), name); let dz = num!(args.remove(0), name);
                let min_x = num!(args.remove(0), name); let min_y = num!(args.remove(0), name); let min_z = num!(args.remove(0), name);
                let max_x = num!(args.remove(0), name); let max_y = num!(args.remove(0), name); let max_z = num!(args.remove(0), name);
                unsafe {
                    let ray = ffi::Ray { position: ffi::Vector3 { x: ox, y: oy, z: oz }, direction: ffi::Vector3 { x: dx, y: dy, z: dz } };
                    let bbox = ffi::BoundingBox { min: ffi::Vector3 { x: min_x, y: min_y, z: min_z }, max: ffi::Vector3 { x: max_x, y: max_y, z: max_z } };
                    let coll = ffi::GetRayCollisionBox(ray, bbox);
                    Ok(Value::Bool(coll.hit))
                }
            }

            other => Err(Signal::Runtime(format!("vek3d '{}' не реализован", other))),
        }
    }
}

// ==========================================
// ХЕЛПЕРЫ ДЛЯ ВЕКТОРОВ (Чистый Rust)
// ==========================================
fn make_vec3(x: f32, y: f32, z: f32) -> Value {
    let fields = vec![
        ("x".to_string(), Value::Float(x as f64)),
        ("y".to_string(), Value::Float(y as f64)),
        ("z".to_string(), Value::Float(z as f64)),
    ];
    Value::Struct(Rc::new("Vec3".to_string()), Rc::new(RefCell::new(fields)))
}

fn extract_vec3(v: &Value) -> (f32, f32, f32) {
    if let Value::Struct(_, fields) = v {
        let f = fields.borrow();
        let x = f.iter().find(|(k, _)| k == "x").map(|(_, v)| match v { Value::Float(n) => *n as f32, Value::Int(n) => *n as f32, _ => 0.0 }).unwrap_or(0.0);
        let y = f.iter().find(|(k, _)| k == "y").map(|(_, v)| match v { Value::Float(n) => *n as f32, Value::Int(n) => *n as f32, _ => 0.0 }).unwrap_or(0.0);
        let z = f.iter().find(|(k, _)| k == "z").map(|(_, v)| match v { Value::Float(n) => *n as f32, Value::Int(n) => *n as f32, _ => 0.0 }).unwrap_or(0.0);
        (x, y, z)
    } else {
        (0.0, 0.0, 0.0)
    }
}

// Математические операции над векторами
fn vec3_add(a: ffi::Vector3, b: ffi::Vector3) -> ffi::Vector3 { 
    ffi::Vector3 { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z } 
}
fn vec3_sub(a: ffi::Vector3, b: ffi::Vector3) -> ffi::Vector3 { 
    ffi::Vector3 { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z } 
}
fn vec3_scale(a: ffi::Vector3, scalar: f32) -> ffi::Vector3 { 
    ffi::Vector3 { x: a.x * scalar, y: a.y * scalar, z: a.z * scalar } 
}
fn vec3_normalize(a: ffi::Vector3) -> ffi::Vector3 {
    let len = (a.x * a.x + a.y * a.y + a.z * a.z).sqrt();
    if len == 0.0 { ffi::Vector3 { x: 0.0, y: 0.0, z: 0.0 } } 
    else { ffi::Vector3 { x: a.x / len, y: a.y / len, z: a.z / len } }
}
fn vec3_cross(a: ffi::Vector3, b: ffi::Vector3) -> ffi::Vector3 {
    ffi::Vector3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}
fn vec3_rotate_axis(v: ffi::Vector3, axis: ffi::Vector3, angle: f32) -> ffi::Vector3 {
    let k = vec3_normalize(axis);
    let cos_theta = angle.cos();
    let sin_theta = angle.sin();
    let cross = vec3_cross(k, v);
    let dot = k.x * v.x + k.y * v.y + k.z * v.z;
    
    ffi::Vector3 {
        x: v.x * cos_theta + cross.x * sin_theta + k.x * dot * (1.0 - cos_theta),
        y: v.y * cos_theta + cross.y * sin_theta + k.y * dot * (1.0 - cos_theta),
        z: v.z * cos_theta + cross.z * sin_theta + k.z * dot * (1.0 - cos_theta),
    }
}