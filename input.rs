use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};
use std::collections::HashSet;

pub struct InputHandler {
    pressed_keys: HashSet<VirtualKeyCode>,
    command_buffer: String,
    cursor_moved: bool,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            command_buffer: String::new(),
            cursor_moved: false,
        }
    }

    pub fn handle_keyboard_input(&mut self, input: &KeyboardInput, script_editor_active: bool) -> InputAction {
        if let Some(key_code) = input.virtual_keycode {
            match input.state {
                ElementState::Pressed => {
                    self.pressed_keys.insert(key_code);
                    self.handle_key_press(key_code, script_editor_active)
                }
                ElementState::Released => {
                    self.pressed_keys.remove(&key_code);
                    InputAction::None
                }
            }
        } else {
            InputAction::None
        }
    }

    fn handle_key_press(&mut self, key_code: VirtualKeyCode, script_editor_active: bool) -> InputAction {
        let shift_pressed = self.pressed_keys.contains(&VirtualKeyCode::LShift) 
                     || self.pressed_keys.contains(&VirtualKeyCode::RShift);
        
        match key_code {
            VirtualKeyCode::Up => {
                if shift_pressed && !script_editor_active {
                    // Shift+Up for command history (only when script editor is not active)
                    InputAction::HistoryPrevious
                } else {
                    self.cursor_moved = true;
                    InputAction::MoveCursor(0, -1)
                }
            }
            VirtualKeyCode::Down => {
                if shift_pressed && !script_editor_active {
                    // Shift+Down for command history (only when script editor is not active)
                    InputAction::HistoryNext
                } else {
                    self.cursor_moved = true;
                    InputAction::MoveCursor(0, 1)
                }
            }
            VirtualKeyCode::Left => {
                self.cursor_moved = true;
                InputAction::MoveCursor(-1, 0)
            }
            VirtualKeyCode::Right => {
                self.cursor_moved = true;
                InputAction::MoveCursor(1, 0)
            }
            
            // Remove the space toggle - let it be handled as text input
            // VirtualKeyCode::Space => InputAction::ToggleCell,
            
            // Enter to execute command
            VirtualKeyCode::Return => {
                if !self.command_buffer.is_empty() {
                    let command = self.command_buffer.clone();
                    self.command_buffer.clear();
                    InputAction::ExecuteCommand(command)
                } else {
                    InputAction::None
                }
            }
            
            // Backspace to delete character
            VirtualKeyCode::Back => {
                self.command_buffer.pop();
                InputAction::UpdateCommandBuffer(self.command_buffer.clone())
            }
            
            // Escape to clear command buffer
            VirtualKeyCode::Escape => {
                self.command_buffer.clear();
                InputAction::UpdateCommandBuffer(self.command_buffer.clone())
            }
            
            // Handle text input for commands (including space now)
            _ => {
                // Don't process shift keys as text input
                if key_code == VirtualKeyCode::LShift || key_code == VirtualKeyCode::RShift {
                    return InputAction::None;
                }
                
                if let Some(character) = self.key_code_to_char(key_code) {
                    self.command_buffer.push(character);
                    InputAction::UpdateCommandBufferAndResetHistory(self.command_buffer.clone())
                } else {
                    InputAction::None
                }
            }
        }
    }

    fn key_code_to_char(&self, key_code: VirtualKeyCode) -> Option<char> {
        let shift_pressed = self.pressed_keys.contains(&VirtualKeyCode::LShift) 
                         || self.pressed_keys.contains(&VirtualKeyCode::RShift);
        
        match key_code {
            // Letters
            VirtualKeyCode::A => Some(if shift_pressed { 'A' } else { 'a' }),
            VirtualKeyCode::B => Some(if shift_pressed { 'B' } else { 'b' }),
            VirtualKeyCode::C => Some(if shift_pressed { 'C' } else { 'c' }),
            VirtualKeyCode::D => Some(if shift_pressed { 'D' } else { 'd' }),
            VirtualKeyCode::E => Some(if shift_pressed { 'E' } else { 'e' }),
            VirtualKeyCode::F => Some(if shift_pressed { 'F' } else { 'f' }),
            VirtualKeyCode::G => Some(if shift_pressed { 'G' } else { 'g' }),
            VirtualKeyCode::H => Some(if shift_pressed { 'H' } else { 'h' }),
            VirtualKeyCode::I => Some(if shift_pressed { 'I' } else { 'i' }),
            VirtualKeyCode::J => Some(if shift_pressed { 'J' } else { 'j' }),
            VirtualKeyCode::K => Some(if shift_pressed { 'K' } else { 'k' }),
            VirtualKeyCode::L => Some(if shift_pressed { 'L' } else { 'l' }),
            VirtualKeyCode::M => Some(if shift_pressed { 'M' } else { 'm' }),
            VirtualKeyCode::N => Some(if shift_pressed { 'N' } else { 'n' }),
            VirtualKeyCode::O => Some(if shift_pressed { 'O' } else { 'o' }),
            VirtualKeyCode::P => Some(if shift_pressed { 'P' } else { 'p' }),
            VirtualKeyCode::Q => Some(if shift_pressed { 'Q' } else { 'q' }),
            VirtualKeyCode::R => Some(if shift_pressed { 'R' } else { 'r' }),
            VirtualKeyCode::S => Some(if shift_pressed { 'S' } else { 's' }),
            VirtualKeyCode::T => Some(if shift_pressed { 'T' } else { 't' }),
            VirtualKeyCode::U => Some(if shift_pressed { 'U' } else { 'u' }),
            VirtualKeyCode::V => Some(if shift_pressed { 'V' } else { 'v' }),
            VirtualKeyCode::W => Some(if shift_pressed { 'W' } else { 'w' }),
            VirtualKeyCode::X => Some(if shift_pressed { 'X' } else { 'x' }),
            VirtualKeyCode::Y => Some(if shift_pressed { 'Y' } else { 'y' }),
            VirtualKeyCode::Z => Some(if shift_pressed { 'Z' } else { 'z' }),
            
            // Numbers
            VirtualKeyCode::Key0 => Some(if shift_pressed { ')' } else { '0' }),
            VirtualKeyCode::Key1 => Some(if shift_pressed { '!' } else { '1' }),
            VirtualKeyCode::Key2 => Some(if shift_pressed { '@' } else { '2' }),
            VirtualKeyCode::Key3 => Some(if shift_pressed { '#' } else { '3' }),
            VirtualKeyCode::Key4 => Some(if shift_pressed { '$' } else { '4' }),
            VirtualKeyCode::Key5 => Some(if shift_pressed { '%' } else { '5' }),
            VirtualKeyCode::Key6 => Some(if shift_pressed { '^' } else { '6' }),
            VirtualKeyCode::Key7 => Some(if shift_pressed { '&' } else { '7' }),
            VirtualKeyCode::Key8 => Some(if shift_pressed { '*' } else { '8' }),
            VirtualKeyCode::Key9 => Some(if shift_pressed { '(' } else { '9' }),
            
            // Special characters
            VirtualKeyCode::Comma => Some(if shift_pressed { '<' } else { ',' }),
            VirtualKeyCode::Period => Some(if shift_pressed { '>' } else { '.' }),
            VirtualKeyCode::Semicolon => Some(if shift_pressed { ':' } else { ';' }),
            VirtualKeyCode::Apostrophe => Some(if shift_pressed { '"' } else { '\'' }),
            VirtualKeyCode::LBracket => Some(if shift_pressed { '{' } else { '[' }),
            VirtualKeyCode::RBracket => Some(if shift_pressed { '}' } else { ']' }),
            VirtualKeyCode::Backslash => Some(if shift_pressed { '|' } else { '\\' }),
            VirtualKeyCode::Slash => Some(if shift_pressed { '?' } else { '/' }),
            VirtualKeyCode::Equals => Some(if shift_pressed { '+' } else { '=' }),
            VirtualKeyCode::Minus => Some(if shift_pressed { '_' } else { '-' }),
            VirtualKeyCode::Grave => Some(if shift_pressed { '~' } else { '`' }),
            
            // Add space character support
            VirtualKeyCode::Space => Some(' '),
            
            _ => None,
        }
    }

    pub fn get_command_buffer(&self) -> &str {
        &self.command_buffer
    }
    
    pub fn set_command_buffer(&mut self, buffer: String) {
        self.command_buffer = buffer;
    }

    pub fn clear_cursor_moved(&mut self) {
        self.cursor_moved = false;
    }

    pub fn cursor_moved(&self) -> bool {
        self.cursor_moved
    }
}

#[derive(Debug, Clone)]
pub enum InputAction {
    None,
    MoveCursor(i32, i32),
    ToggleCell,
    ExecuteCommand(String),
    UpdateCommandBuffer(String),
    UpdateCommandBufferAndResetHistory(String), 
    HistoryPrevious,
    HistoryNext,
}