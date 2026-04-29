use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Maps Rust target triples to Zig target triples for cross-compilation.
/// Returns `None` for native builds (where the Rust target matches the host).
fn detect_zig_target() -> Option<String> {
    let rust_target = env::var("TARGET").ok()?;

    let zig_target = match rust_target.as_str() {
        // Linux targets
        "x86_64-unknown-linux-gnu" => "x86_64-linux-gnu.2.17",
        "aarch64-unknown-linux-gnu" => "aarch64-linux-gnu.2.17",
        // Windows targets (MinGW)
        "x86_64-pc-windows-gnu" => "x86_64-windows-gnu",
        "aarch64-pc-windows-gnu" => "aarch64-windows-gnu",
        // macOS targets
        "aarch64-apple-darwin" => "aarch64-macos-gnu",
        // Native macOS builds — no target flag needed, let Zig detect
        "x86_64-apple-darwin" => return None,
        _ => return None,
    };

    Some(zig_target.to_string())
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ghostty_dir = PathBuf::from(&manifest_dir)
        .join("resource")
        .join("ghostty");

    // 通知 cargo 在源文件变更时才触发重编译
    println!("cargo:rerun-if-changed=resource/ghostty/build.zig");
    println!("cargo:rerun-if-changed=resource/ghostty/src/");
    println!("cargo:rerun-if-changed=resource/ghostty/include/ghostty/vt.h");

    // 1. 调用 zig 编译生成目标平台的 C-compatible 静态库
    let mut cmd = Command::new("zig");
    cmd.current_dir(&ghostty_dir).args(&[
        "build",
        "-Doptimize=ReleaseSafe",
        "-Dapp-runtime=none",         // 强制引用的方式现在由 main_c.zig 处理
        "-Demit-macos-app=false",    // 不要跑 xcodebuild 生成 App
        "-Demit-xcframework=false",  // 不要生成臃肿的框架
    ]);

    if let Some(target) = detect_zig_target() {
        cmd.arg(format!("-Dtarget={}", target));
        eprintln!("Cross-compiling ghostty for target: {}", target);
    } else {
        eprintln!("Building ghostty for native target");
    }

    let status = cmd
        .status()
        .expect("Failed to execute zig build");

    if !status.success() {
        panic!("Zig build failed for Ghostty");
    }

    // 2. 指向 zig-out 并绑定静态库链接
    let lib_path = ghostty_dir.join("zig-out").join("lib");
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    // libghostty-vt currently depends on some C++ symbols that are bundled in
    // libghostty.a in this checkout; link both, but keep the app/surface APIs
    // feature-gated at the Rust level to avoid runtime crashes.
    println!("cargo:rustc-link-lib=static=ghostty-vt");
    println!("cargo:rustc-link-lib=static=ghostty");

    #[cfg(target_os = "macos")]
    {
        // Keep link target in sync with Ghostty's default macOS target.
        println!("cargo:rustc-link-arg=-Wl,-platform_version,macos,13.0,13.0");
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=13.0");
        // Ensure test binaries can locate ghostty runtime libraries.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_path.display());

        println!("cargo:rustc-link-lib=c++"); // macOS 需要 libc++
    }

    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=stdc++"); // Linux 采用 libstdc++
    }

    #[cfg(target_os = "windows")]
    {
        // MinGW toolchain via Zig: link kernel32 and C++ stdlib
        println!("cargo:rustc-link-lib=dylib=kernel32");
        println!("cargo:rustc-link-lib=dylib=user32");
        println!("cargo:rustc-link-lib=dylib=gdi32");
        println!("cargo:rustc-link-lib=stdc++");
    }

    // 3. 构建 bindgen C-FFI wrapper (VT-only by default)
    eprintln!("Generating bindings from ghostty/vt.h...");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("ghostty_wrapper.h");
    std::fs::write(
        &wrapper_path,
        format!(
            "#include <stdbool.h>\n\
         #include <stdint.h>\n\
         #include \"{0}/include/ghostty/vt.h\"\n",
            ghostty_dir.display()
        ),
    )
    .unwrap();

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_str().unwrap())
        .clang_arg(format!("-I{}/include", ghostty_dir.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .expect("Unable to generate binding");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("ghostty_vt_bindings.rs"))
        .expect("Couldn't write bindings");

    // VT-only: no ghostty surface bindings.
}
