use std::collections::VecDeque;

pub struct Console {
    lines: VecDeque<String>,
    max_lines: usize,
    current_command: String,
    prompt: String,
}

impl Console {
    pub fn new(max_lines: usize) -> Self {
        let mut console = Self {
            lines: VecDeque::new(),
            max_lines,
            current_command: String::new(),
            prompt: "cant> ".to_string(),
        };
        
        console.add_line("CANT Language Interpreter v3.0".to_string());
        console.add_line("Use arrow keys to move cursor, SPACE to toggle cells".to_string());
        console.add_line("Type commands and press ENTER to execute".to_string());
        console.add_line("".to_string());
        
        console
    }

    pub fn add_line(&mut self, line: String) {
        self.lines.push_back(line);
        while self.lines.len() > self.max_lines {
            self.lines.pop_front();
        }
    }

    pub fn add_output(&mut self, output: &str) {
        for line in output.lines() {
            self.add_line(line.to_string());
        }
    }

    pub fn add_error(&mut self, error: &str) {
        self.add_line(format!("Error: {}", error));
    }

    pub fn add_command(&mut self, command: &str) {
        self.add_line(format!("{}{}", self.prompt, command));
    }

    pub fn set_current_command(&mut self, command: String) {
        self.current_command = command;
    }

    pub fn get_current_command(&self) -> &str {
        &self.current_command
    }

    pub fn get_lines(&self) -> Vec<String> {
        let mut result = Vec::new();
        
        // Add all stored lines
        for line in &self.lines {
            result.push(line.clone());
        }
        
        // Don't add current command here - it's handled in get_display_lines
        result
    }

    pub fn get_display_lines(&self, max_display_lines: usize) -> Vec<String> {
        let mut all_lines = self.get_lines();
        
        // Add the current command prompt with typed text
        let current_prompt = format!("{}{}", self.prompt, self.current_command);
        all_lines.push(current_prompt);
        
        let start_index = if all_lines.len() > max_display_lines {
            all_lines.len() - max_display_lines
        } else {
            0
        };
        
        all_lines[start_index..].to_vec()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.current_command.clear();
    }

    pub fn execute_command(&mut self, command: String) -> String {
        self.add_command(&command);
        self.current_command.clear();
        command
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new(100)
    }
}