use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Debug, Clone)]
pub struct Ball {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub speed: f64,
    pub direction: f64, // angle in radians
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub script: Option<String>, // script to execute on collision
}

#[derive(Debug, Clone)]
pub struct Square {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub script: Option<String>, // script to execute when collided with
}

#[derive(Debug, Clone)]
pub enum GameObject {
    Ball(Ball),
    Square(Square),
}

impl GameObject {
    pub fn get_id(&self) -> u32 {
        match self {
            GameObject::Ball(ball) => ball.id,
            GameObject::Square(square) => square.id,
        }
    }
    
    pub fn get_position(&self) -> (f64, f64) {
        match self {
            GameObject::Ball(ball) => (ball.x, ball.y),
            GameObject::Square(square) => (square.x, square.y),
        }
    }
}

pub struct GameObjectManager {
    objects: HashMap<u32, GameObject>,
    balls: HashMap<u32, u32>, // ball_id -> object_id mapping
    squares: HashMap<u32, u32>, // square_id -> object_id mapping
}

impl GameObjectManager {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            balls: HashMap::new(),
            squares: HashMap::new(),
        }
    }
    
    pub fn create_ball(&mut self, x: f64, y: f64, speed: f64, direction: f64) -> u32 {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let ball = Ball {
            id,
            x,
            y,
            speed,
            direction,
            velocity_x: speed * direction.cos(),
            velocity_y: speed * direction.sin(),
            script: None,
        };
        
        self.objects.insert(id, GameObject::Ball(ball));
        self.balls.insert(id, id);
        id
    }
    
    pub fn create_square(&mut self, x: f64, y: f64) -> u32 {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let square = Square {
            id,
            x,
            y,
            script: None,
        };
        
        self.objects.insert(id, GameObject::Square(square));
        self.squares.insert(id, id);
        id
    }
    
    pub fn destroy_object(&mut self, id: u32) -> bool {
        if let Some(obj) = self.objects.remove(&id) {
            match obj {
                GameObject::Ball(_) => { self.balls.remove(&id); }
                GameObject::Square(_) => { self.squares.remove(&id); }
            }
            true
        } else {
            false
        }
    }
    
    pub fn find_object_at(&self, x: f64, y: f64, tolerance: f64) -> Option<u32> {
        for (id, obj) in &self.objects {
            let (obj_x, obj_y) = obj.get_position();
            let distance = ((obj_x - x).powi(2) + (obj_y - y).powi(2)).sqrt();
            if distance <= tolerance {
                return Some(*id);
            }
        }
        None
    }
    
    pub fn get_object(&self, id: u32) -> Option<&GameObject> {
        self.objects.get(&id)
    }
    
    pub fn get_all_objects(&self) -> &HashMap<u32, GameObject> {
        &self.objects
    }
    
    pub fn update_ball_physics(&mut self, dt: f64) {
        let mut updates = Vec::new();
        
        for (id, obj) in &self.objects {
            if let GameObject::Ball(ball) = obj {
                let new_x = ball.x + ball.velocity_x * dt;
                let new_y = ball.y + ball.velocity_y * dt;
                updates.push((*id, new_x, new_y));
            }
        }
        
        for (id, new_x, new_y) in updates {
            if let Some(GameObject::Ball(ball)) = self.objects.get_mut(&id) {
                ball.x = new_x;
                ball.y = new_y;
            }
        }
    }
    
    pub fn check_collisions(&self) -> Vec<(u32, u32)> {
        let mut collisions = Vec::new();
        let objects: Vec<_> = self.objects.iter().collect();
        
        for i in 0..objects.len() {
            for j in i+1..objects.len() {
                let (id1, obj1) = objects[i];
                let (id2, obj2) = objects[j];
                
                // Check if one is a ball and one is a square
                if matches!((obj1, obj2), (GameObject::Ball(_), GameObject::Square(_)) | (GameObject::Square(_), GameObject::Ball(_))) {
                    let (x1, y1) = obj1.get_position();
                    let (x2, y2) = obj2.get_position();
                    let distance = ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt();
                    
                    if distance <= 1.0 { // collision threshold
                        collisions.push((*id1, *id2));
                    }
                }
            }
        }
        
        collisions
    }
}