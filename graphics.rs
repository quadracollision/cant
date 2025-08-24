use pixels::{Pixels, SurfaceTexture};
use winit::window::Window;
use crate::grid::GridState;
use crate::font;
use crate::game_objects::{GameObjectManager, GameObject};

pub const GRID_PADDING: u32 = 10;
pub const CONSOLE_HEIGHT: u32 = 120;  // Reduced from 200 to 120

pub struct GraphicsRenderer {
    pixels: Pixels,
    width: u32,
    height: u32,
    grid_width: u32,
    grid_height: u32,
    cursor_x: u32,
    cursor_y: u32,
    tile_size: u32,
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
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Err(err) = self.pixels.resize_surface(width, height) {
            log::error!("Failed to resize surface: {}", err);
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

    pub fn render(&mut self, grid_state: Option<&GridState>, console_lines: &[String], game_objects: Option<&GameObjectManager>) {
        let frame = self.pixels.frame_mut();
        
        // Clear the frame
        for pixel in frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[32, 32, 32, 255]);
        }
        
        // Render grid if available
        if let Some(grid) = grid_state {
            Self::render_grid_static(
                frame, grid, self.width, self.height, 
                self.grid_width, self.grid_height, 
                self.cursor_x, self.cursor_y, self.tile_size
            );
        }
        
        // Render game objects with proper dynamic scaling
        if let Some(objects) = game_objects {
            Self::render_game_objects_static(
                frame, objects, self.width, self.height, 
                self.grid_width, self.grid_height, self.tile_size
            );
        }
        
        // Render console with font
        Self::render_console_static(frame, console_lines, self.width, self.height);
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

    fn render_game_objects_static(frame: &mut [u8], objects: &GameObjectManager, width: u32, height: u32, grid_width: u32, grid_height: u32, tile_size: u32) {
        // Calculate the same dynamic tile size as the grid rendering
        let available_width = width.saturating_sub(GRID_PADDING * 2);
        let available_height = height.saturating_sub(CONSOLE_HEIGHT + GRID_PADDING * 2);
        
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
        // Draw top and bottom borders
        for dx in 0..tile_size {
            // Top border
            let px = x + dx;
            let py = y;
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
            let py = y + tile_size - 1;
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
        
        // Draw left and right borders
        for dy in 0..tile_size {
            // Left border
            let px = x;
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
            let px = x + tile_size - 1;
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

    pub fn set_tile_size(&mut self, size: u32) {
        self.tile_size = size.clamp(4, 100);
    }

    pub fn get_tile_size(&self) -> u32 {
        self.tile_size
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
        tile_size: u32
    ) {
        // Calculate available space (excluding console area)
        let available_width = width.saturating_sub(GRID_PADDING * 2);
        let available_height = height.saturating_sub(CONSOLE_HEIGHT + GRID_PADDING * 2);
        
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
        
        // Draw grid lines first
        Self::draw_grid_lines_static(frame, start_x, start_y, grid_pixel_width, grid_pixel_height, grid_width, grid_height, width, height, dynamic_tile_size);
        
        // Then draw cursor outline on top of everything
        let cursor_cell_x = start_x + cursor_x * dynamic_tile_size;
        let cursor_cell_y = start_y + cursor_y * dynamic_tile_size;
        Self::draw_cell_outline_static(frame, cursor_cell_x, cursor_cell_y, [255, 255, 0, 255], width, height, dynamic_tile_size);
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

    fn render_console_static(frame: &mut [u8], lines: &[String], width: u32, height: u32) {
        let console_start_y = height - CONSOLE_HEIGHT;
        
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
        
        // Draw console text using font
        let text_color = [200, 200, 200]; // Light gray text
        let line_height = 14; // 12px font + 2px spacing
        let start_x = 10;
        let padding = 10;
        
        // Calculate available lines in console area
        let available_height = CONSOLE_HEIGHT - (padding * 2);
        let max_lines = (available_height / line_height as u32) as usize;
        
        // Ensure we don't exceed available space
        let display_lines = if lines.len() > max_lines {
            &lines[lines.len() - max_lines..]
        } else {
            lines
        };
        
        for (i, line) in display_lines.iter().enumerate() {
            let text_y = console_start_y as usize + padding as usize + (i * line_height);
            if text_y + 12 < height as usize {
                font::draw_text(frame, line, start_x, text_y, text_color, false, width as usize);
            }
        }
    }
}

fn draw_text_on_square(frame: &mut [u8], x: u32, y: u32, text: &str, width: u32, height: u32, tile_size: u32) {
    let lines: Vec<&str> = text.split('\n').collect();
    let char_width = 8;  // Font character width
    let char_height = 12; // Font character height
    let line_spacing = 14; // 12px font + 2px spacing
    let text_color = [255, 255, 255]; // White text
    
    for (line_idx, line) in lines.iter().enumerate() {
        let line_y = y + (line_idx as u32 * line_spacing as u32);
        let line_x = x + 2; // Small padding from square edge
        
        // Ensure text fits within the square bounds
        if line_y + char_height as u32 <= y + tile_size && line_x + (line.len() as u32 * char_width as u32) <= x + tile_size {
            font::draw_text(frame, line, line_x as usize, line_y as usize, text_color, false, width as usize);
        }
    }
}

// Remove the draw_simple_char function as it's no longer needed
fn draw_simple_char(frame: &mut [u8], x: u32, y: u32, ch: char, color: [u8; 4], width: u32, height: u32) {
    // Very basic character rendering - you can improve this
    // For now, just draw a small rectangle for each character
    for dy in 0..3 {
        for dx in 0..2 {
            let px = x + dx;
            let py = y + dy;
            if px < width && py < height {
                let index = ((py * width + px) * 4) as usize;
                if index + 3 < frame.len() {
                    frame[index..index + 4].copy_from_slice(&color);
                }
            }
        }
    }
}