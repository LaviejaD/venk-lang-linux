use std::collections::HashMap;
use std::cell::RefCell;

// 0: Deep Water, 1: Water, 2: Sand, 3: Grass, 4: Forest, 5: Rock, 6: Snow
pub struct WorldMap {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<i32>,
    pub elevation: Vec<f32>,
    pub moisture: Vec<f32>,
}

thread_local! {
    pub static WORLDS: RefCell<HashMap<i32, WorldMap>> = RefCell::new(HashMap::new());
    pub static NEXT_WORLD_ID: RefCell<i32> = RefCell::new(1);
}

// Простая функция хэширования для шума
fn hash(x: i32, y: i32, seed: i32) -> f32 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263)).wrapping_add(seed);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h & 0x7fffffff) as f32 / 0x7fffffff as f32
}

// Сглаживание (интерполяция)
fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }
fn smooth(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

// 2D Шум Перлина (Value Noise)
fn noise_2d(x: f32, y: f32, seed: i32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();

    let tl = hash(xi, yi, seed);
    let tr = hash(xi + 1, yi, seed);
    let bl = hash(xi, yi + 1, seed);
    let br = hash(xi + 1, yi + 1, seed);

    let u = smooth(xf);
    let v = smooth(yf);

    let top = lerp(tl, tr, u);
    let bot = lerp(bl, br, u);
    lerp(top, bot, v)
}

// Многослойный шум (FBM - для гор и реалистичного ландшафта)
fn fbm(x: f32, y: f32, seed: i32, octaves: i32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_value = 0.0;

    for _ in 0..octaves {
        value += noise_2d(x * frequency, y * frequency, seed) * amplitude;
        max_value += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value / max_value
}

impl WorldMap {
    pub fn new(width: usize, height: usize) -> Self {
        WorldMap {
            width,
            height,
            tiles: vec![0; width * height],
            elevation: vec![0.0; width * height],
            moisture: vec![0.0; width * height],
        }
    }

    pub fn generate(&mut self, seed: i32) {
        let scale = 0.05; // Размер континентов
        
        for y in 0..self.height {
            for x in 0..self.width {
                let fx = x as f32;
                let fy = y as f32;

                // 1. Высота (горы и океаны)
                let e = fbm(fx * scale, fy * scale, seed, 4);
                let idx = y * self.width + x;
                self.elevation[idx] = e;

                // 2. Влажность (леса и пустыни)
                let m = fbm(fx * scale * 1.5, fy * scale * 1.5, seed + 100, 3);
                self.moisture[idx] = m;

                // 3. Определяем биом
                self.tiles[idx] = if e < 0.3 {
                    0 // Глубокая вода
                } else if e < 0.4 {
                    1 // Мелководье
                } else if e < 0.45 {
                    2 // Песок
                } else if e > 0.8 {
                    6 // Снег
                } else if e > 0.7 {
                    5 // Камень
                } else {
                    if m > 0.6 {
                        4 // Лес
                    } else {
                        3 // Трава
                    }
                };
            }
        }
    }
}