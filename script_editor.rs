use std::collections::HashSet;
use std::time::Instant;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Clone)]
pub struct SyntaxToken {
    pub text: String,
    pub token_type: TokenType,
    pub start_col: usize,
    pub end_col: usize,
}

#[derive(Clone, PartialEq)]
pub enum TokenType {
    Keyword,
    Function,
    String,
    Number,
    Comment,
    Operator,
    Identifier,
    Color,
    Normal,
}

#[derive(Clone)]
struct EditorState {
    lines: Vec<String>,
    current_line: usize,
    current_col: usize,
    scroll_offset: usize,
}

pub struct ScriptEditor {
    lines: Vec<String>,
    current_line: usize,
    current_col: usize,
    target_object_id: u32,
    is_active: bool,
    status_message: String,
    clipboard: String,
    undo_stack: Vec<EditorState>,
    redo_stack: Vec<EditorState>,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    theme: Theme,
    scroll_offset: usize,
    viewport_height: usize,
    cursor_blink_timer: Instant,
    cursor_visible: bool,
    syntax_tokens: Vec<Vec<SyntaxToken>>,
    current_filename: Option<String>,
    is_modified: bool,
    filename_input: String,
    is_editing_filename: bool,
    filename_cursor_pos: usize,
    is_memory_script: bool,
    dirty_lines: HashSet<usize>,
    max_line_width: usize,
    next_script_id: u32, // Add this field for script ID generation
}

impl ScriptEditor {
    pub fn new(target_object_id: u32, existing_script: Option<String>) -> Self {
        let mut editor = Self {
            lines: vec![String::new()],
            current_line: 0,
            current_col: 0,
            target_object_id,
            is_active: true,
            status_message: String::new(),
            clipboard: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selection_start: None,
            selection_end: None,
            theme: Theme::Dark,
            scroll_offset: 0,
            viewport_height: 5,
            cursor_blink_timer: Instant::now(),
            cursor_visible: true,
            syntax_tokens: Vec::new(),
            current_filename: None,
            is_modified: false,
            filename_input: "untitled".to_string(),
            is_editing_filename: false,
            filename_cursor_pos: 0,
            is_memory_script: false,
            dirty_lines: HashSet::new(),
            max_line_width: 41,
            next_script_id: 1, // Initialize script ID counter
        };
        
        if let Some(script) = existing_script {
            editor.lines = script.lines().map(|s| s.to_string()).collect();
            if editor.lines.is_empty() {
                editor.lines.push(String::new());
            }
        }
        
        editor.update_syntax_highlighting();
        editor
    }
    
    pub fn new_memory_script(existing_script: Option<String>) -> Self {
        let mut editor = Self::new(0, existing_script);
        editor.is_memory_script = true;
        editor
    }
    
    pub fn new_with_file(filename: String, existing_script: Option<String>) -> Self {
        let mut editor = Self::new(0, existing_script);
        let base_filename = if filename.ends_with(".cant") {
            filename[..filename.len() - 5].to_string()
        } else {
            filename
        };
        editor.current_filename = Some(base_filename.clone());
        editor.filename_input = base_filename;
        editor
    }
    
    pub fn handle_key(&mut self, key: &str) -> bool {
        self.update_cursor_blink();
        
        if self.is_editing_filename {
            return self.handle_filename_key(key);
        }
        
        // Save state for undo before making changes
        if !matches!(key, "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "Home" | "End" | "PageUp" | "PageDown") {
            self.save_state();
        }
        
        match key {
            "Enter" => self.new_line(),
            "Backspace" => self.backspace(),
            "Delete" => self.delete(),
            "Escape" => { self.is_active = false; false },
            "Ctrl+S" => {
                if self.current_filename.is_none() || self.is_editing_filename {
                    // First press or already editing - enter filename editing mode
                    self.is_editing_filename = true;
                    self.filename_cursor_pos = self.filename_input.len();
                    true
                } else {
                    // Second press - save the file
                    self.save_to_file()
                }
            },
            "Ctrl+Shift+S" => self.save_as_file(),
            "Ctrl+O" => self.open_file(),
            "Ctrl+Z" => self.undo(),
            "Ctrl+Y" => self.redo(),
            "Ctrl+A" => self.select_all(),
            "Ctrl+C" => self.copy(),
            "Ctrl+V" => self.paste(),
            "Tab" => self.insert_char('\t'),
            "ArrowUp" => self.move_cursor_up(false),
            "ArrowDown" => self.move_cursor_down(false),
            "ArrowLeft" => self.move_cursor_left(false),
            "ArrowRight" => self.move_cursor_right(false),
            "Shift+ArrowUp" => self.move_cursor_up(true),
            "Shift+ArrowDown" => self.move_cursor_down(true),
            "Shift+ArrowLeft" => self.move_cursor_left(true),
            "Shift+ArrowRight" => self.move_cursor_right(true),
            "Home" => self.move_to_line_start(false),
            "End" => self.move_to_line_end(false),
            "Shift+Home" => self.move_to_line_start(true),
            "Shift+End" => self.move_to_line_end(true),
            "Space" => self.insert_char(' '),
            _ => {
                if key.len() == 1 {
                    let ch = key.chars().next().unwrap();
                    if ch.is_ascii() && !ch.is_control() {
                        return self.insert_char(ch);
                    }
                }
                false
            }
        }
    }
    
    fn save_state(&mut self) {
        let state = EditorState {
            lines: self.lines.clone(),
            current_line: self.current_line,
            current_col: self.current_col,
            scroll_offset: self.scroll_offset,
        };
        self.undo_stack.push(state);
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }
    
    fn undo(&mut self) -> bool {
        if let Some(state) = self.undo_stack.pop() {
            let current_state = EditorState {
                lines: self.lines.clone(),
                current_line: self.current_line,
                current_col: self.current_col,
                scroll_offset: self.scroll_offset,
            };
            self.redo_stack.push(current_state);
            
            self.lines = state.lines;
            self.current_line = state.current_line;
            self.current_col = state.current_col;
            self.scroll_offset = state.scroll_offset;
            self.update_syntax_highlighting();
            true
        } else {
            false
        }
    }
    
    fn redo(&mut self) -> bool {
        if let Some(state) = self.redo_stack.pop() {
            let current_state = EditorState {
                lines: self.lines.clone(),
                current_line: self.current_line,
                current_col: self.current_col,
                scroll_offset: self.scroll_offset,
            };
            self.undo_stack.push(current_state);
            
            self.lines = state.lines;
            self.current_line = state.current_line;
            self.current_col = state.current_col;
            self.scroll_offset = state.scroll_offset;
            self.update_syntax_highlighting();
            true
        } else {
            false
        }
    }
    
    fn ensure_line_exists(&mut self, line: usize) {
        while self.lines.len() <= line {
            self.lines.push(String::new());
        }
    }
    
    fn move_cursor_up(&mut self, extend_selection: bool) -> bool {
        if extend_selection {
            self.start_selection_if_needed();
        } else {
            self.clear_selection();
        }
        
        if self.current_line > 0 {
            self.current_line -= 1;
            let line_len = if self.current_line < self.lines.len() {
                self.lines[self.current_line].len()
            } else {
                0
            };
            self.current_col = self.current_col.min(line_len);
            
            if extend_selection {
                self.update_selection_end();
            }
        }
        
        self.ensure_cursor_visible();
        true
    }
    
    fn move_cursor_down(&mut self, extend_selection: bool) -> bool {
        if extend_selection {
            self.start_selection_if_needed();
        } else {
            self.clear_selection();
        }
        
        if self.current_line + 1 < self.lines.len() {
            self.current_line += 1;
            let line_len = self.lines[self.current_line].len();
            self.current_col = self.current_col.min(line_len);
            
            if extend_selection {
                self.update_selection_end();
            }
        }
        
        self.ensure_cursor_visible();
        true
    }
    
    fn move_cursor_left(&mut self, extend_selection: bool) -> bool {
        if extend_selection {
            self.start_selection_if_needed();
        } else {
            self.clear_selection();
        }
        
        if self.current_col > 0 {
            self.current_col -= 1;
        } else if self.current_line > 0 {
            self.current_line -= 1;
            self.current_col = if self.current_line < self.lines.len() {
                self.lines[self.current_line].len()
            } else {
                0
            };
        }
        
        if extend_selection {
            self.update_selection_end();
        }
        
        true
    }
    
    fn move_cursor_right(&mut self, extend_selection: bool) -> bool {
        if extend_selection {
            self.start_selection_if_needed();
        } else {
            self.clear_selection();
        }
        
        let current_line_len = if self.current_line < self.lines.len() {
            self.lines[self.current_line].len()
        } else {
            0
        };
        
        if self.current_col < current_line_len {
            self.current_col += 1;
        } else {
            self.current_line += 1;
            self.current_col = 0;
            self.ensure_line_exists(self.current_line);
        }
        
        if extend_selection {
            self.update_selection_end();
        }
        
        true
    }
    
    fn move_to_line_start(&mut self, extend_selection: bool) -> bool {
        if extend_selection {
            self.start_selection_if_needed();
        } else {
            self.clear_selection();
        }
        
        self.current_col = 0;
        
        if extend_selection {
            self.update_selection_end();
        }
        
        true
    }
    
    fn move_to_line_end(&mut self, extend_selection: bool) -> bool {
        if extend_selection {
            self.start_selection_if_needed();
        } else {
            self.clear_selection();
        }
        
        if self.current_line < self.lines.len() {
            self.current_col = self.lines[self.current_line].len();
        }
        
        if extend_selection {
            self.update_selection_end();
        }
        
        true
    }
    
    fn start_selection_if_needed(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some((self.current_line, self.current_col));
        }
    }
    
    fn update_selection_end(&mut self) {
        self.selection_end = Some((self.current_line, self.current_col));
    }
    
    fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }
    
    fn insert_char(&mut self, c: char) -> bool {
        self.ensure_line_exists(self.current_line);
        
        // Handle tab as 4 spaces
        if c == '\t' {
            for _ in 0..4 {
                if self.current_col < self.max_line_width {
                    self.lines[self.current_line].insert(self.current_col, ' ');
                    self.current_col += 1;
                }
            }
        } else {
            // Only insert if within line width limit
            if self.current_col < self.max_line_width {
                self.lines[self.current_line].insert(self.current_col, c);
                self.current_col += 1;
            }
        }
        
        self.dirty_lines.insert(self.current_line);
        self.is_modified = true;
        self.clear_selection();
        self.update_syntax_highlighting_incremental();
        true
    }
    
    fn new_line(&mut self) -> bool {
        self.ensure_line_exists(self.current_line);
        
        let current_line_content = self.lines[self.current_line].clone();
        let (left, right) = current_line_content.split_at(self.current_col);
        
        self.lines[self.current_line] = left.to_string();
        self.lines.insert(self.current_line + 1, right.to_string());
        
        self.current_line += 1;
        self.current_col = 0;
        
        self.dirty_lines.insert(self.current_line - 1);
        self.dirty_lines.insert(self.current_line);
        self.is_modified = true;
        self.clear_selection();
        self.update_syntax_highlighting_incremental();
        true
    }
    
    fn backspace(&mut self) -> bool {
        if self.current_col > 0 {
            self.current_col -= 1;
            if self.current_line < self.lines.len() {
                self.lines[self.current_line].remove(self.current_col);
                self.dirty_lines.insert(self.current_line);
            }
        } else if self.current_line > 0 {
            let current_line_content = if self.current_line < self.lines.len() {
                self.lines.remove(self.current_line)
            } else {
                String::new()
            };
            
            self.current_line -= 1;
            self.current_col = self.lines[self.current_line].len();
            self.lines[self.current_line].push_str(&current_line_content);
            self.dirty_lines.insert(self.current_line);
        }
        
        self.is_modified = true;
        self.clear_selection();
        self.update_syntax_highlighting_incremental();
        true
    }
    
    fn delete(&mut self) -> bool {
        if self.current_line < self.lines.len() {
            if self.current_col < self.lines[self.current_line].len() {
                self.lines[self.current_line].remove(self.current_col);
                self.dirty_lines.insert(self.current_line);
            } else if self.current_line + 1 < self.lines.len() {
                let next_line = self.lines.remove(self.current_line + 1);
                self.lines[self.current_line].push_str(&next_line);
                self.dirty_lines.insert(self.current_line);
            }
        }
        
        self.is_modified = true;
        self.clear_selection();
        self.update_syntax_highlighting_incremental();
        true
    }
    
    fn select_all(&mut self) -> bool {
        self.selection_start = Some((0, 0));
        if !self.lines.is_empty() {
            let last_line = self.lines.len() - 1;
            let last_col = self.lines[last_line].len();
            self.selection_end = Some((last_line, last_col));
        }
        true
    }
    
    fn copy(&mut self) -> bool {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start_line, start_col) = start;
            let (end_line, end_col) = end;
            
            let mut content = String::new();
            
            if start_line == end_line {
                if start_line < self.lines.len() {
                    let line = &self.lines[start_line];
                    let start_idx = start_col.min(line.len());
                    let end_idx = end_col.min(line.len());
                    content = line[start_idx..end_idx].to_string();
                }
            } else {
                for line_idx in start_line..=end_line.min(self.lines.len() - 1) {
                    let line = &self.lines[line_idx];
                    if line_idx == start_line {
                        content.push_str(&line[start_col.min(line.len())..]);
                    } else if line_idx == end_line {
                        content.push_str(&line[..end_col.min(line.len())]);
                    } else {
                        content.push_str(line);
                    }
                    if line_idx < end_line {
                        content.push('\n');
                    }
                }
            }
            
            self.clipboard = content;
        }
        true
    }
    
    fn paste(&mut self) -> bool {
        if !self.clipboard.is_empty() {
            let clipboard_content = self.clipboard.clone();
            for c in clipboard_content.chars() {
                if c == '\n' {
                    self.new_line();
                } else {
                    self.insert_char(c);
                }
            }
        }
        true
    }

    pub fn update_cursor_blink(&mut self) {
        let elapsed = self.cursor_blink_timer.elapsed();
        if elapsed.as_millis() > 500 {
            self.cursor_visible = !self.cursor_visible;
            self.cursor_blink_timer = Instant::now();
        }
    }

    pub fn get_display_lines(&self) -> Vec<String> {
        let mut display_lines = Vec::new();
        
        // Add status line at the top with script info
        let filename_display = if self.is_editing_filename {
            &self.filename_input
        } else {
            self.current_filename.as_deref().unwrap_or("untitled")
        };
        
        let status_line = format!(
            "Script: {} Line {} Col {}",
            filename_display,
            self.current_line + 1,
            self.current_col + 1
        );
        display_lines.push(status_line);
        
        let start_line = self.scroll_offset;
        let end_line = (start_line + self.viewport_height).min(self.lines.len());
        
        for i in start_line..end_line {
            let mut line = if i < self.lines.len() {
                // Use the original line without syntax tags
                self.lines[i].clone()
            } else {
                String::new()
            };
            
            // Ensure line is exactly max_line_width characters
            if line.len() < self.max_line_width {
                line.push_str(&" ".repeat(self.max_line_width - line.len()));
            } else if line.len() > self.max_line_width {
                line.truncate(self.max_line_width);
            }
            
            // Add cursor if this is the current line and cursor is visible
            if i == self.current_line && self.cursor_visible && self.is_active {
                if self.current_col < line.len() {
                    line.replace_range(self.current_col..self.current_col+1, "█");
                } else if self.current_col == line.len() {
                    line.push('█');
                }
            }
            
            display_lines.push(line);
        }
        
        // Fill remaining viewport with empty lines
        while display_lines.len() < self.viewport_height + 1 {
            display_lines.push(" ".repeat(self.max_line_width));
        }
        
        display_lines
    }

    fn format_line_with_syntax(&self, line_index: usize) -> String {
        if line_index >= self.lines.len() {
            return String::new();
        }
        
        // Return the original line without tags - highlighting should be handled by the renderer
        self.lines[line_index].clone()
    }

    // Add new method to get syntax tokens for a line
    fn get_line_tokens(&self, line_index: usize) -> Vec<SyntaxToken> {
        if line_index >= self.lines.len() {
            return Vec::new();
        }
        
        let line = &self.lines[line_index];
        let mut tokens = Vec::new();
        let mut chars = line.chars().peekable();
        let mut pos = 0;
        
        while let Some(&c) = chars.peek() {
            let start_pos = pos;
            
            match c {
                // String literals
                '"' => {
                    let mut text = String::new();
                    text.push(chars.next().unwrap());
                    pos += 1;
                    
                    while let Some(&next_ch) = chars.peek() {
                        let ch = chars.next().unwrap();
                        text.push(ch);
                        pos += 1;
                        if ch == '"' {
                            break;
                        }
                    }
                    
                    tokens.push(SyntaxToken {
                        text,
                        token_type: TokenType::String,
                        start_col: start_pos,
                        end_col: pos,
                    });
                }
                '\'' => {
                    let mut text = String::new();
                    text.push(chars.next().unwrap());
                    pos += 1;
                    
                    while let Some(&next_ch) = chars.peek() {
                        let ch = chars.next().unwrap();
                        text.push(ch);
                        pos += 1;
                        if ch == '\'' {
                            break;
                        }
                    }
                    
                    tokens.push(SyntaxToken {
                        text,
                        token_type: TokenType::String,
                        start_col: start_pos,
                        end_col: pos,
                    });
                }
                // Comments
                '/' if chars.clone().nth(1) == Some('/') => {
                    let mut text = String::new();
                    while let Some(ch) = chars.next() {
                        text.push(ch);
                        pos += 1;
                    }
                    
                    tokens.push(SyntaxToken {
                        text,
                        token_type: TokenType::Comment,
                        start_col: start_pos,
                        end_col: pos,
                    });
                }
                // Numbers
                c if c.is_ascii_digit() => {
                    let mut text = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_ascii_digit() || next_ch == '.' {
                            text.push(chars.next().unwrap());
                            pos += 1;
                        } else {
                            break;
                        }
                    }
                    
                    tokens.push(SyntaxToken {
                        text,
                        token_type: TokenType::Number,
                        start_col: start_pos,
                        end_col: pos,
                    });
                }
                // Keywords and identifiers
                c if c.is_alphabetic() || c == '_' => {
                    let mut text = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            text.push(chars.next().unwrap());
                            pos += 1;
                        } else {
                            break;
                        }
                    }
                    
                    let token_type = match text.as_str() {
                        "if" | "else" | "while" | "for" | "function" | "return" |
                        "true" | "false" | "null" | "let" | "const" | "var" |
                        "hit" | "create" | "move" | "destroy" | "when" | "then" |
                        "pause" | "stop" | "clear" | "label" | "script" |
                        "run" | "verbose" | "hits" | "balls" | "squares" | "cursor" | "self" |
                        "left" | "right" | "up" | "down" | "up-left" | "left-up" |
                        "up-right" | "right-up" | "down-left" | "left-down" |
                        "down-right" | "right-down" => TokenType::Keyword,
                        "red" | "blue" | "green" | "yellow" | "orange" | "purple" |
                        "pink" | "cyan" | "magenta" | "white" | "black" | "gray" |
                        "brown" | "lime" => TokenType::Color,
                        _ => {
                            if chars.peek() == Some(&'(') {
                                TokenType::Function
                            } else {
                                TokenType::Identifier
                            }
                        }
                    };
                    
                    tokens.push(SyntaxToken {
                        text,
                        token_type,
                        start_col: start_pos,
                        end_col: pos,
                    });
                }
                // Operators
                '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' => {
                    let text = chars.next().unwrap().to_string();
                    pos += 1;
                    
                    tokens.push(SyntaxToken {
                        text,
                        token_type: TokenType::Operator,
                        start_col: start_pos,
                        end_col: pos,
                    });
                }
                // Everything else
                _ => {
                    let text = chars.next().unwrap().to_string();
                    pos += 1;
                    
                    tokens.push(SyntaxToken {
                        text,
                        token_type: TokenType::Normal,
                        start_col: start_pos,
                        end_col: pos,
                    });
                }
            }
        }
        
        tokens
    }

    pub fn update_syntax_highlighting(&mut self) {
        self.syntax_tokens.clear();
        for i in 0..self.lines.len() {
            self.syntax_tokens.push(self.tokenize_line(i));
        }
    }

    pub fn update_syntax_highlighting_incremental(&mut self) {
        for &line_idx in &self.dirty_lines {
            if line_idx < self.lines.len() {
                while self.syntax_tokens.len() <= line_idx {
                    self.syntax_tokens.push(Vec::new());
                }
                self.syntax_tokens[line_idx] = self.tokenize_line(line_idx);
            }
        }
        self.dirty_lines.clear();
    }

    fn tokenize_line(&self, line_idx: usize) -> Vec<SyntaxToken> {
        let mut tokens = Vec::new();
        if line_idx >= self.lines.len() {
            return tokens;
        }
        
        let line = &self.lines[line_idx];
        let mut chars = line.char_indices().peekable();
        
        while let Some((start_col, ch)) = chars.next() {
            let end_col = start_col + 1;
            
            match ch {
                '"' | '\'' => {
                    tokens.push(SyntaxToken {
                        text: ch.to_string(),
                        token_type: TokenType::String,
                        start_col,
                        end_col,
                    });
                }
                c if c.is_ascii_digit() => {
                    tokens.push(SyntaxToken {
                        text: ch.to_string(),
                        token_type: TokenType::Number,
                        start_col,
                        end_col,
                    });
                }
                '/' if chars.peek().map(|(_, ch)| *ch) == Some('/') => {
                    tokens.push(SyntaxToken {
                        text: ch.to_string(),
                        token_type: TokenType::Comment,
                        start_col,
                        end_col,
                    });
                }
                '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' => {
                    tokens.push(SyntaxToken {
                        text: ch.to_string(),
                        token_type: TokenType::Operator,
                        start_col,
                        end_col,
                    });
                }
                c if c.is_alphabetic() || c == '_' => {
                    let mut word = String::new();
                    word.push(ch);
                    
                    while let Some(&(_, next_ch)) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            word.push(chars.next().unwrap().1);
                        } else {
                            break;
                        }
                    }
                    
                    let token_type = match word.as_str() {
                        "if" | "else" | "while" | "for" | "function" | "return" |
                        "true" | "false" | "null" | "let" | "const" | "var" |
                        "hit" | "create" | "move" | "destroy" | "when" | "then" |
                        "pause" | "stop" | "clear" | "label" | "script" |
                        "run" | "verbose" | "hits" | "balls" | "squares" | "cursor" | "self" |
                        "left" | "right" | "up" | "down" | "up-left" | "left-up" |
                        "up-right" | "right-up" | "down-left" | "left-down" |
                        "down-right" | "right-down" => TokenType::Keyword,
                        
                        "red" | "blue" | "green" | "yellow" | "orange" | "purple" |
                        "pink" | "cyan" | "magenta" | "white" | "black" | "gray" |
                        "brown" | "lime" => TokenType::Color,
                        
                        _ => {
                            if chars.peek().map(|(_, ch)| *ch) == Some('(') {
                                TokenType::Function
                            } else {
                                TokenType::Identifier
                            }
                        }
                    };
                    tokens.push(SyntaxToken {
                        text: word,
                        token_type,
                        start_col,
                        end_col,
                    });
                }
                _ => {
                    tokens.push(SyntaxToken {
                        text: ch.to_string(),
                        token_type: TokenType::Normal,
                        start_col,
                        end_col,
                    });
                }
            }
        }
        
        tokens
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn get_target_object_id(&self) -> u32 {
        self.target_object_id
    }

    pub fn get_script_content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn handle_filename_key(&mut self, key: &str) -> bool {
        if !self.is_editing_filename {
            return false;
        }
    
        match key {
            "Ctrl+S" => {
                // Save with current filename when Ctrl+S is pressed during editing
                self.is_editing_filename = false;
                if !self.filename_input.is_empty() {
                    self.current_filename = Some(self.filename_input.clone());
                    
                    // Check if it's a lib.* file - save to memory instead of disk
                    if self.filename_input.starts_with("lib.") {
                        self.is_memory_script = true;
                        self.save_to_memory()
                    } else {
                        self.is_memory_script = false;
                        self.save_to_file()
                    }
                } else {
                    false
                }
            },
            "Enter" => {
                self.is_editing_filename = false;
                if !self.filename_input.is_empty() {
                    self.current_filename = Some(self.filename_input.clone());
                    
                    // Check if it's a lib.* file - save to memory instead of disk
                    if self.filename_input.starts_with("lib.") {
                        self.is_memory_script = true;
                        self.save_to_memory();
                    } else {
                        self.is_memory_script = false;
                        self.save_to_file();
                    }
                }
                true
            }
            "Escape" => {
                self.is_editing_filename = false;
                self.filename_input.clear();
                true
            }
            "Backspace" => {
                if self.filename_cursor_pos > 0 {
                    // Special case: if filename is "untitled" and we're backspacing, clear entire filename
                    if self.filename_input == "untitled" {
                        self.filename_input.clear();
                        self.filename_cursor_pos = 0;
                    } else {
                        self.filename_input.remove(self.filename_cursor_pos - 1);
                        self.filename_cursor_pos -= 1;
                    }
                }
                true
            }
            "Delete" => {
                if self.filename_cursor_pos < self.filename_input.len() {
                    self.filename_input.remove(self.filename_cursor_pos);
                }
                true
            }
            "ArrowLeft" => {
                if self.filename_cursor_pos > 0 {
                    self.filename_cursor_pos -= 1;
                }
                true
            }
            "ArrowRight" => {
                if self.filename_cursor_pos < self.filename_input.len() {
                    self.filename_cursor_pos += 1;
                }
                true
            }
            _ => {
                if key.len() == 1 {
                    let ch = key.chars().next().unwrap();
                    if ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
                        self.filename_input.insert(self.filename_cursor_pos, ch);
                        self.filename_cursor_pos += 1;
                        return true;
                    }
                }
                false
            }
        }
    }

    pub fn save_to_file(&mut self) -> bool {
        if let Some(filename) = &self.current_filename {
            let content = self.get_script_content();
            
            // Check if it's a lib.* file - save to memory instead
            if filename.starts_with("lib.") {
                return self.save_to_memory();
            }
            
            let file_path = if filename.ends_with(".cant") {
                filename.clone()
            } else {
                format!("{}.cant", filename)
            };
            
            match fs::write(&file_path, content) {
                Ok(_) => {
                    self.is_modified = false;
                    self.status_message = format!("Saved to {}", file_path);
                    true
                }
                Err(e) => {
                    self.status_message = format!("Error saving: {}", e);
                    false
                }
            }
        } else {
            // If no filename is set, this is an unnamed script - assign script ID and save to memory
            self.save_unnamed_to_memory()
        }
    }
    
    pub fn save_as_file(&mut self) -> bool {
        self.is_editing_filename = true;
        self.filename_cursor_pos = self.filename_input.len();
        true
    }
    
    pub fn open_file(&mut self) -> bool {
        self.is_editing_filename = true;
        self.filename_input.clear();
        self.filename_cursor_pos = 0;
        true
    }
    
    // Add new method to save to memory
    pub fn save_to_memory(&mut self) -> bool {
        if let Some(filename) = &self.current_filename {
            let content = self.get_script_content();
            // This will be handled by the interpreter when the editor closes
            self.is_modified = false;
            self.status_message = format!("Saved to memory: {}", filename);
            true
        } else {
            self.save_unnamed_to_memory()
        }
    }
    
    // Add new method to save unnamed scripts with auto-generated IDs
    pub fn save_unnamed_to_memory(&mut self) -> bool {
        let script_id = format!("script{}", self.next_script_id);
        self.next_script_id += 1;
        self.current_filename = Some(script_id.clone());
        self.filename_input = script_id;
        self.is_memory_script = true;
        self.is_modified = false;
        self.status_message = format!("Saved to memory as: {}", self.current_filename.as_ref().unwrap());
        true
    }
    
    // Add getter for memory script status
    pub fn is_memory_script(&self) -> bool {
        self.is_memory_script
    }
    
    // Add getter for filename
    pub fn get_filename(&self) -> Option<&String> {
        self.current_filename.as_ref()
    }
    
    pub fn ensure_cursor_visible(&mut self) {
        // Ensure the cursor is visible within the viewport
        if self.current_line < self.scroll_offset {
            self.scroll_offset = self.current_line;
        } else if self.current_line >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.current_line - self.viewport_height + 1;
        }
    }
}