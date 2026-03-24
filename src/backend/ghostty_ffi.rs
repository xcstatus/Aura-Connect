#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/ghostty_bindings.rs"));
}
pub use bindings::*;

/// 跨语言纹理句柄保留接口（暂未使用）
#[repr(C)]
pub struct GhosttyTextureHandle {
    pub texture_id: u64,
}

#[derive(Clone, PartialEq)]
pub struct SshCell {
    pub c: char,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
}

/// 包装真实 Ghostty 无头（Headless）VT生命周期的 Rust 本地资源容器
pub struct GhosttyIntegration {
    pub term: Option<GhosttyTerminal_ptr>,
    pub encoder: Option<GhosttyKeyEncoder_ptr>,
}

impl GhosttyIntegration {
    /// 初始化真实 Ghostty VT 引擎
    pub fn new() -> Option<Self> {
        let mut term: GhosttyTerminal_ptr = std::ptr::null_mut();
        // 设置一个默认的终端大小
        let options = GhosttyTerminalOptions {
            cols: 80,
            rows: 24,
            max_scrollback: 10000, 
        };
        
        let mut encoder: GhosttyKeyEncoder_ptr = std::ptr::null_mut();

        unsafe {
            if ghostty_terminal_new(std::ptr::null(), &mut term as *mut _, options) != GhosttyResult_GHOSTTY_SUCCESS {
                return None;
            }
            if ghostty_key_encoder_new(std::ptr::null(), &mut encoder as *mut _) != GhosttyResult_GHOSTTY_SUCCESS {
                ghostty_terminal_free(term);
                return None;
            }
        }

        Some(Self {
            term: Some(term),
            encoder: Some(encoder),
        })
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        if let Some(t) = self.term {
            unsafe { ghostty_terminal_resize(t, cols, rows); }
        }
    }

    pub fn write_ansi(&mut self, data: &[u8]) {
        if let Some(t) = self.term {
            unsafe { ghostty_terminal_vt_write(t, data.as_ptr(), data.len()); }
        }
    }

    pub fn get_texture_handle(&self) -> Option<GhosttyTextureHandle> {
        None
    }

    pub fn get_scrollbar(&self) -> Option<(u64, u64, u64)> {
        if let Some(t) = self.term {
            let mut scrollbar = GhosttyTerminalScrollbar { total: 0, offset: 0, len: 0 };
            unsafe {
                let res = ghostty_terminal_get(
                    t,
                    GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_SCROLLBAR,
                    &mut scrollbar as *mut _ as *mut std::os::raw::c_void,
                );
                if res == GhosttyResult_GHOSTTY_SUCCESS {
                    return Some((scrollbar.total, scrollbar.offset, scrollbar.len));
                }
            }
        }
        None
    }

    pub fn get_size(&self) -> (u16, u16) {
        if let Some(t) = self.term {
            let mut cols: u16 = 0;
            let mut rows: u16 = 0;
            unsafe {
                ghostty_terminal_get(t, GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_COLS, &mut cols as *mut _ as *mut _);
                ghostty_terminal_get(t, GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_ROWS, &mut rows as *mut _ as *mut _);
            }
            return (cols, rows);
        }
        (80, 24)
    }

    pub fn get_cursor(&self) -> (u16, u16) {
        if let Some(t) = self.term {
            let mut cx: u16 = 0;
            let mut cy: u16 = 0;
            unsafe {
                ghostty_terminal_get(t, GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_CURSOR_X, &mut cx as *mut _ as *mut _);
                ghostty_terminal_get(t, GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_CURSOR_Y, &mut cy as *mut _ as *mut _);
            }
            return (cx, cy);
        }
        (0, 0)
    }

    pub fn get_line(&self, y: u32, cols: u16) -> Vec<SshCell> {
        let mut line = Vec::with_capacity(cols as usize);
        if let Some(t) = self.term {
            for x in 0..cols {
                let point = GhosttyPoint {
                    tag: GhosttyPointTag_GHOSTTY_POINT_TAG_SCREEN,
                    value: GhosttyPointValue {
                        coordinate: GhosttyPointCoordinate { x, y }
                    }
                };
                let mut grid_ref = GhosttyGridRef { 
                    size: std::mem::size_of::<GhosttyGridRef>(), 
                    node: std::ptr::null_mut(), 
                    x: 0, 
                    y: 0 
                };
                
                let mut c_char = ' ';
                let mut fg_arr = [200, 200, 200];
                let mut bg_arr = [10, 10, 15];

                unsafe {
                    if ghostty_terminal_grid_ref(t, point, &mut grid_ref) == GhosttyResult_GHOSTTY_SUCCESS {
                        let mut graphemes = [0u32; 4];
                        let mut len: usize = 0;
                        if ghostty_grid_ref_graphemes(&grid_ref, graphemes.as_mut_ptr(), 4, &mut len) == GhosttyResult_GHOSTTY_SUCCESS {
                            if len > 0 { 
                                c_char = std::char::from_u32(graphemes[0]).unwrap_or(' ');
                            }
                        }

                        let mut style = GhosttyStyle {
                           size: std::mem::size_of::<GhosttyStyle>(),
                           fg_color: std::mem::zeroed(),
                           bg_color: std::mem::zeroed(),
                           underline_color: std::mem::zeroed(),
                           bold: false, italic: false, faint: false, blink: false,
                           inverse: false, invisible: false, strikethrough: false, overline: false,
                           underline: 0,
                        };
                        ghostty_grid_ref_style(&grid_ref, &mut style);
                        
                        if style.fg_color.tag == GhosttyStyleColorTag_GHOSTTY_STYLE_COLOR_RGB {
                            fg_arr = [style.fg_color.value.rgb.r, style.fg_color.value.rgb.g, style.fg_color.value.rgb.b];
                        }
                        if style.bg_color.tag == GhosttyStyleColorTag_GHOSTTY_STYLE_COLOR_RGB {
                            bg_arr = [style.bg_color.value.rgb.r, style.bg_color.value.rgb.g, style.bg_color.value.rgb.b];
                        }
                    }
                }
                line.push(SshCell { c: c_char, fg: fg_arr, bg: bg_arr });
            }
        }
        line
    }

    pub fn process_key(&self, action: u32, key: u32, mods: u16, text: Option<&str>) -> Option<Vec<u8>> {
        let (t, enc) = match (self.term, self.encoder) {
            (Some(term), Some(encoder)) => (term, encoder),
            _ => return None,
        };

        unsafe {
            let mut event: GhosttyKeyEvent_ptr = std::ptr::null_mut();
            if ghostty_key_event_new(std::ptr::null(), &mut event) != GhosttyResult_GHOSTTY_SUCCESS {
                return None;
            }

            ghostty_key_event_set_action(event, action);
            ghostty_key_event_set_key(event, key);
            ghostty_key_event_set_mods(event, mods);

            if let Some(utf8) = text {
                // Ensure the string is null-terminated or passed properly with its length
                ghostty_key_event_set_utf8(event, utf8.as_ptr() as *const i8, utf8.len());
            }

            // Sync Kitty flag options from actual parser state
            ghostty_key_encoder_setopt_from_terminal(enc, t);

            let mut out_len: usize = 0;
            // First run with NULL to fetch required len
            let res = ghostty_key_encoder_encode(enc, event, std::ptr::null_mut(), 0, &mut out_len);

            let mut output = None;

            if res == GhosttyResult_GHOSTTY_OUT_OF_SPACE && out_len > 0 {
                let mut buf = vec![0u8; out_len];
                let res2 = ghostty_key_encoder_encode(enc, event, buf.as_mut_ptr() as *mut i8, out_len, &mut out_len);
                if res2 == GhosttyResult_GHOSTTY_SUCCESS && out_len > 0 {
                    buf.truncate(out_len);
                    output = Some(buf);
                }
            } else if res == GhosttyResult_GHOSTTY_SUCCESS && out_len > 0 {
                // Should not happen with NULL buf
            }

            ghostty_key_event_free(event);
            output
        }
    }
}

impl Drop for GhosttyIntegration {
    fn drop(&mut self) {
        if let Some(e) = self.encoder {
            unsafe { ghostty_key_encoder_free(e) };
            self.encoder = None;
        }
        if let Some(t) = self.term {
            unsafe { ghostty_terminal_free(t) };
            self.term = None;
        }
    }
}
