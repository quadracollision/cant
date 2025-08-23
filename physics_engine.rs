use crate::ball::Ball;
use crate::square::Square;
use crate::grid::GridState;

pub struct PhysicsEngine {
    pub grid_width: f64,
    pub grid_height: f64,
    pub tile_size: f64,
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

    pub fn update_ball(&self, ball: &mut Ball, dt: f64, squares: &[Square]) {
        // Update position
        ball.x += ball.velocity_x * dt;
        ball.y += ball.velocity_y * dt;

        // Check grid boundary collisions
        self.check_boundary_collision(ball);
        
        // Check square collisions
        self.check_square_collisions(ball, squares);
    }

    fn check_boundary_collision(&self, ball: &mut Ball) {
        let ball_radius = 0.4; // Ball radius in grid units
        
        // Grid boundaries: 0 to grid_width (actual grid cell edges)
        if ball.x - ball_radius <= 0.0 || ball.x + ball_radius >= self.grid_width {
            ball.velocity_x = -ball.velocity_x;
            ball.x = ball.x.clamp(ball_radius, self.grid_width - ball_radius);
        }
        
        if ball.y - ball_radius <= 0.0 || ball.y + ball_radius >= self.grid_height {
            ball.velocity_y = -ball.velocity_y;
            ball.y = ball.y.clamp(ball_radius, self.grid_height - ball_radius);
        }
        
        ball.update_direction_from_velocity();
    }

    fn check_square_collisions(&self, ball: &mut Ball, squares: &[Square]) {
        let ball_radius = 0.4; // Grid units
        
        for square in squares {
            if self.ball_square_collision(ball, square, ball_radius) {
                // Simple collision response - reverse both velocity components
                ball.velocity_x = -ball.velocity_x;
                ball.velocity_y = -ball.velocity_y;
                ball.update_direction_from_velocity();
                break; // Handle one collision per frame
            }
        }
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