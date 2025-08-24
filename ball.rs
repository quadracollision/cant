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
    pub audio_file: Option<String>, // path to audio file
    pub audio_volume: f32, // volume level (0.0 to 1.0)
    pub color: String, // New: store the color as a string
}

// Add to existing Ball implementation
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
            audio_file: None,
            audio_volume: 1.0,
            color: "white".to_string(), // Default color
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
    
    pub fn set_direction(&mut self, direction_radians: f64) {
        self.direction = direction_radians;
        self.velocity_x = self.speed * direction_radians.cos();
        self.velocity_y = self.speed * direction_radians.sin();
    }
    
    pub fn set_direction_from_string(direction: &str) -> f64 {
        use std::f64::consts::PI;
        match direction {
            "right" => 0.0,
            "up-right" | "right-up" => -PI / 4.0,
            "up" => -PI / 2.0,
            "up-left" | "left-up" => -3.0 * PI / 4.0,
            "left" => PI,
            "down-left" | "left-down" => 3.0 * PI / 4.0,
            "down" => PI / 2.0,
            "down-right" | "right-down" => PI / 4.0,
            _ => 0.0, // default to right
        }
    }
    
    pub fn update_direction_from_velocity(&mut self) {
        self.direction = self.velocity_y.atan2(self.velocity_x);
    }
    
    pub fn set_audio_file(&mut self, file_path: String) {
        self.audio_file = Some(file_path);
    }
    
    pub fn set_audio_volume(&mut self, volume: f32) {
        self.audio_volume = volume.clamp(0.0, 1.0);
    }
    
    pub fn play_collision_audio(&self) {
        if let Some(ref audio_file) = self.audio_file {
            if let Err(e) = crate::audio_engine::play_audio_sample(audio_file, self.audio_volume) {
                log::warn!("Failed to play audio for {}: {}", self.get_friendly_name(), e);
            }
        }
    }
    
    pub fn load_audio_file<P: AsRef<std::path::Path>>(&mut self, file_path: P) -> Result<(), crate::audio_engine::AudioError> {
        let sample_key = crate::audio_engine::load_audio_file(&file_path)?;
        self.audio_file = Some(sample_key);
        Ok(())
    }
    
    pub fn set_color(&mut self, color: String) {
        self.color = color;
    }
    
    pub fn get_color(&self) -> &str {
        &self.color
    }
}