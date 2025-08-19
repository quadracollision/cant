use std::sync::atomic::{AtomicU32, Ordering};

// Start ball IDs from 1000 to avoid conflicts with squares
static NEXT_BALL_ID: AtomicU32 = AtomicU32::new(1000);
// Sequential counter for friendly naming (ball1, ball2, etc.)
static BALL_SEQUENCE: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub struct Ball {
    pub id: u32,
    pub sequence_number: u32, // For friendly naming like "ball1", "ball2"
    pub x: f64,
    pub y: f64,
    pub speed: f64,
    pub direction: f64, // angle in radians
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub script: Option<String>, // script to execute on collision
}

impl Ball {
    pub fn new(x: f64, y: f64, speed: f64, direction: f64) -> Self {
        let id = NEXT_BALL_ID.fetch_add(1, Ordering::SeqCst);
        let sequence_number = BALL_SEQUENCE.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            sequence_number,
            x,
            y,
            speed,
            direction,
            velocity_x: speed * direction.cos(),
            velocity_y: speed * direction.sin(),
            script: None,
        }
    }
    
    pub fn get_friendly_name(&self) -> String {
        format!("ball{}", self.sequence_number)
    }
    
    pub fn update_physics(&mut self, dt: f64) {
        self.x += self.velocity_x * dt;
        self.y += self.velocity_y * dt;
    }
    
    pub fn get_position(&self) -> (f64, f64) {
        (self.x, self.y)
    }
    
    pub fn set_position(&mut self, x: f64, y: f64) {
        self.x = x;
        self.y = y;
    }
    
    pub fn set_velocity(&mut self, vx: f64, vy: f64) {
        self.velocity_x = vx;
        self.velocity_y = vy;
        self.speed = (vx * vx + vy * vy).sqrt();
        self.direction = vy.atan2(vx);
    }
}