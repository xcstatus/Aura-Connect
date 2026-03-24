use eframe::egui;
use rust_ssh::app::RustSshApp;

fn main() -> eframe::Result {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");
    let _guard = rt.enter();

    env_logger::init();

    // 创建一个常驻的异步运行时，不阻塞 eframe 的事件循环
    // let _rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    // let _enter = _rt.enter();

    let icon_data = {
        let image = image::load_from_memory(include_bytes!("../assets/icon/linux/256x256.png"))
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        egui::IconData {
            rgba: image.into_raw(),
            width,
            height,
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_decorations(true) // 为了显示 macOS 红绿灯，必须开启 decorations
            .with_transparent(true)  // 透明背景
            .with_fullsize_content_view(true) // 关键：使内容填充标题栏区域
            .with_titlebar_shown(false)       // 隐藏原生标题栏文本
            .with_icon(std::sync::Arc::new(icon_data)),
        ..Default::default()
    };

    eframe::run_native(
        "",
        options,
        Box::new(|cc| Ok(Box::new(RustSshApp::new(cc)))),
    )
}
