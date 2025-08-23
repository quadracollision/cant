use std::collections::HashMap;
use crate::ball::Ball;
use crate::square::Square;

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
            GameObject::Ball(ball) => ball.get_position(),
            GameObject::Square(square) => square.get_position(),
        }
    }
}

#[derive(Clone, Debug)]
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
        let ball = Ball::new(x, y, speed, direction);
        let id = ball.id;
        
        self.objects.insert(id, GameObject::Ball(ball));
        self.balls.insert(id, id);
        id
    }
    
    pub fn create_square(&mut self, x: f64, y: f64) -> u32 {
        let square = Square::new(x, y);
        let id = square.id;
        
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
                ball.set_position(new_x, new_y);
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
    
    pub fn find_objects_at_grid_with_names(&self, grid_x: u32, grid_y: u32) -> Vec<String> {
        let mut object_names = Vec::new();
        let tolerance = 0.5;
        
        for (_id, obj) in &self.objects {
            match obj {
                GameObject::Ball(ball) => {
                    let (obj_x, obj_y) = ball.get_position();
                    if (obj_x - grid_x as f64).abs() <= tolerance && (obj_y - grid_y as f64).abs() <= tolerance {
                        object_names.push(ball.get_friendly_name());
                    }
                }
                GameObject::Square(square) => {
                    let (obj_x, obj_y) = square.get_position();
                    if (obj_x - grid_x as f64).abs() <= tolerance && (obj_y - grid_y as f64).abs() <= tolerance {
                        object_names.push(square.get_friendly_name());
                    }
                }
            }
        }
        
        object_names
    }
    
    pub fn find_object_by_name(&self, name: &str) -> Option<u32> {
        for (id, obj) in &self.objects {
            match obj {
                GameObject::Ball(ball) => {
                    if ball.get_friendly_name() == name {
                        return Some(*id);
                    }
                },
                GameObject::Square(_) => {
                    // Could add square naming later
                }
            }
        }
        None
    }
    
    pub fn set_ball_direction(&mut self, object_id: u32, direction_radians: f64) -> Result<(), String> {
        if let Some(GameObject::Ball(ball)) = self.objects.get_mut(&object_id) {
            ball.set_direction(direction_radians);
            Ok(())
        } else {
            Err("Object is not a ball or does not exist".to_string())
        }
    }
    
    pub fn get_all_squares(&self) -> Vec<Square> {
        self.objects.values()
            .filter_map(|obj| match obj {
                GameObject::Square(square) => Some(square.clone()),
                _ => None,
            })
            .collect()
    }
    
    pub fn get_all_ball_ids(&self) -> Vec<u32> {
        self.balls.keys().cloned().collect()
    }
    
    pub fn get_ball_mut(&mut self, ball_id: u32) -> Option<&mut Ball> {
        if let Some(GameObject::Ball(ball)) = self.objects.get_mut(&ball_id) {
            Some(ball)
        } else {
            None
        }
    }

    pub fn clear_all_balls(&mut self) -> usize {
        let ball_ids: Vec<u32> = self.balls.keys().cloned().collect();
        let count = ball_ids.len();
        
        for ball_id in ball_ids {
            self.objects.remove(&ball_id);
            self.balls.remove(&ball_id);
        }
        
        count
    }

    pub fn clear_all_squares(&mut self) -> usize {
        let square_ids: Vec<u32> = self.squares.keys().cloned().collect();
        let count = square_ids.len();
        
        for square_id in square_ids {
            self.objects.remove(&square_id);
            self.squares.remove(&square_id);
        }
        
        count
    }
}