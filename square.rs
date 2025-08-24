use std::sync::atomic::{AtomicU32, Ordering};

// Start square IDs from 2000 to avoid conflicts with balls
static NEXT_SQUARE_ID: AtomicU32 = AtomicU32::new(2000);
// Sequential counter for friendly naming (square1, square2, etc.)
static SQUARE_SEQUENCE: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub struct Square {
    pub id: u32,
    pub sequence_number: u32, // For friendly naming like "square1", "square2"
    pub x: f64,
    pub y: f64,
    pub script: Option<String>, // script to execute when collided with
    pub color: String, // New: store the color as a string
}

impl Square {
    pub fn new(x: f64, y: f64) -> Self {
        let id = NEXT_SQUARE_ID.fetch_add(1, Ordering::SeqCst);
        let sequence_number = SQUARE_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            sequence_number,
            x,
            y,
            script: None,
            color: "white".to_string(), // Default color
        }
    }
    
    pub fn get_friendly_name(&self) -> String {
        format!("square{}", self.sequence_number)
    }
    
    pub fn get_position(&self) -> (f64, f64) {
        (self.x, self.y)
    }
    
    pub fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }
    
    pub fn set_color(&mut self, color: String) {
        self.color = color;
    }
    
    pub fn get_color(&self) -> &str {
        &self.color
    }
}