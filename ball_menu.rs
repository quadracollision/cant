use crate::audio_engine::AudioEngine;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum MenuOption {
    LoadSample,
    // Future options can be added here
}

impl MenuOption {
    pub fn display_text(&self) -> &'static str {
        match self {
            MenuOption::LoadSample => "Load Sample",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BallMenu {
    pub ball_id: u32,
    pub ball_name: String,
    pub options: Vec<String>,
    pub selected_index: usize,
    pub is_open: bool,
    pub audio_channel_id: Option<u32>,
}

impl BallMenu {
    pub fn new(ball_id: u32, ball_name: String) -> Self {
        Self {
            ball_id,
            ball_name,
            options: vec![
                "Load Sample".to_string(),
                "Close".to_string(),
            ],
            selected_index: 0,
            is_open: true,
            audio_channel_id: None,
        }
    }

    pub fn navigate_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.options.len() - 1;
        }
    }

    pub fn navigate_down(&mut self) {
        if self.selected_index < self.options.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
    }

    pub fn get_selected_option(&self) -> Option<&MenuOption> {
        self.options.get(self.selected_index)
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn execute_selected_option(&mut self, audio_engine: &mut AudioEngine) -> Result<String, String> {
        match self.get_selected_option() {
            Some(MenuOption::LoadSample) => {
                self.load_sample(audio_engine)
            }
            None => Err("No option selected".to_string()),
        }
    }

    fn load_sample(&mut self, audio_engine: &mut AudioEngine) -> Result<String, String> {
        // Create a dedicated audio channel for this ball if it doesn't exist
        if self.audio_channel_id.is_none() {
            let channel_name = format!("{}_audio", self.ball_name);
            let channel_id = audio_engine.create_channel(channel_name);
            self.audio_channel_id = Some(channel_id);
        }

        // For now, we'll use a placeholder sample path
        // In a real implementation, this would open a file dialog or use a predefined sample
        let sample_path = "sample.wav"; // This should be configurable
        
        match self.audio_channel_id {
            Some(channel_id) => {
                match audio_engine.preload_sample(sample_path) {
                    Ok(_) => {
                        Ok(format!("Sample loaded for {} on channel {}", self.ball_name, channel_id))
                    }
                    Err(e) => {
                        Err(format!("Failed to load sample: {}", e))
                    }
                }
            }
            None => Err("No audio channel available".to_string()),
        }
    }

    pub fn render(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("=== {} Menu ===", self.ball_name));
        lines.push(String::new());
        
        for (index, option) in self.options.iter().enumerate() {
            let prefix = if index == self.selected_index { "> " } else { "  " };
            lines.push(format!("{}{}", prefix, option.display_text()));
        }
        
        lines.push(String::new());
        lines.push("Use arrow keys to navigate, Enter to select, Esc to close".to_string());
        
        lines
    }
}