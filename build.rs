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

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=CoreText");
        println!("cargo:rustc-link-lib=c++"); // macOS 需要 libc++
    }

    #[cfg(not(target_os = "macos"))]
    {
        println!("cargo:rustc-link-lib=stdc++"); // Linux 采用 libstdc++
    }

    // 3. 构建 bindgen C-FFI wrapper
    println!("cargo:warning=Generating bindings from comprehensive ghostty headers...");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_path.join("ghostty_wrapper.h");
    std::fs::write(&wrapper_path, format!(
        "#include \"{0}/include/ghostty/vt/terminal.h\"\n\
         #include \"{0}/include/ghostty/vt/screen.h\"\n\
         #include \"{0}/include/ghostty/vt/key.h\"\n\
         #include \"{0}/include/ghostty/vt/mouse.h\"\n\
         #include \"{0}/include/ghostty/vt/formatter.h\"\n\
         #include \"{0}/include/ghostty/vt/render.h\"\n",
        ghostty_dir.display()
    )).unwrap();

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_str().unwrap())
        .clang_arg(format!("-I{}/include", ghostty_dir.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .layout_tests(false)
        .generate()
        .expect("Unable to generate binding");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("ghostty_bindings.rs"))
        .expect("Couldn't write bindings");
}
