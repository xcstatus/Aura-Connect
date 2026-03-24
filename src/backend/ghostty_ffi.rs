#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// 动态载入 build.rs 从 ghostty.h 生成的裸 C-API 绑定
include!(concat!(env!("OUT_DIR"), "/ghostty_bindings.rs"));

/// 跨语言纹理句柄，例如 OpenGL的 texture_id 或者 WGPU的 Handle。
/// 由于这是我们针对 egui 的内部抽象，此处不属于 ghostty_bindings 的范围。
#[repr(C)]
pub struct GhosttyTextureHandle {
    pub texture_id: u64,
}

/// 包装真实 Ghostty 生命周期的 Rust 本地资源容器
pub struct GhosttyIntegration {
    pub app: ghostty_app_t,
    pub surface: ghostty_surface_t,
}

impl GhosttyIntegration {
    /// 初始化真实 Ghostty 引擎
    pub fn new() -> Option<Self> {
        // 由于需要复杂的 config 注入和上下文环境搭建（Task 4），
        // 暂且保持桥接未实例化，避免破坏应用启动
        None
    }

    pub fn resize(&mut self, _cols: u16, _rows: u16) {
        // unsafe { ghostty_surface_set_size(self.surface, cols as u32, rows as u32); }
    }

    pub fn write_ansi(&mut self, _data: &[u8]) {
        // unsafe { ghostty_surface_text(self.surface, data.as_ptr() as *const _, data.len()); }
    }

    pub fn get_texture_handle(&self) -> Option<GhosttyTextureHandle> {
        None
    }
}

impl Drop for GhosttyIntegration {
    fn drop(&mut self) {
        // unsafe { 
        //   ghostty_surface_free(self.surface); 
        //   ghostty_app_free(self.app);
        // }
    }
}
