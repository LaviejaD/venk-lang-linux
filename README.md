# 🚀 Venk Language (Beta v0.0.2)

> A modern, high-performance programming language designed for 2D/3D game development and beyond.

![Version](https://img.shields.io/badge/version-0.0.2--beta-blue)
![Rust](https://img.shields.io/badge/backend-Rust-orange)
![License](https://img.shields.io/badge/license-MPL--2.0-green)

Venk is an ambitious project that combines an elegant, readable syntax with native execution speed. Built with Rust under the hood, Venk provides a seamless, out-of-the-box experience for developers who want to create games without the hassle of complex engine setups.

---

## 🌟 Why Venk?

*   ⚡ **Blazing Fast:** Powered by a native Rust interpreter. Upcoming LLVM backend will make it as fast as C++.
*   🎮 **Built-in 2D/3D Engine:** Create immersive worlds right away. Powered by `raylib`, Venk includes native support for 3D vectors, cameras, particles, and skeletal animations.
*   🌍 **Procedural Generation:** Built-in noise functions and `map_create` for generating massive worlds (RimWorld / WorldBox style).
*   🧠 **Modern & Expressive Syntax:** Forget about complex pointer arithmetic and memory leaks. Venk's syntax is clean, readable, and inspired by modern languages.
*   📦 **One-Click Compilation:** Bundle your entire game into a single, standalone `.exe` file using the built-in `build("game.venk")` command.
*   🛠️ **VIP Package Manager:** Easily share your code, import third-party modules, and manage dependencies.

---

## 🎥 Showcase


![3D Engine](assets/3d_screenshot.png)
*Built-in 3D engine with particles, lighting, and FPS controls.*

![2D Game](assets/2d_screenshot.png)
*Classic 2D arcade action running smoothly at 60 FPS.*

---

## 🚀 Quick Start

### Installation (Windows)
1. Go to the [Releases page](https://github.com/VBanna-coder/venk-lang/releases).
2. Download `venk.exe` and `raylib.dll`.
3. Place them in the same folder.
4. Open your terminal and run `venk.exe` to start the interactive console (REPL), or `venk.exe ide` to open the graphical IDE.

### Write your first program
Create a file named `hello.venk`:
```venk
fn main() {
    print("Hello, Venk!")
}
