use crate::ball::Ball;
use crate::square::Square;
use crate::grid::GridState;

pub struct PhysicsEngine {
    pub grid_width: f64,
    pub grid_height: f64,
    pub tile_size: f64,
}

pub struct CollisionInfo {
    pub ball_id: u32,
    pub collision_type: CollisionType,
    pub other_object_id: Option<u32>, // For square collisions
}

pub enum CollisionType {
    Wall,
    Square,
}

enum CollisionResult {
    None,
    Square { collision_point: (f64, f64), square_id: u32 },
    Wall { collision_point: (f64, f64) },
}

impl PhysicsEngine {
    pub fn new(grid_width: f64, grid_height: f64, tile_size: f64) -> Self {
        Self { grid_width, grid_height, tile_size }
    }
    
    // Add this method to update boundaries when grid changes
    pub fn update_grid_size(&mut self, width: f64, height: f64) {
        self.grid_width = width;
        self.grid_height = height;
    }

    pub fn update_ball(&self, ball: &mut Ball, dt: f64, squares: &[Square]) -> Vec<CollisionInfo> {
        let mut collisions = Vec::new();
        
        // Calculate intended new position
        let new_x = ball.x + ball.velocity_x * dt;
        let new_y = ball.y + ball.velocity_y * dt;
        
        // Check for collisions along the movement path
        let collision_result = self.check_collision_path(ball, new_x, new_y, squares);
        
        match collision_result {
            CollisionResult::None => {
                // No collision, move to intended position
                ball.x = new_x;
                ball.y = new_y;
            },
            CollisionResult::Square { collision_point, square_id } => {
                // Move to exact collision point
                ball.x = collision_point.0;
                ball.y = collision_point.1;
                
                // Handle collision physics
                ball.velocity_x = -ball.velocity_x;
                ball.velocity_y = -ball.velocity_y;
                
                // Add separation to prevent getting stuck
                let ball_radius = 0.4;
                let separation_distance = 0.01; // Small separation to prevent overlap
                
                // Find the square that was hit
                if let Some(square) = squares.iter().find(|s| s.id == square_id) {
                    let square_center_x = square.x + 0.5;
                    let square_center_y = square.y + 0.5;
                    
                    // Calculate direction from square center to ball
                    let dx = ball.x - square_center_x;
                    let dy = ball.y - square_center_y;
                    let distance = (dx * dx + dy * dy).sqrt();
                    
                    if distance > 0.0 {
                        // Normalize and apply separation
                        let norm_dx = dx / distance;
                        let norm_dy = dy / distance;
                        
                        // Move ball away from square by separation distance
                        ball.x += norm_dx * separation_distance;
                        ball.y += norm_dy * separation_distance;
                    }
                }
                
                ball.update_direction_from_velocity();
                ball.play_collision_audio();
                
                collisions.push(CollisionInfo {
                    ball_id: ball.id,
                    collision_type: CollisionType::Square,
                    other_object_id: Some(square_id),
                });
            },
            CollisionResult::Wall { collision_point } => {
                // Move to exact collision point
                ball.x = collision_point.0;
                ball.y = collision_point.1;
                
                // Handle wall collision physics directly
                let ball_radius = 0.4;
                let separation_distance = 0.01; // Small separation to prevent overlap
                
                // Determine which wall was hit and reverse appropriate velocity + add separation
                if ball.x - ball_radius <= 0.0 {
                    ball.velocity_x = -ball.velocity_x;
                    ball.x = ball_radius + separation_distance; // Move away from left wall
                } else if ball.x + ball_radius >= self.grid_width {
                    ball.velocity_x = -ball.velocity_x;
                    ball.x = self.grid_width - ball_radius - separation_distance; // Move away from right wall
                }
                
                if ball.y - ball_radius <= 0.0 {
                    ball.velocity_y = -ball.velocity_y;
                    ball.y = ball_radius + separation_distance; // Move away from top wall
                } else if ball.y + ball_radius >= self.grid_height {
                    ball.velocity_y = -ball.velocity_y;
                    ball.y = self.grid_height - ball_radius - separation_distance; // Move away from bottom wall
                }
                
                ball.update_direction_from_velocity();
                ball.play_collision_audio();
                
                collisions.push(CollisionInfo {
                    ball_id: ball.id,
                    collision_type: CollisionType::Wall,
                    other_object_id: None,
                });
            }
        }
        
        collisions
    }
    
    fn check_collision_path(&self, ball: &Ball, target_x: f64, target_y: f64, squares: &[Square]) -> CollisionResult {
        let ball_radius = 0.4;
        
        // Check square collisions first (they take priority)
        for square in squares {
            if let Some(collision_point) = self.calculate_collision_point(ball, target_x, target_y, square, ball_radius) {
                return CollisionResult::Square { 
                    collision_point, 
                    square_id: square.id 
                };
            }
        }
        
        // Check wall collisions
        if let Some(collision_point) = self.calculate_wall_collision_point(ball, target_x, target_y, ball_radius) {
            return CollisionResult::Wall { collision_point };
        }
        
        CollisionResult::None
    }
    
    fn calculate_collision_point(&self, ball: &Ball, target_x: f64, target_y: f64, square: &Square, ball_radius: f64) -> Option<(f64, f64)> {
        // Ray-box intersection to find exact collision point
        let square_left = square.x;
        let square_right = square.x + 1.0;
        let square_top = square.y;
        let square_bottom = square.y + 1.0;
        
        // Expand square bounds by ball radius
        let expanded_left = square_left - ball_radius;
        let expanded_right = square_right + ball_radius;
        let expanded_top = square_top - ball_radius;
        let expanded_bottom = square_bottom + ball_radius;
        
        // Ray from current position to target
        let dx = target_x - ball.x;
        let dy = target_y - ball.y;
        
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        
        // Calculate intersection times for each edge
        let t_left = if dx != 0.0 { (expanded_left - ball.x) / dx } else { f64::INFINITY };
        let t_right = if dx != 0.0 { (expanded_right - ball.x) / dx } else { f64::INFINITY };
        let t_top = if dy != 0.0 { (expanded_top - ball.y) / dy } else { f64::INFINITY };
        let t_bottom = if dy != 0.0 { (expanded_bottom - ball.y) / dy } else { f64::INFINITY };
        
        // Find the earliest valid intersection
        let mut min_t = f64::INFINITY;
        
        for &t in &[t_left, t_right, t_top, t_bottom] {
            if t >= 0.0 && t <= 1.0 && t < min_t {
                let collision_x = ball.x + dx * t;
                let collision_y = ball.y + dy * t;
                
                // Verify the collision point is actually on the square boundary
                if collision_x >= expanded_left && collision_x <= expanded_right &&
                   collision_y >= expanded_top && collision_y <= expanded_bottom {
                    min_t = t;
                }
            }
        }
        
        if min_t < f64::INFINITY {
            Some((ball.x + dx * min_t, ball.y + dy * min_t))
        } else {
            None
        }
    }
    
    fn calculate_wall_collision_point(&self, ball: &Ball, target_x: f64, target_y: f64, ball_radius: f64) -> Option<(f64, f64)> {
        let dx = target_x - ball.x;
        let dy = target_y - ball.y;
        
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        
        let mut min_t = f64::INFINITY;
        
        // Check each wall
        if dx != 0.0 {
            // Left wall
            let t = (ball_radius - ball.x) / dx;
            if t >= 0.0 && t <= 1.0 && t < min_t {
                min_t = t;
            }
            
            // Right wall
            let t = (self.grid_width - ball_radius - ball.x) / dx;
            if t >= 0.0 && t <= 1.0 && t < min_t {
                min_t = t;
            }
        }
        
        if dy != 0.0 {
            // Top wall
            let t = (ball_radius - ball.y) / dy;
            if t >= 0.0 && t <= 1.0 && t < min_t {
                min_t = t;
            }
            
            // Bottom wall
            let t = (self.grid_height - ball_radius - ball.y) / dy;
            if t >= 0.0 && t <= 1.0 && t < min_t {
                min_t = t;
            }
        }
        
        if min_t < f64::INFINITY {
            Some((ball.x + dx * min_t, ball.y + dy * min_t))
        } else {
            None
        }
    }
    
    fn check_square_collisions(&self, ball: &mut Ball, squares: &[Square]) -> Option<u32> {
        let ball_radius = 0.4;
        
        for square in squares {
            if self.ball_square_collision(ball, square, ball_radius) {
                ball.velocity_x = -ball.velocity_x;
                ball.velocity_y = -ball.velocity_y;
                ball.update_direction_from_velocity();
                return Some(square.id); // Return the square ID that was hit
            }
        }
        None
    }

    fn check_boundary_collision(&self, ball: &mut Ball) -> bool {
        let ball_radius = 0.4; // Ball radius in grid units
        let mut collision_occurred = false;
        
        // Grid boundaries: 0 to grid_width (actual grid cell edges)
        if ball.x - ball_radius <= 0.0 || ball.x + ball_radius >= self.grid_width {
            ball.velocity_x = -ball.velocity_x;
            ball.x = ball.x.clamp(ball_radius, self.grid_width - ball_radius);
            collision_occurred = true;
        }
        
        if ball.y - ball_radius <= 0.0 || ball.y + ball_radius >= self.grid_height {
            ball.velocity_y = -ball.velocity_y;
            ball.y = ball.y.clamp(ball_radius, self.grid_height - ball_radius);
            collision_occurred = true;
        }
        
        if collision_occurred {
            ball.update_direction_from_velocity();
        }
        
        collision_occurred
    }

    fn ball_square_collision(&self, ball: &Ball, square: &Square, ball_radius: f64) -> bool {
        // AABB collision detection in grid coordinates
        let square_left = square.x;
        let square_right = square.x + 1.0; // Each square is 1 grid unit
        let square_top = square.y;
        let square_bottom = square.y + 1.0;
        
        ball.x + ball_radius >= square_left &&
        ball.x - ball_radius <= square_right &&
        ball.y + ball_radius >= square_top &&
        ball.y - ball_radius <= square_bottom
    }
}