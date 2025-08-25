use winit::event::{KeyboardInput, VirtualKeyCode};

pub struct InputMapper;

impl InputMapper {
    pub fn new() -> Self {
        Self
    }

    /// Maps keyboard input to string representation for script editor
    pub fn map_script_editor_key(&self, input: &KeyboardInput) -> String {
        if let Some(key_code) = input.virtual_keycode {
            if input.state == winit::event::ElementState::Pressed {
                return self.key_code_to_string(key_code, input);
            }
        }
        String::new()
    }

    fn key_code_to_string(&self, key_code: VirtualKeyCode, input: &KeyboardInput) -> String {
        match key_code {
            // Control keys
            VirtualKeyCode::Return => "Enter".to_string(),
            VirtualKeyCode::Back => "Backspace".to_string(),
            VirtualKeyCode::Delete => {
                if input.modifiers.shift() {
                    "Shift+Delete".to_string()
                } else {
                    "Delete".to_string()
                }
            },
            VirtualKeyCode::Escape => "Escape".to_string(),
            VirtualKeyCode::Tab => "Tab".to_string(),
            VirtualKeyCode::Home => "Home".to_string(),
            VirtualKeyCode::End => "End".to_string(),
            
            // Arrow keys
            VirtualKeyCode::Up => "ArrowUp".to_string(),
            VirtualKeyCode::Down => "ArrowDown".to_string(),
            VirtualKeyCode::Left => "ArrowLeft".to_string(),
            VirtualKeyCode::Right => "ArrowRight".to_string(),
            
            // Ctrl combinations
            VirtualKeyCode::S if input.modifiers.ctrl() => "Ctrl+S".to_string(),
            VirtualKeyCode::Z if input.modifiers.ctrl() => "Ctrl+Z".to_string(),
            VirtualKeyCode::Y if input.modifiers.ctrl() => "Ctrl+Y".to_string(),
            VirtualKeyCode::A if input.modifiers.ctrl() => "Ctrl+A".to_string(),
            VirtualKeyCode::C if input.modifiers.ctrl() => "Ctrl+C".to_string(),
            VirtualKeyCode::V if input.modifiers.ctrl() => "Ctrl+V".to_string(),
            
            // Number keys with shift support
            VirtualKeyCode::Key1 => {
                if input.modifiers.shift() { "!".to_string() } else { "1".to_string() }
            },
            VirtualKeyCode::Key2 => {
                if input.modifiers.shift() { "@".to_string() } else { "2".to_string() }
            },
            VirtualKeyCode::Key3 => {
                if input.modifiers.shift() { "#".to_string() } else { "3".to_string() }
            },
            VirtualKeyCode::Key4 => {
                if input.modifiers.shift() { "$".to_string() } else { "4".to_string() }
            },
            VirtualKeyCode::Key5 => {
                if input.modifiers.shift() { "%".to_string() } else { "5".to_string() }
            },
            VirtualKeyCode::Key6 => {
                if input.modifiers.shift() { "^".to_string() } else { "6".to_string() }
            },
            VirtualKeyCode::Key7 => {
                if input.modifiers.shift() { "&".to_string() } else { "7".to_string() }
            },
            VirtualKeyCode::Key8 => {
                if input.modifiers.shift() { "*".to_string() } else { "8".to_string() }
            },
            VirtualKeyCode::Key9 => {
                if input.modifiers.shift() { "(".to_string() } else { "9".to_string() }
            },
            VirtualKeyCode::Key0 => {
                if input.modifiers.shift() { ")".to_string() } else { "0".to_string() }
            },
            
            // Special characters with shift support
            VirtualKeyCode::Minus => {
                if input.modifiers.shift() { "_".to_string() } else { "-".to_string() }
            },
            VirtualKeyCode::Equals => {
                if input.modifiers.shift() { "+".to_string() } else { "=".to_string() }
            },
            VirtualKeyCode::LBracket => {
                if input.modifiers.shift() { "{".to_string() } else { "[".to_string() }
            },
            VirtualKeyCode::RBracket => {
                if input.modifiers.shift() { "}".to_string() } else { "]".to_string() }
            },
            VirtualKeyCode::Backslash => {
                if input.modifiers.shift() { "|".to_string() } else { "\\".to_string() }
            },
            VirtualKeyCode::Semicolon => {
                if input.modifiers.shift() { ":".to_string() } else { ";".to_string() }
            },
            VirtualKeyCode::Apostrophe => {
                if input.modifiers.shift() { "\"".to_string() } else { "'".to_string() }
            },
            VirtualKeyCode::Comma => {
                if input.modifiers.shift() { "<".to_string() } else { ",".to_string() }
            },
            VirtualKeyCode::Period => {
                if input.modifiers.shift() { ">".to_string() } else { ".".to_string() }
            },
            VirtualKeyCode::Slash => {
                if input.modifiers.shift() { "?".to_string() } else { "/".to_string() }
            },
            VirtualKeyCode::Grave => {
                if input.modifiers.shift() { "~".to_string() } else { "`".to_string() }
            },
            
            // Letter keys (handle shift for uppercase)
            VirtualKeyCode::A if !input.modifiers.ctrl() => {
                if input.modifiers.shift() { "A".to_string() } else { "a".to_string() }
            },
            VirtualKeyCode::B => {
                if input.modifiers.shift() { "B".to_string() } else { "b".to_string() }
            },
            VirtualKeyCode::C if !input.modifiers.ctrl() => {
                if input.modifiers.shift() { "C".to_string() } else { "c".to_string() }
            },
            VirtualKeyCode::D => {
                if input.modifiers.shift() { "D".to_string() } else { "d".to_string() }
            },
            VirtualKeyCode::E => {
                if input.modifiers.shift() { "E".to_string() } else { "e".to_string() }
            },
            VirtualKeyCode::F => {
                if input.modifiers.shift() { "F".to_string() } else { "f".to_string() }
            },
            VirtualKeyCode::G => {
                if input.modifiers.shift() { "G".to_string() } else { "g".to_string() }
            },
            VirtualKeyCode::H => {
                if input.modifiers.shift() { "H".to_string() } else { "h".to_string() }
            },
            VirtualKeyCode::I => {
                if input.modifiers.shift() { "I".to_string() } else { "i".to_string() }
            },
            VirtualKeyCode::J => {
                if input.modifiers.shift() { "J".to_string() } else { "j".to_string() }
            },
            VirtualKeyCode::K => {
                if input.modifiers.shift() { "K".to_string() } else { "k".to_string() }
            },
            VirtualKeyCode::L => {
                if input.modifiers.shift() { "L".to_string() } else { "l".to_string() }
            },
            VirtualKeyCode::M => {
                if input.modifiers.shift() { "M".to_string() } else { "m".to_string() }
            },
            VirtualKeyCode::N => {
                if input.modifiers.shift() { "N".to_string() } else { "n".to_string() }
            },
            VirtualKeyCode::O => {
                if input.modifiers.shift() { "O".to_string() } else { "o".to_string() }
            },
            VirtualKeyCode::P => {
                if input.modifiers.shift() { "P".to_string() } else { "p".to_string() }
            },
            VirtualKeyCode::Q => {
                if input.modifiers.shift() { "Q".to_string() } else { "q".to_string() }
            },
            VirtualKeyCode::R => {
                if input.modifiers.shift() { "R".to_string() } else { "r".to_string() }
            },
            VirtualKeyCode::S if !input.modifiers.ctrl() => {
                if input.modifiers.shift() { "S".to_string() } else { "s".to_string() }
            },
            VirtualKeyCode::T => {
                if input.modifiers.shift() { "T".to_string() } else { "t".to_string() }
            },
            VirtualKeyCode::U => {
                if input.modifiers.shift() { "U".to_string() } else { "u".to_string() }
            },
            VirtualKeyCode::V if !input.modifiers.ctrl() => {
                if input.modifiers.shift() { "V".to_string() } else { "v".to_string() }
            },
            VirtualKeyCode::W => {
                if input.modifiers.shift() { "W".to_string() } else { "w".to_string() }
            },
            VirtualKeyCode::X => {
                if input.modifiers.shift() { "X".to_string() } else { "x".to_string() }
            },
            VirtualKeyCode::Y if !input.modifiers.ctrl() => {
                if input.modifiers.shift() { "Y".to_string() } else { "y".to_string() }
            },
            VirtualKeyCode::Z if !input.modifiers.ctrl() => {
                if input.modifiers.shift() { "Z".to_string() } else { "z".to_string() }
            },
            
            // Space
            VirtualKeyCode::Space => "Space".to_string(),
            
            // Default case
            _ => String::new(),
        }
    }
}