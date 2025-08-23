use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub struct Console {
    lines: VecDeque<String>,
    max_lines: usize,
    current_command: String,
    prompt: String,
    log_file: Option<std::fs::File>,
    // Add command history fields
    command_history: VecDeque<String>,
    max_history: usize,
    history_index: Option<usize>,
    temp_command: String, // Store current command when navigating history
}

impl Console {
    pub fn new(max_lines: usize) -> Self {
        let log_file = Self::create_log_file();
        
        let mut console = Self {
            lines: VecDeque::new(),
            max_lines,
            current_command: String::new(),
            prompt: "cant> ".to_string(),
            log_file,
            command_history: VecDeque::new(),
            max_history: 50, // Store last 50 commands
            history_index: None,
            temp_command: String::new(),
        };
        
        console.add_line("Quadracollision Canticle".to_string());
        console.add_line("".to_string());
        
        console
    }

    fn create_log_file() -> Option<std::fs::File> {
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open("console.log")
        {
            Ok(file) => {
                println!("Console logging enabled: console.log");
                Some(file)
            }
            Err(e) => {
                eprintln!("Failed to create console.log: {}", e);
                None
            }
        }
    }

    fn write_to_log(&mut self, text: &str) {
        if let Some(ref mut file) = self.log_file {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            if let Err(e) = writeln!(file, "[{}] {}", timestamp, text) {
                eprintln!("Failed to write to console.log: {}", e);
            }
            
            // Ensure the log is written immediately
            let _ = file.flush();
        }
    }

    pub fn add_line(&mut self, line: String) {
        // Write to log file first
        self.write_to_log(&line);
        
        // Then add to console display
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
        let error_msg = format!("Error: {}", error);
        self.add_line(error_msg);
    }

    pub fn add_command(&mut self, command: &str) {
        let command_line = format!("{}{}", self.prompt, command);
        self.add_line(command_line);
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
        self.write_to_log("--- Console cleared ---");
        self.lines.clear();
        self.current_command.clear();
    }

    pub fn execute_command(&mut self, command: String) -> String {
        self.add_command(&command);
        
        
        if !command.trim().is_empty() {
            if self.command_history.is_empty() || self.command_history.back() != Some(&command) {
                self.command_history.push_back(command.clone());
                while self.command_history.len() > self.max_history {
                    self.command_history.pop_front();
                }
            }
        }
        
        self.current_command.clear();
        self.history_index = None;
        self.temp_command.clear();
        command
    }

    // Navigate to previous command in history (Shift+Up)
    pub fn history_previous(&mut self) {
        if self.command_history.is_empty() {
            return;
        }

        match self.history_index {
            None => {
                // First time navigating history - save current command
                self.temp_command = self.current_command.clone();
                self.history_index = Some(self.command_history.len() - 1);
                self.current_command = self.command_history[self.command_history.len() - 1].clone();
            }
            Some(index) => {
                if index > 0 {
                    self.history_index = Some(index - 1);
                    self.current_command = self.command_history[index - 1].clone();
                }
            }
        }
    }

    // Navigate to next command in history (Shift+Down)
    pub fn history_next(&mut self) {
        if let Some(index) = self.history_index {
            if index < self.command_history.len() - 1 {
                self.history_index = Some(index + 1);
                self.current_command = self.command_history[index + 1].clone();
            } else {
                // Reached end of history - restore temp command
                self.history_index = None;
                self.current_command = self.temp_command.clone();
                self.temp_command.clear();
            }
        }
    }

    // Reset history navigation when user types
    pub fn reset_history_navigation(&mut self) {
        self.history_index = None;
        self.temp_command.clear();
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new(100)
    }
}