use eframe::egui::{Key, Event, Modifiers};

/// 将 egui 的按键事件精确转换为 PTY (Xterm/VT100) 能够识别的字节流
pub fn translate_egui_event(event: &Event) -> Option<Vec<u8>> {
    match event {
        Event::Text(text) => {
            // 普通字符直接转 UTF-8
            Some(text.as_bytes().to_vec())
        }
        Event::Key { key, pressed: true, modifiers, .. } => {
            translate_egui_key(*key, *modifiers)
        }
        _ => None,
    }
}

fn translate_egui_key(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    // 优先处理组合键 (Ctrl + 字母)
    if modifiers.ctrl && !modifiers.shift && !modifiers.alt {
        let code = match key {
            Key::A => Some(b"\x01"),
            Key::B => Some(b"\x02"),
            Key::C => Some(b"\x03"), // SIGINT
            Key::D => Some(b"\x04"), // EOF
            Key::E => Some(b"\x05"),
            Key::F => Some(b"\x06"),
            Key::G => Some(b"\x07"),
            Key::H => Some(b"\x08"), // Backspace
            Key::I => Some(b"\x09"), // Tab
            Key::J => Some(b"\x0A"),
            Key::K => Some(b"\x0B"),
            Key::L => Some(b"\x0C"),
            Key::M => Some(b"\x0D"), // Enter
            Key::N => Some(b"\x0E"),
            Key::O => Some(b"\x0F"),
            Key::P => Some(b"\x10"),
            Key::Q => Some(b"\x11"),
            Key::R => Some(b"\x12"),
            Key::S => Some(b"\x13"),
            Key::T => Some(b"\x14"),
            Key::U => Some(b"\x15"),
            Key::V => Some(b"\x16"),
            Key::W => Some(b"\x17"),
            Key::X => Some(b"\x18"),
            Key::Y => Some(b"\x19"),
            Key::Z => Some(b"\x1A"),
            _ => None,
        };
        if let Some(c) = code {
            return Some(c.to_vec());
        }
    }

    // 处理特殊控制键、导航键、功能键
    let seq: &[u8] = match key {
        Key::Enter => b"\r",
        Key::Backspace => b"\x7F", // DEL is usually \x7F in modern terminals
        Key::Tab => b"\t",
        Key::Escape => b"\x1B",
        
        // 导航键 (CSI序列)
        Key::ArrowUp => b"\x1B[A",
        Key::ArrowDown => b"\x1B[B",
        Key::ArrowRight => b"\x1B[C",
        Key::ArrowLeft => b"\x1B[D",
        Key::Home => b"\x1B[H",
        Key::End => b"\x1B[F",
        Key::PageUp => b"\x1B[5~",
        Key::PageDown => b"\x1B[6~",
        Key::Insert => b"\x1B[2~",
        Key::Delete => b"\x1B[3~",

        // 功能键 (F1-F12 standard PC-Style)
        Key::F1 => b"\x1BOP",
        Key::F2 => b"\x1BOQ",
        Key::F3 => b"\x1BOR",
        Key::F4 => b"\x1BOS",
        Key::F5 => b"\x1B[15~",
        Key::F6 => b"\x1B[17~",
        Key::F7 => b"\x1B[18~",
        Key::F8 => b"\x1B[19~",
        Key::F9 => b"\x1B[20~",
        Key::F10 => b"\x1B[21~",
        Key::F11 => b"\x1B[23~",
        Key::F12 => b"\x1B[24~",

        _ => return None,
    };
    
    Some(seq.to_vec())
}
