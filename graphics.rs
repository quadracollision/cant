use pixels::{Pixels, SurfaceTexture};
use winit::window::Window;
use crate::grid::GridState;
use crate::game_objects::{GameObjectManager, GameObject};
use std::time::Instant;

#[derive(Debug)]
pub struct AudioPlaybackState {
    pub is_playing: bool,
    pub start_time: Instant,
    pub start_sample: f32,
    pub end_sample: f32,
    pub playback_duration: f64,
}

impl AudioPlaybackState {
    pub fn new() -> Self {
        Self {
            is_playing: false,
            start_time: Instant::now(),
            start_sample: 0.0,
            end_sample: 0.0,
            playback_duration: 0.0,
        }
    }
    
    pub fn start_playback(&mut self, start_sample: f32, end_sample: f32, duration: f64) {
        self.is_playing = true;
        self.start_time = Instant::now();
        self.start_sample = start_sample;
        self.end_sample = end_sample;
        self.playback_duration = duration;
    }
    
    pub fn stop_playback(&mut self) {
        self.is_playing = false;
    }
    
    pub fn get_current_playback_position(&self) -> Option<f32> {
        if !self.is_playing {
            return None;
        }
        
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed >= self.playback_duration {
            return None; // Playback finished
        }
        
        let progress = elapsed / self.playback_duration;
        let current_sample = self.start_sample + (self.end_sample - self.start_sample) * progress as f32;
        Some(current_sample)
    }
}

pub const GRID_PADDING: u32 = 10;
// Make console height scale with window size - more conservative sizing
fn get_console_height(window_height: u32, font_size_px: f32) -> u32 {
    // Fixed console height calculation for exactly 6 lines + padding
    let font_scale = font_size_px / 14.0;
    let line_height = crate::font::get_line_height(font_scale);
    let padding = (10.0 * font_scale).max(8.0) as usize;
    
    // Calculate height for exactly 6 lines (5 history + 1 command line) + padding
    let console_height = (6 * line_height) + (padding * 2);
    console_height as u32
}

pub struct GraphicsRenderer {
    pixels: Pixels,
    width: u32,
    height: u32,
    grid_width: u32,
    grid_height: u32,
    cursor_x: u32,
    cursor_y: u32,
    tile_size: u32,
    font_size: f32,  // Changed from font_scale to font_size (in pixels)
    // Waveform state
    waveform_cursor_position: f32,
    waveform_zoom_level: f32,
    waveform_scroll_position: f32,
    // Audio playback state
    audio_playback_state: AudioPlaybackState,
}

impl GraphicsRenderer {
    pub fn new(window: &Window, width: u32, height: u32) -> Result<Self, pixels::Error> {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
        let pixels = Pixels::new(width, height, surface_texture)?;
        
        Ok(Self {
            pixels,
            width,
            height,
            grid_width: 0,
            grid_height: 0,
            cursor_x: 0,
            cursor_y: 0,
            tile_size: 20,
            font_size: 14.0,  // Default 14px font size
            // Waveform state
            waveform_cursor_position: 0.0,
            waveform_zoom_level: 1.0,
            waveform_scroll_position: 0.0,
            // Audio playback state
            audio_playback_state: AudioPlaybackState::new(),
         })
     }

     // Render filename in top left corner of waveform view
     pub fn render_waveform_filename(&mut self, filename: &str) {
        let frame = self.pixels.frame_mut();
        
        // Extract just the filename from the path
        let display_name = if let Some(name) = std::path::Path::new(filename).file_name() {
            name.to_string_lossy().to_string()
        } else {
            filename.to_string()
        };
        
        // Draw filename at top left (10, 10) using the font system
        let start_x = 10usize;
        let start_y = 10usize;
        let font_scale = 1.0; // Use default scale for waveform filename
        
        crate::font::draw_text_scaled(
            frame,
            &display_name,
            start_x,
            start_y,
            [255, 255, 255], // White text
            false, // Not selected
            self.width as usize,
            font_scale,
        );
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        // Update internal dimensions to actual window size
        self.width = width;
        self.height = height;
        
        // Resize both surface and buffer to actual window size
        if let Err(err) = self.pixels.resize_surface(width, height) {
            log::error!("Failed to resize surface: {}", err);
        }
        if let Err(err) = self.pixels.resize_buffer(width, height) {
            log::error!("Failed to resize buffer: {}", err);
        }
    }

    pub fn set_grid_size(&mut self, width: u32, height: u32) {
        self.grid_width = width;
        self.grid_height = height;
        // Reset cursor to bounds
        self.cursor_x = self.cursor_x.min(width.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(height.saturating_sub(1));
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        if dx < 0 {
            self.cursor_x = self.cursor_x.saturating_sub((-dx) as u32);
        } else {
            self.cursor_x = (self.cursor_x + dx as u32).min(self.grid_width.saturating_sub(1));
        }
        
        if dy < 0 {
            self.cursor_y = self.cursor_y.saturating_sub((-dy) as u32);
        } else {
            self.cursor_y = (self.cursor_y + dy as u32).min(self.grid_height.saturating_sub(1));
        }
    }

    pub fn get_cursor_position(&self) -> (u32, u32) {
        (self.cursor_x, self.cursor_y)
    }

    // Waveform state getters
    pub fn get_waveform_state(&self) -> (f32, f32, f32) {
        (self.waveform_cursor_position, self.waveform_zoom_level, self.waveform_scroll_position)
    }

    // Waveform input handling
    pub fn handle_waveform_input(&mut self, key_code: winit::event::VirtualKeyCode, audio_samples: &[f32], modifiers: winit::event::ModifiersState, slice_markers: &[f32], sample_rate: f32, loaded_sample_key: Option<&str>) -> Option<String> {
        match key_code {
            winit::event::VirtualKeyCode::Left => {
                if modifiers.shift() && !slice_markers.is_empty() {
                    // Shift+Left: Jump to previous slice marker
                    let current_pos = self.waveform_cursor_position;
                    let mut prev_marker = None;
                    
                    // Find the closest marker to the left of current position
                    for &marker in slice_markers.iter().rev() {
                        if marker < current_pos {
                            prev_marker = Some(marker);
                            break;
                        }
                    }
                    
                    if let Some(marker_pos) = prev_marker {
                        self.waveform_cursor_position = marker_pos;
                        
                        // Auto-scroll if cursor goes off-screen
                        let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * self.waveform_zoom_level);
                        let cursor_screen_x = (self.waveform_cursor_position / samples_per_pixel) - self.waveform_scroll_position;
                        
                        if cursor_screen_x < 0.0 {
                            self.waveform_scroll_position = (self.waveform_cursor_position / samples_per_pixel) - (self.width as f32 * 0.1);
                            self.waveform_scroll_position = self.waveform_scroll_position.max(0.0);
                        }
                        
                        Some(format!("Jumped to previous slice marker at position: {:.0}", self.waveform_cursor_position))
                    } else {
                        Some("No previous slice marker found".to_string())
                    }
                } else if !audio_samples.is_empty() {
                    // Calculate step size based on zoom level for pixel-precise movement
                    let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * self.waveform_zoom_level);
                    let step_size = if self.waveform_zoom_level >= 5.0 {
                        // At high zoom levels, move by 1 pixel worth of samples
                        samples_per_pixel.max(1.0)
                    } else {
                        // At lower zoom levels, use percentage-based movement
                        (audio_samples.len() as f32 * 0.01).max(samples_per_pixel)
                    };
                    self.waveform_cursor_position = (self.waveform_cursor_position - step_size).max(0.0);
                    
                    // Auto-scroll if cursor goes off-screen
                    let cursor_screen_x = (self.waveform_cursor_position / samples_per_pixel) - self.waveform_scroll_position;
                    
                    if cursor_screen_x < 0.0 {
                        self.waveform_scroll_position = (self.waveform_cursor_position / samples_per_pixel) - (self.width as f32 * 0.1);
                        self.waveform_scroll_position = self.waveform_scroll_position.max(0.0);
                    }
                    
                    Some(format!("Cursor moved left to position: {:.0}", self.waveform_cursor_position))
                } else {
                    None
                }
            }
            winit::event::VirtualKeyCode::Right => {
                if modifiers.shift() && !slice_markers.is_empty() {
                    // Shift+Right: Jump to next slice marker
                    let current_pos = self.waveform_cursor_position;
                    let mut next_marker = None;
                    
                    // Find the closest marker to the right of current position
                    for &marker in slice_markers.iter() {
                        if marker > current_pos {
                            next_marker = Some(marker);
                            break;
                        }
                    }
                    
                    if let Some(marker_pos) = next_marker {
                        self.waveform_cursor_position = marker_pos;
                        
                        // Auto-scroll if cursor goes off-screen
                        let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * self.waveform_zoom_level);
                        let cursor_screen_x = (self.waveform_cursor_position / samples_per_pixel) - self.waveform_scroll_position;
                        
                        if cursor_screen_x > self.width as f32 {
                            self.waveform_scroll_position = (self.waveform_cursor_position / samples_per_pixel) - (self.width as f32 * 0.9);
                        }
                        
                        Some(format!("Jumped to next slice marker at position: {:.0}", self.waveform_cursor_position))
                    } else {
                        Some("No next slice marker found".to_string())
                    }
                } else if !audio_samples.is_empty() {
                    // Calculate step size based on zoom level for pixel-precise movement
                    let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * self.waveform_zoom_level);
                    let step_size = if self.waveform_zoom_level >= 5.0 {
                        // At high zoom levels, move by 1 pixel worth of samples
                        samples_per_pixel.max(1.0)
                    } else {
                        // At lower zoom levels, use percentage-based movement
                        (audio_samples.len() as f32 * 0.01).max(samples_per_pixel)
                    };
                    let max_position = audio_samples.len() as f32;
                    self.waveform_cursor_position = (self.waveform_cursor_position + step_size).min(max_position);
                    
                    // Auto-scroll if cursor goes off-screen
                    let cursor_screen_x = (self.waveform_cursor_position / samples_per_pixel) - self.waveform_scroll_position;
                    
                    if cursor_screen_x > self.width as f32 {
                        self.waveform_scroll_position = (self.waveform_cursor_position / samples_per_pixel) - (self.width as f32 * 0.9);
                    }
                    
                    Some(format!("Cursor moved right to position: {:.0}", self.waveform_cursor_position))
                } else {
                    None
                }
            }
            winit::event::VirtualKeyCode::Up => {
                // Zoom in and center on cursor
                self.waveform_zoom_level = (self.waveform_zoom_level * 1.2).min(100.0);

                if !audio_samples.is_empty() {
                    let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * self.waveform_zoom_level);
                    let center_x = self.width as f32 / 2.0;
                    let mut desired_scroll = (self.waveform_cursor_position / samples_per_pixel) - center_x;
                    let max_scroll = ((audio_samples.len() as f32) / samples_per_pixel) - self.width as f32;
                    let max_scroll = max_scroll.max(0.0);
                    self.waveform_scroll_position = desired_scroll.clamp(0.0, max_scroll);
                }

                Some(format!("Zoomed in to level: {:.2}", self.waveform_zoom_level))
            }
            winit::event::VirtualKeyCode::Down => {
                // Zoom out and center on cursor
                let min_zoom = 1.0;
                self.waveform_zoom_level = (self.waveform_zoom_level / 1.2).max(min_zoom);

                if !audio_samples.is_empty() {
                    let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * self.waveform_zoom_level);
                    let center_x = self.width as f32 / 2.0;
                    let mut desired_scroll = (self.waveform_cursor_position / samples_per_pixel) - center_x;
                    let max_scroll = ((audio_samples.len() as f32) / samples_per_pixel) - self.width as f32;
                    let max_scroll = max_scroll.max(0.0);
                    self.waveform_scroll_position = desired_scroll.clamp(0.0, max_scroll);
                }

                Some(format!("Zoomed out to level: {:.2}", self.waveform_zoom_level))
            }
            winit::event::VirtualKeyCode::Space => {
                // Handle Shift+Space for zoom reset
                if modifiers.shift() {
                    // Reset zoom to show entire waveform
                    self.waveform_zoom_level = 1.0;
                    self.waveform_scroll_position = 0.0;
                    Some("Zoom reset to show entire waveform".to_string())
                } else {
                    // Regular Space is handled in main.rs for slice markers
                    None
                }
            }
            winit::event::VirtualKeyCode::Return => {
                // Enter key: Play slice segment from current cursor to next slice marker
                if !slice_markers.is_empty() && !audio_samples.is_empty() {
                    let current_pos = self.waveform_cursor_position;
                    
                    // Find the current slice marker (closest marker at or before cursor)
                    let mut current_marker_idx = None;
                    for (idx, &marker) in slice_markers.iter().enumerate() {
                        if marker <= current_pos {
                            current_marker_idx = Some(idx);
                        } else {
                            break;
                        }
                    }
                    
                    if let Some(start_idx) = current_marker_idx {
                        let start_sample = slice_markers[start_idx] as usize;
                        let end_sample = if start_idx + 1 < slice_markers.len() {
                            slice_markers[start_idx + 1] as usize
                        } else {
                            audio_samples.len()
                        };
                        
                        // Move cursor to the start of the slice being played
                        self.waveform_cursor_position = slice_markers[start_idx];
                        
                        // Convert sample positions to time for audio playback
                        // Use actual sample rate from waveform editor
                        let start_time = start_sample as f64 / sample_rate as f64;
                        let end_time = end_sample as f64 / sample_rate as f64;
                        let duration = end_time - start_time;
                        
                        println!("DEBUG: Using sample rate: {} Hz", sample_rate);
                        println!("DEBUG: Sample indices {} to {} converted to time {:.3}s to {:.3}s", 
                                start_sample, end_sample, start_time, end_time);
                        
                        // Start audio playback state tracking
                        self.audio_playback_state.start_playback(
                            slice_markers[start_idx], 
                            slice_markers.get(start_idx + 1).copied().unwrap_or(audio_samples.len() as f32),
                            duration
                        );
                        
                        // Try to play the slice segment using the audio engine
                        match crate::audio_engine::with_audio_engine(|engine| {
                            // Use the loaded sample key from the waveform editor
                            if let Some(sample_key) = loaded_sample_key {
                                // Play the specific slice using the public wrapper method
                                engine.play_sample_slice_public(sample_key, start_time, end_time)
                            } else {
                                Err(crate::audio_engine::AudioError::PlaybackError("No audio file loaded in waveform editor".to_string()))
                            }
                        }) {
                            Ok(_) => Some(format!("Playing slice {} (samples {}-{}, {:.2}s-{:.2}s) - Cursor will follow playback", 
                                       start_idx, start_sample, end_sample, start_time, end_time)),
                            Err(e) => {
                                // Stop playback state if audio failed
                                self.audio_playback_state.stop_playback();
                                Some(format!("Audio playback failed: {} - Slice {} would play samples {}-{} ({:.2}s-{:.2}s)", 
                                         e, start_idx, start_sample, end_sample, start_time, end_time))
                            }
                        }
                    } else {
                        Some("No slice marker found at current position".to_string())
                    }
                } else {
                    Some("No slice markers or audio loaded".to_string())
                }
            }
            _ => None
        }
    }

    pub fn render(&mut self, grid_state: Option<&GridState>, console_lines: &[String], game_objects: Option<&GameObjectManager>) {
        let frame = self.pixels.frame_mut();
        
        // Clear the frame
        for pixel in frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[32, 32, 32, 255]);
        }
        
        // Render grid if available (without cursor)
        if let Some(grid) = grid_state {
            Self::render_grid_static(
                frame, grid, self.width, self.height, 
                self.grid_width, self.grid_height, 
                self.cursor_x, self.cursor_y, self.tile_size, self.font_size
            );
        }
        
        // Render game objects with proper dynamic scaling
        if let Some(objects) = game_objects {
            Self::render_game_objects_static(
                frame, objects, self.width, self.height, 
                self.grid_width, self.grid_height, self.tile_size, self.font_size
            );
        }
        
        // Render cursor outline AFTER game objects so it's always visible
        if let Some(grid) = grid_state {
            Self::render_cursor_overlay(
                frame, self.width, self.height,
                self.grid_width, self.grid_height,
                self.cursor_x, self.cursor_y, self.tile_size, self.font_size
            );
        }
        
        // Render console with font size
        Self::render_console_static(frame, console_lines, self.width, self.height, self.font_size);
    }

    pub fn render_waveform_mode(&mut self, console_lines: &[String], audio_samples: &[f32]) {
        let frame = self.pixels.frame_mut();
        
        // Clear frame with dark background
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 20;  // R
            pixel[1] = 20;  // G
            pixel[2] = 30;  // B
            pixel[3] = 255; // A
        }
        
        if audio_samples.is_empty() {
            // Show placeholder text if no audio is loaded
            let center_x = self.width / 2;
            let center_y = self.height / 2;
            
            let text = "No audio loaded - Use 'waveform(\"filename.wav\")' command";
            let text_width = text.len() as u32 * 8;
            let start_x = if center_x > text_width / 2 { center_x - text_width / 2 } else { 0 };
            
            // Draw simple white text pixels
            for (i, _ch) in text.chars().enumerate() {
                let char_x = start_x + (i as u32 * 8);
                if char_x < self.width && center_y < self.height {
                    for dy in 0..12 {
                        for dx in 0..6 {
                            let x = char_x + dx;
                            let y = center_y + dy;
                            if x < self.width && y < self.height {
                                let pixel_index = ((y * self.width + x) * 4) as usize;
                                if pixel_index + 3 < frame.len() {
                                    frame[pixel_index] = 255;     // R
                                    frame[pixel_index + 1] = 255; // G
                                    frame[pixel_index + 2] = 255; // B
                                    frame[pixel_index + 3] = 255; // A
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Draw the actual waveform
            let console_height = get_console_height(self.height, self.font_size);
            let waveform_height = self.height - console_height - 20; // Leave space for console and padding
            let waveform_center = waveform_height / 2;
            let waveform_scale = (waveform_height / 2) as f32 * 0.8;

            // Calculate samples per pixel with zoom and scroll
            let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * self.waveform_zoom_level);

            // Draw waveform
            for x in 0..self.width {
                let sample_start = ((x as f32 + self.waveform_scroll_position) * samples_per_pixel) as usize;
                let sample_end = (((x + 1) as f32 + self.waveform_scroll_position) * samples_per_pixel) as usize;
                
                if sample_start >= audio_samples.len() {
                    break;
                }
                
                let sample_end = sample_end.min(audio_samples.len());
                
                // Find min and max in this pixel range
                let mut min_val = 0.0f32;
                let mut max_val = 0.0f32;
                
                for i in sample_start..sample_end {
                    let sample = audio_samples[i];
                    min_val = min_val.min(sample);
                    max_val = max_val.max(sample);
                }
                
                // Convert to screen coordinates
                let min_y = (waveform_center as f32 - min_val * waveform_scale) as u32;
                let max_y = (waveform_center as f32 - max_val * waveform_scale) as u32;
                
                // Draw vertical line for this pixel
                let start_y = min_y.min(max_y).min(waveform_height - 1);
                let end_y = min_y.max(max_y).min(waveform_height - 1);
                
                for y in start_y..=end_y {
                    let pixel_index = ((y * self.width + x) * 4) as usize;
                    if pixel_index + 3 < frame.len() {
                        frame[pixel_index] = 100;     // R
                        frame[pixel_index + 1] = 200; // G
                        frame[pixel_index + 2] = 255; // B
                        frame[pixel_index + 3] = 255; // A
                    }
                }
            }
            
            // Update cursor position during playback
            if self.audio_playback_state.is_playing {
                if let Some(current_position) = self.audio_playback_state.get_current_playback_position() {
                    self.waveform_cursor_position = current_position;
                }
            }
            
            // Draw cursor - align with waveform sample mapping
            let cursor_screen_x = (self.waveform_cursor_position / samples_per_pixel - self.waveform_scroll_position) as u32;
            if cursor_screen_x < self.width {
                // Draw thick yellow cursor line spanning the waveform height
                for cursor_offset in 0..3 { // 3 pixels wide
                    let cursor_x = cursor_screen_x + cursor_offset;
                    if cursor_x < self.width {
                        for y in 0..waveform_height {
                            let pixel_index = ((y * self.width + cursor_x) * 4) as usize;
                            if pixel_index + 3 < frame.len() {
                                frame[pixel_index] = 255;     // R - bright yellow cursor
                                frame[pixel_index + 1] = 255; // G
                                frame[pixel_index + 2] = 0;   // B
                                frame[pixel_index + 3] = 255; // A
                            }
                        }
                    }
                }
            }
        }
        
        // Render console at the bottom
        Self::render_console_static(frame, console_lines, self.width, self.height, self.font_size);
    }
    
    fn color_name_to_rgba(color_name: &str) -> [u8; 4] {
        match color_name.to_lowercase().as_str() {
            "red" => [255, 0, 0, 255],
            "blue" => [0, 0, 255, 255],
            "green" => [0, 255, 0, 255],
            "yellow" => [255, 255, 0, 255],
            "orange" => [255, 165, 0, 255],
            "purple" => [128, 0, 128, 255],
            "pink" => [255, 192, 203, 255],
            "cyan" => [0, 255, 255, 255],
            "magenta" => [255, 0, 255, 255],
            "white" => [255, 255, 255, 255],
            "black" => [0, 0, 0, 255],
            "gray" => [128, 128, 128, 255],
            "brown" => [165, 42, 42, 255],
            "lime" => [0, 255, 0, 255],
            _ => [255, 255, 255, 255], // Default to white
        }
    }

    fn render_game_objects_static(frame: &mut [u8], objects: &GameObjectManager, width: u32, height: u32, grid_width: u32, grid_height: u32, tile_size: u32, font_size_px: f32) {
        // Calculate the same dynamic tile size as the grid rendering
        let available_width = width.saturating_sub(GRID_PADDING * 2);
        let available_height = height.saturating_sub(get_console_height(height, font_size_px) + GRID_PADDING * 2);
        
        // Use the EXACT same logic as render_grid_static - no fallback values!
        let max_tile_width = if grid_width > 0 { available_width / grid_width } else { tile_size };
        let max_tile_height = if grid_height > 0 { available_height / grid_height } else { tile_size };
        let dynamic_tile_size = max_tile_width.min(max_tile_height).max(1);
        
        let grid_pixel_width = grid_width * dynamic_tile_size;
        let grid_pixel_height = grid_height * dynamic_tile_size;
        
        // Center the grid in the available space (same as grid rendering)
        let start_x = GRID_PADDING + (available_width.saturating_sub(grid_pixel_width)) / 2;
        let start_y = GRID_PADDING + (available_height.saturating_sub(grid_pixel_height)) / 2;
        
        for obj in objects.get_all_objects().values() {
            match obj {
                GameObject::Ball(ball) => {
                    let screen_x = start_x + (ball.x * dynamic_tile_size as f64) as u32;
                    let screen_y = start_y + (ball.y * dynamic_tile_size as f64) as u32;
                    
                    let radius = (dynamic_tile_size as f64 * 0.4) as u32;
                    let color = Self::color_name_to_rgba(ball.get_color());
                    Self::draw_circle_static(frame, screen_x, screen_y, radius, color, width, height);
                },
                GameObject::Square(square) => {
                    let screen_x = start_x + (square.x * dynamic_tile_size as f64) as u32;
                    let screen_y = start_y + (square.y * dynamic_tile_size as f64) as u32;
                    let size = dynamic_tile_size;
                    let color = Self::color_name_to_rgba(square.get_color());
                    Self::draw_square_static(frame, screen_x, screen_y, size, color, width, height);
                    
                    // Draw label text if the square has one
                    if let Some(label_text) = square.get_label() {
                        draw_text_on_square(frame, screen_x, screen_y, label_text, width, height, size);
                    }
                }
            }
        }
    }
    
    fn draw_circle_static(frame: &mut [u8], center_x: u32, center_y: u32, radius: u32, color: [u8; 4], width: u32, height: u32) {
        let radius_sq = (radius * radius) as i32;
        
        for dy in -(radius as i32)..=(radius as i32) {
            for dx in -(radius as i32)..=(radius as i32) {
                if dx * dx + dy * dy <= radius_sq {
                    let px = (center_x as i32 + dx) as u32;
                    let py = (center_y as i32 + dy) as u32;
                    
                    if px < width && py < height {
                        let index = ((py * width + px) * 4) as usize;
                        if index + 3 < frame.len() {
                            frame[index] = color[0];
                            frame[index + 1] = color[1];
                            frame[index + 2] = color[2];
                            frame[index + 3] = color[3];
                        }
                    }
                }
            }
        }
    }
    
    fn draw_square_static(frame: &mut [u8], x: u32, y: u32, size: u32, color: [u8; 4], width: u32, height: u32) {
        for dy in 0..size {
            for dx in 0..size {
                let px = x + dx;
                let py = y + dy;
                
                if px < width && py < height {
                    let index = ((py * width + px) * 4) as usize;
                    if index + 3 < frame.len() {
                        frame[index] = color[0];
                        frame[index + 1] = color[1];
                        frame[index + 2] = color[2];
                        frame[index + 3] = color[3];
                    }
                }
            }
        }
    }

    fn draw_cell_outline_static(frame: &mut [u8], x: u32, y: u32, color: [u8; 4], width: u32, height: u32, tile_size: u32) {
        let thickness = 3; // Make cursor outline 3 pixels thick
        
        // Draw top and bottom borders with thickness
        for t in 0..thickness {
            for dx in 0..tile_size {
                // Top border
                let px = x + dx;
                let py = y + t;
                if px < width && py < height {
                    let index = ((py * width + px) * 4) as usize;
                    if index + 3 < frame.len() {
                        frame[index] = color[0];
                        frame[index + 1] = color[1];
                        frame[index + 2] = color[2];
                        frame[index + 3] = color[3];
                    }
                }
                
                // Bottom border
                let py = y + tile_size - 1 - t;
                if px < width && py < height {
                    let index = ((py * width + px) * 4) as usize;
                    if index + 3 < frame.len() {
                        frame[index] = color[0];
                        frame[index + 1] = color[1];
                        frame[index + 2] = color[2];
                        frame[index + 3] = color[3];
                    }
                }
            }
        }
        
        // Draw left and right borders with thickness
        for t in 0..thickness {
            for dy in 0..tile_size {
                // Left border
                let px = x + t;
                let py = y + dy;
                if px < width && py < height {
                    let index = ((py * width + px) * 4) as usize;
                    if index + 3 < frame.len() {
                        frame[index] = color[0];
                        frame[index + 1] = color[1];
                        frame[index + 2] = color[2];
                        frame[index + 3] = color[3];
                    }
                }
                
                // Right border
                let px = x + tile_size - 1 - t;
                if px < width && py < height {
                    let index = ((py * width + px) * 4) as usize;
                    if index + 3 < frame.len() {
                        frame[index] = color[0];
                        frame[index + 1] = color[1];
                        frame[index + 2] = color[2];
                        frame[index + 3] = color[3];
                    }
                }
            }
        }
    }

    pub fn render_waveform(&mut self, audio_samples: &[f32], zoom_level: f32, scroll_position: f32, markers: &[f32], cursor_position: f32) {
        let frame = self.pixels.frame_mut();
        
        // Clear frame with dark background
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 20;  // R
            pixel[1] = 20;  // G
            pixel[2] = 30;  // B
            pixel[3] = 255; // A
        }

        if audio_samples.is_empty() {
            return;
        }

        let waveform_height = self.height - 100; // Leave space for controls
        let waveform_center = waveform_height / 2;
        let waveform_scale = (waveform_height / 2) as f32 * 0.8;

        // Calculate samples per pixel based on zoom
        let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * zoom_level);

        // Draw waveform
        for x in 0..self.width {
            let sample_start = ((x as f32 + scroll_position) * samples_per_pixel) as usize;
            let sample_end = (((x + 1) as f32 + scroll_position) * samples_per_pixel) as usize;
            
            if sample_start >= audio_samples.len() {
                break;
            }
            
            let sample_end = sample_end.min(audio_samples.len());
            
            // Find min and max in this pixel range
            let mut min_val = 0.0f32;
            let mut max_val = 0.0f32;
            
            for i in sample_start..sample_end {
                let sample = audio_samples[i];
                min_val = min_val.min(sample);
                max_val = max_val.max(sample);
            }
            
            // Convert to screen coordinates
            let min_y = (waveform_center as f32 - min_val * waveform_scale) as u32;
            let max_y = (waveform_center as f32 - max_val * waveform_scale) as u32;
            
            // Draw vertical line for this pixel
            let start_y = min_y.min(max_y).min(waveform_height - 1);
            let end_y = min_y.max(max_y).min(waveform_height - 1);
            
            for y in start_y..=end_y {
                let pixel_index = ((y * self.width + x) * 4) as usize;
                if pixel_index + 3 < frame.len() {
                    frame[pixel_index] = 100;     // R
                    frame[pixel_index + 1] = 150; // G
                    frame[pixel_index + 2] = 255; // B
                    frame[pixel_index + 3] = 255; // A
                }
            }
        }

        // Draw cursor position
        let cursor_x = ((cursor_position / samples_per_pixel) - scroll_position) as u32;
        if cursor_x < self.width {
            // Draw vertical cursor line
            for y in 0..waveform_height {
                let pixel_index = ((y * self.width + cursor_x) * 4) as usize;
                if pixel_index + 3 < frame.len() {
                    frame[pixel_index] = 255;     // R
                    frame[pixel_index + 1] = 255; // G
                    frame[pixel_index + 2] = 100; // B
                    frame[pixel_index + 3] = 255; // A
                }
            }
        }

        // Draw markers (existing markers from the old system)
        for (i, &marker_time) in markers.iter().enumerate() {
            let marker_x = ((marker_time / samples_per_pixel) - scroll_position) as u32;
            
            if marker_x < self.width {
                // Draw vertical marker line
                for y in 0..waveform_height {
                    let pixel_index = ((y * self.width + marker_x) * 4) as usize;
                    if pixel_index + 3 < frame.len() {
                        frame[pixel_index] = 255;     // R
                        frame[pixel_index + 1] = 100; // G
                        frame[pixel_index + 2] = 100; // B
                        frame[pixel_index + 3] = 255; // A
                    }
                }
                
                // Draw marker number at the top
                if marker_x > 10 && marker_x < self.width - 10 {
                    let marker_text = format!("{}", i);
                    // Simple text rendering - just draw a small rectangle for now
                    for dy in 0..10 {
                        for dx in 0..20 {
                            let px = marker_x - 10 + dx;
                            let py = 5 + dy;
                            if px < self.width && py < self.height {
                                let pixel_index = ((py * self.width + px) * 4) as usize;
                                if pixel_index + 3 < frame.len() {
                                    frame[pixel_index] = 255;     // R
                                    frame[pixel_index + 1] = 255; // G
                                    frame[pixel_index + 2] = 100; // B
                                    frame[pixel_index + 3] = 255; // A
                                }
                            }
                        }
                    }
                }
            }
        }

        // Draw center line
        let center_y = waveform_center;
        for x in 0..self.width {
            let pixel_index = ((center_y * self.width + x) * 4) as usize;
            if pixel_index + 3 < frame.len() {
                frame[pixel_index] = 80;      // R
                frame[pixel_index + 1] = 80;  // G
                frame[pixel_index + 2] = 80;  // B
                frame[pixel_index + 3] = 255; // A
            }
        }
    }

     pub fn set_tile_size(&mut self, size: u32) {
         self.tile_size = size.clamp(4, 100);
     }

    pub fn get_tile_size(&self) -> u32 {
        self.tile_size
    }

    // Add these methods after the existing get_tile_size method
    pub fn set_font_size(&mut self, size: f32) {
        self.font_size = size.clamp(8.0, 48.0);  // Limit font size between 8px and 48px
    }

    pub fn get_font_size(&self) -> f32 {
        self.font_size
    }

    pub fn force_redraw(&mut self) {
        println!("Debug: force_redraw() called - clearing frame buffer");
        // Clear the entire frame buffer to black
        let frame = self.pixels.frame_mut();
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 0; // Red
            pixel[1] = 0; // Green  
            pixel[2] = 0; // Blue
            pixel[3] = 255; // Alpha
        }
    }

    pub fn present(&mut self) -> Result<(), pixels::Error> {
        self.pixels.render()
    }

    fn render_grid_static(
        frame: &mut [u8], 
        grid: &GridState, 
        width: u32, 
        height: u32, 
        grid_width: u32, 
        grid_height: u32, 
        cursor_x: u32,
        cursor_y: u32,
        tile_size: u32,
        font_size_px: f32
    ) {
        // Calculate available space (excluding console area)
        let available_width = width.saturating_sub(GRID_PADDING * 2);
        let available_height = height.saturating_sub(get_console_height(height, font_size_px) + GRID_PADDING * 2);
        
        // Calculate optimal tile size to fit the grid in available space
        let max_tile_width = if grid_width > 0 { available_width / grid_width } else { tile_size };
        let max_tile_height = if grid_height > 0 { available_height / grid_height } else { tile_size };
        let dynamic_tile_size = max_tile_width.min(max_tile_height).max(1); // Ensure minimum size of 1
        
        let grid_pixel_width = grid_width * dynamic_tile_size;
        let grid_pixel_height = grid_height * dynamic_tile_size;
        
        // Center the grid in the available space
        let start_x = GRID_PADDING + (available_width.saturating_sub(grid_pixel_width)) / 2;
        let start_y = GRID_PADDING + (available_height.saturating_sub(grid_pixel_height)) / 2;
        
        // Draw cells
        for y in 0..grid_height {
            for x in 0..grid_width {
                let cell_x = start_x + x * dynamic_tile_size;
                let cell_y = start_y + y * dynamic_tile_size;
                
                let color = if x < grid.width as u32 && y < grid.height as u32 {
                    // Use the boolean grid system
                    if grid.cells[y as usize][x as usize] {
                        [128, 128, 128, 255] // Gray for filled cells (true)
                    } else {
                        [64, 64, 64, 255]    // Dark gray for empty cells (false)
                    }
                } else {
                    [32, 32, 32, 255] // Background color for empty areas
                };
                
                // Always draw the normal cell (no cursor highlighting here)
                Self::draw_cell_static(frame, cell_x, cell_y, color, width, height, dynamic_tile_size);
            }
        }
        
        // Draw grid lines
        Self::draw_grid_lines_static(frame, start_x, start_y, grid_pixel_width, grid_pixel_height, grid_width, grid_height, width, height, dynamic_tile_size);
    }

    fn draw_cell_static(frame: &mut [u8], x: u32, y: u32, color: [u8; 4], width: u32, height: u32, tile_size: u32) {
        for dy in 0..tile_size {
            for dx in 0..tile_size {
                let px = x + dx;
                let py = y + dy;
                
                if px < width && py < height {
                    let index = ((py * width + px) * 4) as usize;
                    if index + 3 < frame.len() {
                        frame[index] = color[0];     // Red
                        frame[index + 1] = color[1]; // Green
                        frame[index + 2] = color[2]; // Blue
                        frame[index + 3] = color[3]; // Alpha
                    }
                }
            }
        }
    }

    fn draw_grid_lines_static(
        frame: &mut [u8], 
        start_x: u32, 
        start_y: u32, 
        grid_pixel_width: u32, 
        grid_pixel_height: u32, 
        grid_width: u32, 
        grid_height: u32, 
        width: u32, 
        height: u32,
        tile_size: u32
    ) {
        let line_color = [96, 96, 96, 255]; // Gray grid lines
        
        // Draw vertical lines
        for x in 0..=grid_width {
            let line_x = start_x + x * tile_size;
            for y in 0..grid_pixel_height {
                let py = start_y + y;
                if line_x < width && py < height {
                    let index = ((py * width + line_x) * 4) as usize;
                    if index + 3 < frame.len() {
                        frame[index] = line_color[0];
                        frame[index + 1] = line_color[1];
                        frame[index + 2] = line_color[2];
                        frame[index + 3] = line_color[3];
                    }
                }
            }
        }
        
        // Draw horizontal lines
        for y in 0..=grid_height {
            let line_y = start_y + y * tile_size;
            for x in 0..grid_pixel_width {
                let px = start_x + x;
                if px < width && line_y < height {
                    let index = ((line_y * width + px) * 4) as usize;
                    if index + 3 < frame.len() {
                        frame[index] = line_color[0];
                        frame[index + 1] = line_color[1];
                        frame[index + 2] = line_color[2];
                        frame[index + 3] = line_color[3];
                    }
                }
            }
        }
    }

    fn render_console_static(frame: &mut [u8], lines: &[String], width: u32, height: u32, font_size_px: f32) {
        let console_height = get_console_height(height, font_size_px);
        let console_start_y = height - console_height;
        
        // Convert pixel size to scale factor (base font size is 14.0px)
        let font_scale = font_size_px / 14.0;
        
        let line_height = crate::font::get_line_height(font_scale);
        let padding = (10.0 * font_scale).max(8.0) as usize;
        
        // Draw console background
        for y in console_start_y..height {
            for x in 0..width {
                let index = ((y * width + x) * 4) as usize;
                if index + 3 < frame.len() {
                    frame[index] = 16;     // Dark background
                    frame[index + 1] = 16;
                    frame[index + 2] = 16;
                    frame[index + 3] = 255;
                }
            }
        }
        
        // Draw console text using scaled font
        let text_color = [200, 200, 200]; // Light gray text
        let start_x = padding;
        
        // Fixed: Always display exactly 6 lines (5 history + 1 command)
        let max_history_lines = 5;
        
        if !lines.is_empty() {
            // Check if this is script editor content (starts with "Script:")
            let is_script_editor = lines.first().map_or(false, |line| line.starts_with("Script:"));
            
            // Separate the last line as the command line
            let (history_lines, command_line) = if lines.len() > 1 {
                (&lines[..lines.len()-1], &lines[lines.len()-1])
            } else {
                (&[][..], &lines[0])
            };
            
            // Calculate command line position (moved down by 20 pixels for regular console)
            let command_y = if is_script_editor {
                console_start_y + console_height - padding as u32 - line_height as u32
            } else {
                console_start_y + console_height - padding as u32 - line_height as u32 + 20
            };
            
            // Render command line
            crate::font::draw_text_scaled(
                frame,
                command_line,
                start_x,
                command_y as usize,
                text_color,
                false,
                width as usize,
                font_scale,
            );
            
            // Render history lines (from bottom up, above command line)
            let available_history_lines = history_lines.len().min(max_history_lines);
            let start_history_index = if history_lines.len() > max_history_lines {
                history_lines.len() - max_history_lines
            } else {
                0
            };
            
            for (i, line) in history_lines[start_history_index..].iter().enumerate() {
                let line_y = command_y - ((available_history_lines - i) as u32 * line_height as u32);
                
                // Only render if within console bounds
                if line_y >= console_start_y {
                    crate::font::draw_text_scaled(
                        frame,
                        line,
                        start_x,
                        line_y as usize,
                        text_color,
                        false,
                        width as usize,
                        font_scale,
                    );
                }
            }
        }
    }

    fn render_cursor_overlay(
        frame: &mut [u8],
        width: u32,
        height: u32,
        grid_width: u32,
        grid_height: u32,
        cursor_x: u32,
        cursor_y: u32,
        tile_size: u32,
        font_size_px: f32
    ) {
        // Calculate available space (excluding console area) - same as grid rendering
        let available_width = width.saturating_sub(GRID_PADDING * 2);
        let available_height = height.saturating_sub(get_console_height(height, font_size_px) + GRID_PADDING * 2);
        
        // Calculate optimal tile size to fit the grid in available space - same as grid rendering
        let max_tile_width = if grid_width > 0 { available_width / grid_width } else { tile_size };
        let max_tile_height = if grid_height > 0 { available_height / grid_height } else { tile_size };
        let dynamic_tile_size = max_tile_width.min(max_tile_height).max(1); // Ensure minimum size of 1
        
        let grid_pixel_width = grid_width * dynamic_tile_size;
        let grid_pixel_height = grid_height * dynamic_tile_size;
        
        // Center the grid in the available space - same as grid rendering
        let start_x = GRID_PADDING + (available_width.saturating_sub(grid_pixel_width)) / 2;
        let start_y = GRID_PADDING + (available_height.saturating_sub(grid_pixel_height)) / 2;
        
        // Use dynamic tile size for cursor positioning
        let cursor_pixel_x = start_x + cursor_x * dynamic_tile_size;
        let cursor_pixel_y = start_y + cursor_y * dynamic_tile_size;
        
        Self::draw_cell_outline_static(frame, cursor_pixel_x, cursor_pixel_y, [255, 255, 0, 255], width, height, dynamic_tile_size);
    }

    // Render slice markers (rendering only - data comes from external source)
    pub fn render_slice_markers(&mut self, slice_markers: &[f32], zoom_level: f32, scroll_position: f32, audio_samples: &[f32]) {
        let frame = self.pixels.frame_mut();
        let console_height = get_console_height(self.height, self.font_size);
        let waveform_height = self.height - console_height - 20;
        
        // Use the EXACT same coordinate calculation as waveform rendering
        // This must match render_waveform_mode exactly
        let samples_per_pixel = (audio_samples.len() as f32) / (self.width as f32 * zoom_level);
        
        // Draw slice markers in green spanning the full waveform height
        for (index, &marker_pos) in slice_markers.iter().enumerate() {
            // Convert sample position to screen coordinate using the EXACT same formula as waveform
            // This matches the calculation in render_waveform_mode
            let screen_x = ((marker_pos / samples_per_pixel) - scroll_position) as u32;
            
            if screen_x < self.width {
                // Draw vertical line for slice marker spanning full waveform height
                for y in 0..waveform_height {
                    if y < self.height {
                        let pixel_index = ((y * self.width + screen_x) * 4) as usize;
                        if pixel_index + 3 < frame.len() {
                            frame[pixel_index] = 0;     // R - Green slice marker
                            frame[pixel_index + 1] = 255; // G
                            frame[pixel_index + 2] = 0;   // B
                            frame[pixel_index + 3] = 255; // A
                        }
                    }
                }
                
                // Draw slice number at the bottom of the marker
                let slice_number = index + 1; // 1-based indexing for display
                let number_text = slice_number.to_string();
                
                // Draw slice number using the font system
                let digit_x = screen_x as usize;
                let digit_y = waveform_height.saturating_sub(15) as usize; // Draw near bottom of waveform
                let font_scale = 0.8; // Smaller scale for slice numbers
                
                crate::font::draw_text_scaled(
                    frame,
                    &number_text,
                    digit_x,
                    digit_y,
                    [255, 255, 255], // White text
                    false, // Not selected
                    self.width as usize,
                    font_scale,
                );
            }
        }
    }
}

fn draw_text_on_square(frame: &mut [u8], x: u32, y: u32, text: &str, width: u32, height: u32, tile_size: u32) {
    let font_scale = (tile_size as f32 / 32.0).max(0.5);
    let char_width = (8.0 * font_scale) as u32;
    let char_height = (12.0 * font_scale) as u32;
    
    let text_x = x + (tile_size - char_width * text.len() as u32) / 2;
    let text_y = y + (tile_size - char_height) / 2;
    
    crate::font::draw_text_scaled(
        frame,
        text,
        text_x as usize,
        text_y as usize,
        [255, 255, 255],
        false,
        width as usize,
        font_scale,
    );
}
