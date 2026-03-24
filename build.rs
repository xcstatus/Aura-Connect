use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let ghostty_dir = PathBuf::from(&manifest_dir).join("resource").join("ghostty");

    // 通知 cargo 在源文件变更时才触发重编译
    println!("cargo:rerun-if-changed=resource/ghostty/build.zig");
    println!("cargo:rerun-if-changed=resource/ghostty/src/");
    println!("cargo:rerun-if-changed=resource/ghostty/include/ghostty.h");

    // 1. 调用 zig 编译生成本机 C-compatible 静态库
    let status = Command::new("zig")
        .current_dir(&ghostty_dir)
        .args(&[
            "build",
            "-Doptimize=ReleaseSafe",
            "-Dapp-runtime=none", // 无需单独可执行文件
            "-Demit-macos-app=false", // 不要跑 xcodebuild 生成 App
            "-Demit-xcframework=false", // 不要生成臃肿的框架
        ])
        .status()
        .expect("Failed to execute zig build");

    if !status.success() {
        panic!("Zig build failed for Ghostty");
    }

    // 2. 指向 zig-out 并绑定静态库链接
    let lib_path = ghostty_dir.join("zig-out").join("lib");
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=ghostty"); 
    // 也一起链接基础纯字元状态库
    println!("cargo:rustc-link-lib=static=ghostty-vt"); 

    // MacOS 必须要的框架绑定
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=CoreText");
    }

    // 3. 构建 bindgen C-FFI
    println!("cargo:warning=Generating bindings from ghostty.h...");
    let bindings = bindgen::Builder::default()
        .header(ghostty_dir.join("include").join("ghostty.h").to_str().unwrap())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .expect("Unable to generate binding");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("ghostty_bindings.rs"))
        .expect("Couldn't write bindings");
}
