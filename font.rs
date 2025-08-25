//! System font rendering module using ab_glyph
//! Provides scalable, high-quality font rendering

use ab_glyph::{FontRef, PxScale, point, Font};
use std::sync::OnceLock;

/// Font renderer using the font file from assets
pub struct FontRenderer {
    font: FontRef<'static>,
    base_size: f32,
}

static FONT_RENDERER: OnceLock<FontRenderer> = OnceLock::new();

// Load the font from assets folder
const FONT_DATA: &[u8] = include_bytes!("../assets/courier.ttf");

impl FontRenderer {
    pub fn new() -> Self {
        let font = FontRef::try_from_slice(FONT_DATA)
            .expect("Failed to load font from assets/courier.ttf");
        
        Self {
            font,
            base_size: 14.0,
        }
    }
    
    /// Get character dimensions for a given scale
    pub fn get_char_dimensions(&self, scale: f32) -> (usize, usize) {
        let px_scale = PxScale::from(self.base_size * scale);
        let glyph_id = self.font.glyph_id('M');
        let scaled_glyph = glyph_id.with_scale(px_scale);
        
        let h_advance = self.font.h_advance_unscaled(glyph_id) * px_scale.x / self.font.units_per_em().unwrap_or(1000.0);
        let v_metrics = self.font.height_unscaled() * px_scale.y / self.font.units_per_em().unwrap_or(1000.0);
        
        (h_advance as usize, v_metrics as usize)
    }
    
    /// Get line height for a given scale
    pub fn get_line_height(&self, scale: f32) -> usize {
        let px_scale = PxScale::from(self.base_size * scale);
        let v_metrics = self.font.height_unscaled() * px_scale.y / self.font.units_per_em().unwrap_or(1000.0);
        (v_metrics * 1.2) as usize // Add 20% line spacing
    }
    
    /// Render a single character
    pub fn draw_char(&self, frame: &mut [u8], ch: char, x: usize, y: usize, color: [u8; 3], window_width: usize, scale: f32) {
        let px_scale = PxScale::from(self.base_size * scale);
        let glyph_id = self.font.glyph_id(ch);
        let scaled_glyph = glyph_id.with_scale(px_scale);
        
        if let Some(outlined) = self.font.outline_glyph(scaled_glyph) {
            let bounds = outlined.px_bounds();
            
            outlined.draw(|gx, gy, coverage| {
                if coverage > 0.0 {
                    let px = x as i32 + gx as i32 + bounds.min.x as i32;
                    let py = y as i32 + gy as i32 + bounds.min.y as i32;
                    
                    if px >= 0 && py >= 0 {
                        let px = px as usize;
                        let py = py as usize;
                        
                        if px < window_width && py < frame.len() / (window_width * 4) {
                            let idx = (py * window_width + px) * 4;
                            if idx + 3 < frame.len() {
                                let alpha = (coverage * 255.0) as u8;
                                if alpha > 0 {
                                    // Alpha blending
                                    let inv_alpha = 255 - alpha;
                                    frame[idx] = ((frame[idx] as u16 * inv_alpha as u16 + color[2] as u16 * alpha as u16) / 255) as u8; // B
                                    frame[idx + 1] = ((frame[idx + 1] as u16 * inv_alpha as u16 + color[1] as u16 * alpha as u16) / 255) as u8; // G
                                    frame[idx + 2] = ((frame[idx + 2] as u16 * inv_alpha as u16 + color[0] as u16 * alpha as u16) / 255) as u8; // R
                                    frame[idx + 3] = 255; // A
                                }
                            }
                        }
                    }
                }
            });
        }
    }
    
    /// Render text with optional selection highlighting
    pub fn draw_text(&self, frame: &mut [u8], text: &str, x: usize, y: usize, color: [u8; 3], selected: bool, window_width: usize, scale: f32) {
        let (char_width, char_height) = self.get_char_dimensions(scale);
        
        // Draw selection background if selected
        if selected {
            let selection_color = [100, 100, 200]; // Light blue background
            let text_width = text.len() * char_width;
            
            for py in y..y + char_height {
                for px in x..x + text_width {
                    if px < window_width && py < frame.len() / (window_width * 4) {
                        let idx = (py * window_width + px) * 4;
                        if idx + 3 < frame.len() {
                            frame[idx] = selection_color[2];     // B
                            frame[idx + 1] = selection_color[1]; // G
                            frame[idx + 2] = selection_color[0]; // R
                            frame[idx + 3] = 255;                // A
                        }
                    }
                }
            }
        }
        
        // Draw each character
        let mut current_x = x;
        for ch in text.chars() {
            if current_x + char_width <= window_width {
                self.draw_char(frame, ch, current_x, y, color, window_width, scale);
                current_x += char_width;
            } else {
                break; // Stop if we would go off screen
            }
        }
    }
    
    /// Draw syntax highlighted text (placeholder - uses regular text for now)
    pub fn draw_syntax_highlighted_text(&self, frame: &mut [u8], text: &str, x: usize, y: usize, window_width: usize, scale: f32) {
        self.draw_text(frame, text, x, y, [255, 255, 255], false, window_width, scale);
    }
    
    /// Draw cursor
    pub fn draw_cursor(&self, frame: &mut [u8], x: usize, y: usize, window_width: usize, scale: f32) {
        let (char_width, char_height) = self.get_char_dimensions(scale);
        let cursor_color = [255, 255, 255]; // White cursor
        
        // Draw a vertical line cursor
        for py in y..y + char_height {
            for px in x..x + 2 { // 2 pixel wide cursor
                if px < window_width && py < frame.len() / (window_width * 4) {
                    let idx = (py * window_width + px) * 4;
                    if idx + 3 < frame.len() {
                        frame[idx] = cursor_color[2];     // B
                        frame[idx + 1] = cursor_color[1]; // G
                        frame[idx + 2] = cursor_color[0]; // R
                        frame[idx + 3] = 255;             // A
                    }
                }
            }
        }
    }
    
    /// Draw tagged text (placeholder - uses regular text for now)
    pub fn draw_tagged_text(&self, frame: &mut [u8], text: &str, x: usize, y: usize, window_width: usize, scale: f32) {
        self.draw_text(frame, text, x, y, [255, 255, 255], false, window_width, scale);
    }
}

// Global font renderer instance
pub fn get_font() -> &'static FontRenderer {
    FONT_RENDERER.get_or_init(|| FontRenderer::new())
}

// Convenience functions for backward compatibility
pub fn draw_text(frame: &mut [u8], text: &str, x: usize, y: usize, color: [u8; 3], selected: bool, window_width: usize) {
    get_font().draw_text(frame, text, x, y, color, selected, window_width, 1.0);
}

pub fn draw_text_scaled(frame: &mut [u8], text: &str, x: usize, y: usize, color: [u8; 3], selected: bool, window_width: usize, scale: f32) {
    get_font().draw_text(frame, text, x, y, color, selected, window_width, scale);
}

pub fn draw_char(frame: &mut [u8], ch: char, x: usize, y: usize, color: [u8; 3], window_width: usize) {
    get_font().draw_char(frame, ch, x, y, color, window_width, 1.0);
}

pub fn draw_char_scaled(frame: &mut [u8], ch: char, x: usize, y: usize, color: [u8; 3], window_width: usize, scale: f32) {
    get_font().draw_char(frame, ch, x, y, color, window_width, scale);
}

pub fn draw_syntax_highlighted_text(frame: &mut [u8], text: &str, x: usize, y: usize, window_width: usize) {
    get_font().draw_syntax_highlighted_text(frame, text, x, y, window_width, 1.0);
}

pub fn draw_cursor(frame: &mut [u8], x: usize, y: usize, window_width: usize) {
    get_font().draw_cursor(frame, x, y, window_width, 1.0);
}

pub fn draw_tagged_text(frame: &mut [u8], text: &str, x: usize, y: usize, window_width: usize) {
    get_font().draw_tagged_text(frame, text, x, y, window_width, 1.0);
}

pub fn get_char_dimensions(scale: f32) -> (usize, usize) {
    get_font().get_char_dimensions(scale)
}

pub fn get_line_height(scale: f32) -> usize {
    get_font().get_line_height(scale)
}