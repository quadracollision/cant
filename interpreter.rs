use std::collections::HashMap;
use thiserror::Error;
use crate::grid::GridState;
use crate::lexer::{Lexer, LexerError, Token, TokenType};
use crate::parser::{Parser, ParseError};
use crate::ast::*;
use crate::game_objects::{GameObjectManager, GameObject};
use crate::physics_engine::{PhysicsEngine, CollisionInfo, CollisionType};
use crate::game_state::GameStateManager;
use crate::console::Console;
use crate::script_editor::ScriptEditor;
use crate::ball::Ball;
use crate::square::Square;

#[derive(Error, Debug)]
pub enum InterpreterError {
    #[error("Lexer error: {0}")]
    LexerError(#[from] LexerError),
    #[error("Parser error: {0}")]
    ParseError(#[from] ParseError),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),
    #[error("Undefined function: {0}")]
    UndefinedFunction(String),
    #[error("Type error: {0}")]
    TypeError(String),
    #[error("Return value: {0:?}")]
    Return(Value),
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
    Function {
        name: String,
        parameters: Vec<String>,
        body: Box<Stmt>,
    },
    GameObject(u32), // Reference to game object by ID
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Nil => false,
            _ => true,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Boolean(b) => b.to_string(),
            Value::Nil => "nil".to_string(),
            Value::Function { name, .. } => format!("<function {}>", name),
            Value::GameObject(id) => format!("<object {}>", id),
        }
    }
    
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }


}

pub struct Interpreter {
    grid_state: Option<GridState>,
    globals: HashMap<String, Value>,
    environment: HashMap<String, Value>,
    game_objects: GameObjectManager,
    game_state_manager: GameStateManager,
    physics_engine: PhysicsEngine,
    cursor_x: u32,
    cursor_y: u32,
    script_editor: Option<ScriptEditor>,
    current_script_owner: Option<u32>,
    verbose_mode: bool,
    graphics_update_needed: bool,
    // Add in-memory script storage
    memory_scripts: HashMap<String, String>, // script_name -> script_content
    next_script_id: u32, // Add script ID counter to interpreter too
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            grid_state: None,
            globals: HashMap::new(),
            environment: HashMap::new(),
            game_objects: GameObjectManager::new(),
            game_state_manager: GameStateManager::new(),
            physics_engine: PhysicsEngine::new(10.0, 10.0, 50.0), // Default grid: 10x10 with 50px tiles
            cursor_x: 0,
            cursor_y: 0,
            script_editor: None,
            current_script_owner: None,
            verbose_mode: false,
            graphics_update_needed: false,
            memory_scripts: HashMap::new(),
            next_script_id: 1,
        };
        interpreter.register_builtins();
        interpreter
    }

    fn list_memory_scripts(&self) -> Vec<String> {
        self.memory_scripts.keys().cloned().collect()
    }

    fn get_script_from_memory(&self, script_name: &str) -> Option<&String> {
        self.memory_scripts.get(script_name)
    }

    pub fn save_script_to_memory(&mut self, script_name: String, content: String) {
        self.memory_scripts.insert(script_name, content);
    }

    pub fn remove_script_from_memory(&mut self, script_name: &str) -> Option<String> {
        self.memory_scripts.remove(script_name)
    }

    // Update the execute_play method
    fn execute_play(&mut self) -> Result<Value, InterpreterError> {
        if self.game_state_manager.is_paused() {
            // Resume from paused state
            self.game_state_manager.start_play();
            Ok(Value::String("Game resumed".to_string()))
        } else if !self.game_state_manager.is_playing() {
            // Starting fresh or from stopped state - always save current state as original
            self.game_state_manager.save_original_state(
                &self.game_objects,
                &self.grid_state,
                &self.environment
            );
            
            self.game_state_manager.start_play();
            Ok(Value::String("Game started".to_string()))
        } else {
            // Already playing
            Ok(Value::String("Game is already playing".to_string()))
        }
    }
    
    // Update the execute_pause method
    fn execute_pause(&mut self) -> Result<Value, InterpreterError> {
        if self.game_state_manager.is_playing() {
            // Save current state before pausing
            self.game_state_manager.save_paused_state(
                &self.game_objects,
                &self.grid_state,
                &self.environment
            );
            self.game_state_manager.pause_play();
            Ok(Value::String("Game paused".to_string()))
        } else {
            Ok(Value::String("Game is not currently playing".to_string()))
        }
    }
    
    // Update the execute_stop method
    fn execute_stop(&mut self) -> Result<Value, InterpreterError> {
        // Stop the physics simulation
        self.game_state_manager.stop_play();
        
        // Restore the original saved state if it exists
        if let Some(saved) = self.game_state_manager.get_saved_state() {
            self.game_objects = saved.game_objects.clone();
            self.grid_state = saved.grid_state.clone();
            self.environment = saved.environment.clone();
            Ok(Value::String("Game stopped and state restored to original".to_string()))
        } else {
            Ok(Value::String("Game stopped (no saved state to restore)".to_string()))
        }
    }

    pub fn is_playing(&self) -> bool {
        self.game_state_manager.is_playing()
    }

    pub fn update_physics(&mut self, dt: f64) {
        if self.is_playing() {
            let squares = self.game_objects.get_all_squares();
            let mut all_collisions = Vec::new();
            
            for ball_id in self.game_objects.get_all_ball_ids() {
                if let Some(ball) = self.game_objects.get_ball_mut(ball_id) {
                    let collisions = self.physics_engine.update_ball(ball, dt, &squares);
                    all_collisions.extend(collisions);
                }
            }
            
            // Process physics collisions
            for collision in all_collisions {
                match collision.collision_type {
                    CollisionType::Wall => {
                        // Record wall hit for the ball
                        if let Some(ball) = self.game_objects.get_ball_mut(collision.ball_id) {
                            ball.record_hit(0); // Use 0 or special ID for walls
                        }
                        
                        if self.verbose_mode {
                            println!("{}: wall collision", 
                                self.game_objects.get_ball_name(collision.ball_id).unwrap_or("unknown".to_string()));
                        }
                    },
                    CollisionType::Square => {
                        if let Some(square_id) = collision.other_object_id {
                            // Record hits for both objects
                            if let Some(ball) = self.game_objects.get_ball_mut(collision.ball_id) {
                                ball.record_hit(square_id);
                            }
                            if let Some(square) = self.game_objects.get_square_mut(square_id) {
                                square.record_hit(collision.ball_id);
                            }
                            
                            if self.verbose_mode {
                                self.print_collision_info(collision.ball_id, square_id);
                            }
                            
                            self.execute_collision_script(collision.ball_id, square_id);
                        }
                    }
                }
            }
        }
    }

    fn register_builtins(&mut self) {
        // Built-in functions will be handled specially in function calls
    }

    pub fn execute_command(&mut self, input: &str, cursor_x: u32, cursor_y: u32) -> Result<String, InterpreterError> {
        if input.trim().is_empty() {
            return Ok(String::new());
        }

        // Update cursor position
        self.cursor_x = cursor_x;
        self.cursor_y = cursor_y;

        // Tokenize
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;

        // Parse
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;

        // Execute
        let mut result = Value::Nil;
        for statement in program.statements {
            result = self.execute_statement(&statement)?;
        }

        Ok(result.to_string())
    }

    fn execute_statement(&mut self, stmt: &Stmt) -> Result<Value, InterpreterError> {
        match stmt {
            Stmt::Expression(expr) => self.evaluate_expression(expr),
            Stmt::Let { name, initializer } => {
                let value = if let Some(init) = initializer {
                    self.evaluate_expression(init)?
                } else {
                    Value::Nil
                };
                self.environment.insert(name.clone(), value.clone());
                Ok(value)
            },
            Stmt::Block(statements) => {
                let mut result = Value::Nil;
                for statement in statements {
                    result = self.execute_statement(statement)?;
                }
                Ok(result)
            },
            Stmt::If { condition, then_branch, else_branch } => {
                let condition_value = self.evaluate_expression(condition)?;
                
                // Check if this is a hits condition followed by a threshold
                if let Expr::Binary { left: _, operator: BinaryOp::Hits, right: _ } = condition {
                    // Look ahead to see if the first statement in then_branch is a number (threshold)
                    if let Stmt::Block(statements) = then_branch.as_ref() {
                        if let Some(Stmt::Expression(Expr::Number(threshold))) = statements.first() {
                            // Compare hit count with threshold
                            if let Value::Number(hit_count) = condition_value {
                                if hit_count >= *threshold {
                                    // Execute the rest of the then_branch (skip the threshold number)
                                    for stmt in statements.iter().skip(1) {
                                        self.execute_statement(stmt)?;
                                    }
                                }
                            } else if let Some(else_branch) = else_branch {
                                self.execute_statement(else_branch)?;
                            }
                            return Ok(Value::Nil);
                        }
                    }
                }
                
                // Normal if statement logic
                if condition_value.is_truthy() {
                    self.execute_statement(then_branch)
                } else if let Some(else_stmt) = else_branch {
                    self.execute_statement(else_stmt)
                } else {
                    Ok(Value::Nil)
                }
            },
            Stmt::While { condition, body } => {
                let mut result = Value::Nil;
                while self.evaluate_expression(condition)?.is_truthy() {
                    result = self.execute_statement(body)?;
                }
                Ok(result)
            },
            Stmt::Function { name, parameters, body } => {
                let function = Value::Function {
                    name: name.clone(),
                    parameters: parameters.clone(),
                    body: body.clone(),
                };
                self.environment.insert(name.clone(), function.clone());
                Ok(function)
            },
            Stmt::Return(expr) => {
                let value = if let Some(e) = expr {
                    self.evaluate_expression(e)?
                } else {
                    Value::Nil
                };
                Err(InterpreterError::Return(value))
            },
            Stmt::SetDirection { object_name, direction } => {
                self.execute_set_direction(object_name, direction)
            },
            Stmt::SetColor { object_name, color } => {
                self.execute_set_color(object_name, color)
            },
            Stmt::SetSpeed { object_name, speed } => {
                self.execute_set_speed(object_name, speed)
            },
            Stmt::Label { object_name, arguments, text } => {
                self.execute_label(object_name, arguments, text)
            },
            Stmt::Script { object_name, arguments } => {
                self.execute_script_command(object_name, arguments)
            },
            Stmt::Play => self.execute_play(),
            Stmt::Pause => self.execute_pause(),
            Stmt::Stop => self.execute_stop(),
            Stmt::Verbose => self.execute_verbose(),
            Stmt::ClearBalls => self.execute_clear_balls(),
            Stmt::ClearSquares => self.execute_clear_squares(),
            Stmt::Destroy { object_type, arguments } => {  // Add this
                self.execute_destroy(object_type, arguments)
            },
            Stmt::Run { script_name } => self.execute_run_command(script_name),
        }
    }

    fn execute_destroy(&mut self, object_type: &str, arguments: &[Expr]) -> Result<Value, InterpreterError> {
        if arguments.len() != 1 {
            return Err(InterpreterError::RuntimeError("destroy expects 1 argument".to_string()));
        }
        
        let arg_value = self.evaluate_expression(&arguments[0])?;
        
        match arg_value {
            Value::String(s) if s.starts_with("cursor:") => {
                // Extract cursor coordinates and find objects at that position
                let parts: Vec<&str> = s.split(':').collect();
                if parts.len() == 3 {
                    let cursor_x = parts[1].parse::<u32>().unwrap_or(0);
                    let cursor_y = parts[2].parse::<u32>().unwrap_or(0);
                    
                    // Find objects at cursor position
                    let objects_at_cursor = self.game_objects.find_objects_at_grid_with_names(cursor_x, cursor_y);
                    
                    if objects_at_cursor.is_empty() {
                        return Ok(Value::String("No objects found at cursor position".to_string()));
                    }
                    
                    // Filter by object type and destroy the first match
                    for obj_name in &objects_at_cursor {
                        if obj_name.starts_with(object_type) {
                            if let Some(obj_id) = self.game_objects.find_object_by_name(obj_name) {
                                self.game_objects.destroy_object(obj_id);
                                return Ok(Value::String(format!("Destroyed {} at cursor position", obj_name)));
                            }
                        }
                    }
                    
                    return Ok(Value::String(format!("No {} found at cursor position", object_type)));
                } else {
                    return Err(InterpreterError::RuntimeError("Invalid cursor format".to_string()));
                }
            },
            Value::Number(x) if arguments.len() == 2 => {
                // Handle destroy ball(0, 0) syntax
                let y_value = self.evaluate_expression(&arguments[1])?;
                if let Value::Number(y) = y_value {
                    let objects_at_pos = self.game_objects.find_objects_at_grid_with_names(x as u32, y as u32);
                    
                    for obj_name in &objects_at_pos {
                        if obj_name.starts_with(object_type) {
                            if let Some(obj_id) = self.game_objects.find_object_by_name(obj_name) {
                                self.game_objects.destroy_object(obj_id);
                                return Ok(Value::String(format!("Destroyed {} at ({}, {})", obj_name, x, y)));
                            }
                        }
                    }
                    
                    return Ok(Value::String(format!("No {} found at ({}, {})", object_type, x, y)));
                }
            },
            _ => {
                return Err(InterpreterError::TypeError("destroy expects cursor position or coordinates".to_string()));
            }
        }
        
        Ok(Value::String("Destroy command completed".to_string()))
    }

    fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, InterpreterError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Self_ => {
                if let Some(owner_id) = self.current_script_owner {
                    Ok(Value::GameObject(owner_id))
                } else {
                    Err(InterpreterError::RuntimeError("'self' can only be used within object scripts".to_string()))
                }
            },
            Expr::Identifier(name) => {
                // Handle special cursor identifier
                if name == "cursor" {
                    // Return cursor position as a special value that can be used in create/destroy
                    return Ok(Value::String(format!("cursor:{}:{}", self.cursor_x, self.cursor_y)));
                }
                
                if let Some(value) = self.environment.get(name) {
                    Ok(value.clone())
                } else if let Some(value) = self.globals.get(name) {
                    Ok(value.clone())
                } else {
                    Err(InterpreterError::UndefinedVariable(name.clone()))
                }
            },
            Expr::Binary { left, operator, right } => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                self.apply_binary_operator(operator, left_val, right_val)
            },
            Expr::Unary { operator, operand } => {
                let operand_val = self.evaluate_expression(operand)?;
                self.apply_unary_operator(operator, operand_val)
            },
            Expr::Call { callee, arguments } => {
                if let Expr::Identifier(function_name) = callee.as_ref() {
                    self.call_function(function_name, arguments)
                } else {
                    Err(InterpreterError::RuntimeError("Only function names can be called".to_string()))
                }
            },
            Expr::CreateCall { object_type, arguments } => {
                match object_type.as_str() {
                    "ball" => {
                        let (start_x, start_y) = if arguments.len() >= 1 {
                            let first_arg = self.evaluate_expression(&arguments[0])?;
                            
                            // Check if first argument is cursor
                            if let Value::String(s) = &first_arg {
                                if s.starts_with("cursor:") {
                                    // Extract cursor coordinates
                                    let parts: Vec<&str> = s.split(':').collect();
                                    if parts.len() == 3 {
                                        let cursor_x = parts[1].parse::<f64>().unwrap_or(0.0);
                                        let cursor_y = parts[2].parse::<f64>().unwrap_or(0.0);
                                        // Place ball at center of the grid cell (add 0.5 for cell center)
                                        (cursor_x + 0.5, cursor_y + 0.5)
                                    } else {
                                        return Err(InterpreterError::RuntimeError("Invalid cursor format".to_string()));
                                    }
                                } else {
                                    return Err(InterpreterError::TypeError("Expected cursor or coordinates".to_string()));
                                }
                            } else if arguments.len() >= 2 {
                                // Use provided x,y coordinates
                                let x = first_arg.as_number()
                                    .ok_or_else(|| InterpreterError::TypeError("Ball x coordinate must be a number".to_string()))?;
                                let y = self.evaluate_expression(&arguments[1])?.as_number()
                                    .ok_or_else(|| InterpreterError::TypeError("Ball y coordinate must be a number".to_string()))?;
                                (x + 0.5, y + 0.5)
                            } else {
                                return Err(InterpreterError::RuntimeError("Ball creation with single non-cursor argument not supported".to_string()));
                            }
                        } else {
                            // Create ball at center of current grid if grid exists (no arguments)
                            if let Some(ref grid) = self.grid_state {
                                // Center the ball in the middle cell by adding 0.5 to place it in cell center
                                ((grid.width as f64 / 2.0) - 0.5, (grid.height as f64 / 2.0) - 0.5)
                            } else {
                                // Use physics engine boundaries as fallback
                                ((self.physics_engine.grid_width / 2.0) - 0.5, (self.physics_engine.grid_height / 2.0) - 0.5)
                            }
                        };
                        
                        let id = self.game_objects.create_ball(start_x, start_y, 5.0, 0.0);
                        
                        // Get the ball's friendly name and store it in the environment
                        if let Some(ball_name) = self.game_objects.get_ball_name(id) {
                            self.environment.insert(ball_name, Value::GameObject(id));
                        }
                        
                        return Ok(Value::GameObject(id));
                    },
                    "square" => {
                        if let Some(ref grid) = self.grid_state {
                            let (x, y) = if arguments.len() >= 1 {
                                let first_arg = self.evaluate_expression(&arguments[0])?;
                                
                                // Check if first argument is cursor
                                if let Value::String(s) = &first_arg {
                                    if s.starts_with("cursor:") {
                                        // Extract cursor coordinates
                                        let parts: Vec<&str> = s.split(':').collect();
                                        if parts.len() == 3 {
                                            let cursor_x = parts[1].parse::<f64>().unwrap_or(0.0);
                                            let cursor_y = parts[2].parse::<f64>().unwrap_or(0.0);
                                            (cursor_x, cursor_y)
                                        } else {
                                            return Err(InterpreterError::RuntimeError("Invalid cursor format".to_string()));
                                        }
                                    } else {
                                        return Err(InterpreterError::TypeError("Expected cursor or coordinates".to_string()));
                                    }
                                } else if arguments.len() >= 2 {
                                    // Use provided x,y coordinates
                                    let x = first_arg.as_number()
                                        .ok_or_else(|| InterpreterError::TypeError("Square x coordinate must be a number".to_string()))?;
                                    let y = self.evaluate_expression(&arguments[1])?.as_number()
                                        .ok_or_else(|| InterpreterError::TypeError("Square y coordinate must be a number".to_string()))?;
                                    (x, y)
                                } else {
                                    return Err(InterpreterError::RuntimeError("create square requires cursor or x,y coordinates".to_string()));
                                }
                            } else {
                                // Default to center
                                ((grid.width as f64 / 2.0), (grid.height as f64 / 2.0))
                            };
                            let id = self.game_objects.create_square(x, y);
                            
                            // Get the square's friendly name and store it in the environment
                            if let Some(GameObject::Square(square)) = self.game_objects.get_object(id) {
                                let square_name = square.get_friendly_name();
                                self.environment.insert(square_name, Value::GameObject(id));
                            }
                            
                            Ok(Value::GameObject(id))
                        } else {
                            Err(InterpreterError::RuntimeError("No grid available for square creation".to_string()))
                        }
                    },
                    _ => Err(InterpreterError::RuntimeError(format!("Unknown object type: {}", object_type)))
                }
            },
            Expr::Assignment { name, value } => {
                let val = self.evaluate_expression(value)?;
                self.environment.insert(name.clone(), val.clone());
                Ok(val)
            },
        }
    }

    fn apply_binary_operator(&self, op: &BinaryOp, left: Value, right: Value) -> Result<Value, InterpreterError> {
        match (left, right) {
            (Value::Number(l), Value::Number(r)) => {
                match op {
                    BinaryOp::Add => Ok(Value::Number(l + r)),
                    BinaryOp::Subtract => Ok(Value::Number(l - r)),
                    BinaryOp::Multiply => Ok(Value::Number(l * r)),
                    BinaryOp::Divide => {
                        if r == 0.0 {
                            Err(InterpreterError::RuntimeError("Division by zero".to_string()))
                        } else {
                            Ok(Value::Number(l / r))
                        }
                    },
                    BinaryOp::Equal => Ok(Value::Boolean(l == r)),
                    BinaryOp::NotEqual => Ok(Value::Boolean(l != r)),
                    BinaryOp::Less => Ok(Value::Boolean(l < r)),
                    BinaryOp::Greater => Ok(Value::Boolean(l > r)),
                    BinaryOp::LessEqual => Ok(Value::Boolean(l <= r)),
                    BinaryOp::GreaterEqual => Ok(Value::Boolean(l >= r)),
                    BinaryOp::Hits => Err(InterpreterError::TypeError("Hits operator requires game objects".to_string())),
                }
            },
            (Value::String(l), Value::String(r)) => {
                match op {
                    BinaryOp::Add => Ok(Value::String(format!("{}{}", l, r))),
                    BinaryOp::Equal => Ok(Value::Boolean(l == r)),
                    BinaryOp::NotEqual => Ok(Value::Boolean(l != r)),
                    _ => Err(InterpreterError::TypeError("Invalid operation for strings".to_string())),
                }
            },
            (Value::GameObject(obj1_id), Value::GameObject(obj2_id)) => {
            match op {
                BinaryOp::Hits => {
                    // Return the actual hit count between two game objects
                    let key = format!("hits({},{})", obj1_id, obj2_id);
                    if let Some(Value::Number(count)) = self.environment.get(&key) {
                        Ok(Value::Number(*count))
                    } else {
                        Ok(Value::Number(0.0))
                    }
                },
                BinaryOp::Equal => Ok(Value::Boolean(obj1_id == obj2_id)),
                BinaryOp::NotEqual => Ok(Value::Boolean(obj1_id != obj2_id)),
                _ => Err(InterpreterError::TypeError("Invalid operation for game objects".to_string())),
            }
        },
            _ => Err(InterpreterError::TypeError("Type mismatch in binary operation".to_string())),
        }
    }

    fn apply_unary_operator(&self, op: &UnaryOp, operand: Value) -> Result<Value, InterpreterError> {
        match op {
            UnaryOp::Minus => {
                if let Value::Number(n) = operand {
                    Ok(Value::Number(-n))
                } else {
                    Err(InterpreterError::TypeError("Cannot negate non-number".to_string()))
                }
            },
            UnaryOp::Not => Ok(Value::Boolean(!operand.is_truthy())),
        }
    }

    pub fn get_grid_state_mut(&mut self) -> Option<&mut GridState> {
        self.grid_state.as_mut()
    }
    
    pub fn get_grid_state(&self) -> Option<&GridState> {
        self.grid_state.as_ref()
    }
    
    pub fn get_environment_value(&self, key: &str) -> Option<String> {
        self.environment.get(key).map(|v| v.to_string())
    }
    
    // Add this new method
    pub fn remove_environment_value(&mut self, key: &str) -> Option<Value> {
        self.environment.remove(key)
    }
    
    // Add this method for debugging
    pub fn get_all_environment_values(&self) -> &HashMap<String, Value> {
        &self.environment
    }
    
    fn call_function(&mut self, name: &str, arguments: &[Expr]) -> Result<Value, InterpreterError> {
        // Check for built-in functions first
        match name {
            "grid" => return self.call_grid_function(arguments),
            "tilesize" => return self.call_tilesize_function(arguments),
            "font_size" => return self.call_font_size_function(arguments),
            "sample" => return self.call_sample_function(arguments),
            "hits" => {
                if arguments.len() == 1 {
                    // Original single-parameter hits() - returns total hits for an object
                    let object_name = match &arguments[0] {
                        Expr::Identifier(name) => name.clone(),
                        Expr::Self_ => {
                            if let Some(owner_id) = self.current_script_owner {
                                if let Some(name) = self.game_objects.get_square_name(owner_id) {
                                    name
                                } else {
                                    return Err(InterpreterError::RuntimeError("Script owner not found".to_string()));
                                }
                            } else {
                                return Err(InterpreterError::RuntimeError("'self' used outside of script context".to_string()));
                            }
                        },
                        _ => {
                            let target_value = self.evaluate_expression(&arguments[0])?;
                            match target_value {
                                Value::String(obj_name) => obj_name,
                                Value::GameObject(id) => {
                                    if let Some(name) = self.game_objects.get_ball_name(id) {
                                        name
                                    } else if let Some(name) = self.game_objects.get_square_name(id) {
                                        name
                                    } else {
                                        return Err(InterpreterError::RuntimeError(format!("Object with ID {} not found", id)));
                                    }
                                },
                                _ => return Err(InterpreterError::TypeError("hits() expects an object name or identifier".to_string())),
                            }
                        }
                    };
                    
                    let object_id = self.game_objects.find_object_by_name(&object_name)
                        .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object_name)))?;
                    
                    let total_hits = if let Some(GameObject::Ball(ball)) = self.game_objects.get_object(object_id) {
                        ball.get_total_hits() as f64
                    } else if let Some(GameObject::Square(square)) = self.game_objects.get_object(object_id) {
                        square.get_total_hits() as f64
                    } else {
                        return Err(InterpreterError::RuntimeError(format!("Object '{}' not found", object_name)));
                    };
                    
                    return Ok(Value::Number(total_hits));
                } else if arguments.len() == 2 {
                    // New two-parameter hits(object1, object2) - returns hit count between specific objects
                    let mut get_object_name = |arg: &Expr| -> Result<String, InterpreterError> {
                        match arg {
                            Expr::Identifier(name) => Ok(name.clone()),
                            Expr::Self_ => {
                                if let Some(owner_id) = self.current_script_owner {
                                    if let Some(name) = self.game_objects.get_square_name(owner_id) {
                                        Ok(name)
                                    } else {
                                        Err(InterpreterError::RuntimeError("Script owner not found".to_string()))
                                    }
                                } else {
                                    Err(InterpreterError::RuntimeError("'self' used outside of script context".to_string()))
                                }
                            },
                            _ => {
                                let target_value = self.evaluate_expression(arg)?;
                                match target_value {
                                    Value::String(obj_name) => Ok(obj_name),
                                    Value::GameObject(id) => {
                                        if let Some(name) = self.game_objects.get_ball_name(id) {
                                            Ok(name)
                                        } else if let Some(name) = self.game_objects.get_square_name(id) {
                                            Ok(name)
                                        } else {
                                            Err(InterpreterError::RuntimeError(format!("Object with ID {} not found", id)))
                                        }
                                    },
                                    _ => Err(InterpreterError::TypeError("hits() expects an object name or identifier".to_string())),
                                }
                            }
                        }
                    };
                    
                    let object1_name = get_object_name(&arguments[0])?;
                    let object2_name = get_object_name(&arguments[1])?;
                    
                    let object1_id = self.game_objects.find_object_by_name(&object1_name)
                        .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object1_name)))?;
                    let object2_id = self.game_objects.find_object_by_name(&object2_name)
                        .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object2_name)))?;
                    
                    // Get hit count from object1 hitting object2
                    let hit_count = if let Some(GameObject::Ball(ball)) = self.game_objects.get_object(object1_id) {
                        ball.get_hit_count(object2_id) as f64
                    } else if let Some(GameObject::Square(square)) = self.game_objects.get_object(object1_id) {
                        square.get_hit_count(object2_id) as f64
                    } else {
                        return Err(InterpreterError::RuntimeError(format!("Object '{}' not found", object1_name)));
                    };
                    
                    return Ok(Value::Number(hit_count));
                } else {
                    return Err(InterpreterError::RuntimeError("hits expects 1 or 2 arguments".to_string()));
                }
            },
        "speed" => {
            if arguments.len() != 1 {
                return Err(InterpreterError::RuntimeError("speed expects exactly 1 argument".to_string()));
            }
            
            let object_name = match &arguments[0] {
                Expr::Identifier(name) => name.clone(),
                _ => {
                    let target_value = self.evaluate_expression(&arguments[0])?;
                    match target_value {
                        Value::String(ball_name) => ball_name,
                        _ => return Err(InterpreterError::TypeError("speed() expects a ball name as identifier".to_string())),
                    }
                }
            };
            
            let object_id = self.game_objects.find_object_by_name(&object_name)
                .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object_name)))?;
            
            let current_speed = self.game_objects.get_ball_speed(object_id)
                .map_err(|e| InterpreterError::RuntimeError(e))?;
            
            return Ok(Value::Number(current_speed));
        },
            "clear" => {
                self.grid_state = None;
                return Ok(Value::String("Grid cleared".to_string()));
            },
            "help" => return Ok(Value::String(self.show_help())),
            "lib" | "library" => {
                if arguments.is_empty() {
                    // List all memory scripts
                    let scripts = self.list_memory_scripts();
                    if scripts.is_empty() {
                        return Ok(Value::String("No scripts in memory".to_string()));
                    } else {
                        let list = scripts.join(", ");
                        return Ok(Value::String(format!("Memory scripts: {}", list)));
                    }
                } else {
                    // Get specific script name
                    let script_name = self.evaluate_expression(&arguments[0])?.to_string();
                    if let Some(content) = self.get_script_from_memory(&script_name) {
                        // Open the memory script in the editor
                        self.script_editor = Some(ScriptEditor::new(0, Some(content.clone())));
                        return Ok(Value::String(format!("Opened memory script: {}", script_name)));
                    } else {
                        return Err(InterpreterError::RuntimeError(format!("Memory script '{}' not found", script_name)));
                    }
                }
            },
            // In the "create" function around line 398-408
            "ball" => {
                // Create ball at center of current grid if grid exists
                let (start_x, start_y) = if let Some(ref grid) = self.grid_state {
                    // Center the ball in the middle cell by adding 0.5 to place it in cell center
                    ((grid.width as f64 / 2.0) - 0.5, (grid.height as f64 / 2.0) - 0.5)
                } else {
                    // Use physics engine boundaries as fallback
                    ((self.physics_engine.grid_width / 2.0) - 0.5, (self.physics_engine.grid_height / 2.0) - 0.5)
                };
                let id = self.game_objects.create_ball(start_x, start_y, 5.0, 0.0);
                
                // Get the ball's friendly name and store it in the environment
                if let Some(ball_name) = self.game_objects.get_ball_name(id) {
                    self.environment.insert(ball_name, Value::GameObject(id));
                }
                
                return Ok(Value::GameObject(id));
            },
            "destroy" => {
                if arguments.len() != 1 {
                    return Err(InterpreterError::RuntimeError("destroy expects 1 argument".to_string()));
                }
                
                let arg_value = self.evaluate_expression(&arguments[0])?;
                
                match arg_value {
                    Value::GameObject(id) => {
                        self.game_objects.destroy_object(id);
                        return Ok(Value::String("Object destroyed".to_string()));
                    },
                    Value::String(s) if s.starts_with("cursor:") => {
                        // Extract cursor coordinates and find objects at that position
                        let parts: Vec<&str> = s.split(':').collect();
                        if parts.len() == 3 {
                            let cursor_x = parts[1].parse::<u32>().unwrap_or(0);
                            let cursor_y = parts[2].parse::<u32>().unwrap_or(0);
                            
                            // Find objects at cursor position
                            let objects_at_cursor = self.game_objects.find_objects_at_grid_with_names(cursor_x, cursor_y);
                            
                            if objects_at_cursor.is_empty() {
                                return Ok(Value::String("No objects found at cursor position".to_string()));
                            }
                            
                            // Destroy the first object found (could be enhanced to specify type)
                            if let Some(obj_name) = objects_at_cursor.first() {
                                if let Some(obj_id) = self.game_objects.find_object_by_name(obj_name) {
                                    self.game_objects.destroy_object(obj_id);
                                    return Ok(Value::String(format!("Destroyed {} at cursor position", obj_name)));
                                }
                            }
                            
                            return Ok(Value::String("Failed to destroy object at cursor".to_string()));
                        } else {
                            return Err(InterpreterError::RuntimeError("Invalid cursor format".to_string()));
                        }
                    },
                    _ => {
                        return Err(InterpreterError::TypeError("destroy expects a game object or cursor position".to_string()));
                    }
                }
            },
            _ => {}
        }

        // Check for user-defined functions
        if let Some(function) = self.environment.get(name).cloned() {
            if let Value::Function { parameters, body, .. } = function {
                if arguments.len() != parameters.len() {
                    return Err(InterpreterError::RuntimeError(
                        format!("Function {} expects {} arguments, got {}", name, parameters.len(), arguments.len())
                    ));
                }

                // Evaluate arguments
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(self.evaluate_expression(arg)?);
                }

                // Save current environment
                let saved_env = self.environment.clone();

                // Set up function parameters
                for (param, value) in parameters.iter().zip(arg_values.iter()) {
                    self.environment.insert(param.clone(), value.clone());
                }

                // Execute function body
                let result = match self.execute_statement(&body) {
                    Ok(value) => Ok(value),
                    Err(InterpreterError::Return(value)) => Ok(value),
                    Err(e) => Err(e),
                };

                // Restore environment
                self.environment = saved_env;

                result
            } else {
                Err(InterpreterError::TypeError(format!("{} is not a function", name)))
            }
        } else {
            Err(InterpreterError::UndefinedFunction(name.to_string()))
        }
    }

    fn call_grid_function(&mut self, arguments: &[Expr]) -> Result<Value, InterpreterError> {
        let is_script_context = self.current_script_owner.is_some();
        if arguments.len() == 2 {
            let x_val = self.evaluate_expression(&arguments[0])?;
            let y_val = self.evaluate_expression(&arguments[1])?;
            let x = if let Value::Number(n) = x_val {
                if n.fract() == 0.0 && n > 0.0 && n <= 100.0 {
                    n as u32
                } else {
                    return Err(InterpreterError::RuntimeError(
                        "Grid x must be a positive integer <= 100".to_string()
                    ));
                }
            } else {
                return Err(InterpreterError::TypeError(
                    "Grid x must be a number".to_string()
                ));
            };
            let y = if let Value::Number(n) = y_val {
                if n.fract() == 0.0 && n > 0.0 && n <= 100.0 {
                    n as u32
                } else {
                    return Err(InterpreterError::RuntimeError(
                        "Grid y must be a positive integer <= 100".to_string()
                    ));
                }
            } else {
                return Err(InterpreterError::TypeError(
                    "Grid y must be a number".to_string()
                ));
            };
            self.grid_state = Some(GridState::new(x, y));
            self.physics_engine.update_grid_size(x as f64, y as f64);
            
            // Add this line to flag that graphics need updating
            if self.current_script_owner.is_some() {
                self.graphics_update_needed = true;
            }
            
            Ok(Value::String(format!("Created {}x{} grid", x, y)))
        } else if arguments.len() == 3 && is_script_context {
            let x_val = self.evaluate_expression(&arguments[0])?;
            let y_val = self.evaluate_expression(&arguments[1])?;
            let z_val = self.evaluate_expression(&arguments[2])?;
            let x = if let Value::Number(n) = x_val {
                if n.fract() == 0.0 && n > 0.0 && n <= 100.0 {
                    n as u32
                } else {
                    return Err(InterpreterError::RuntimeError(
                        "Grid x must be a positive integer <= 100".to_string()
                    ));
                }
            } else {
                return Err(InterpreterError::TypeError(
                    "Grid x must be a number".to_string()
                ));
            };
            let y = if let Value::Number(n) = y_val {
                if n.fract() == 0.0 && n > 0.0 && n <= 100.0 {
                    n as u32
                } else {
                    return Err(InterpreterError::RuntimeError(
                        "Grid y must be a positive integer <= 100".to_string()
                    ));
                }
            } else {
                return Err(InterpreterError::TypeError(
                    "Grid y must be a number".to_string()
                ));
            };
            let z = if let Value::Number(n) = z_val {
                if n.fract() == 0.0 && n >= 0.0 {
                    n as u32
                } else {
                    return Err(InterpreterError::RuntimeError(
                        "Grid center origin z must be a non-negative integer".to_string()
                    ));
                }
            } else {
                return Err(InterpreterError::TypeError(
                    "Grid center origin z must be a number".to_string()
                ));
            };
            self.grid_state = Some(GridState::new_with_center(x, y, z));
            self.physics_engine.update_grid_size(x as f64, y as f64);
            
            // Add this line to flag that graphics need updating
            if self.current_script_owner.is_some() {
                self.graphics_update_needed = true;
            }
            
            Ok(Value::String(format!("Created {}x{} grid with center origin at {}", x, y, z)))
        } else {
            let expected_args = if is_script_context { "2 or 3" } else { "2" };
            return Err(InterpreterError::RuntimeError(
                format!("grid() requires exactly {} arguments", expected_args)
            ));
        }
    }

    fn call_tilesize_function(&mut self, arguments: &[Expr]) -> Result<Value, InterpreterError> {
        if arguments.len() != 1 {
            return Err(InterpreterError::RuntimeError(
                "tilesize() requires exactly one argument".to_string()
            ));
        }
        
        let size_value = self.evaluate_expression(&arguments[0])?;
        
        match size_value {
            Value::Number(size) => {
                if size < 4.0 || size > 100.0 {
                    return Err(InterpreterError::RuntimeError(
                        "Tile size must be between 4 and 100 pixels".to_string()
                    ));
                }
                
                self.environment.insert("__tile_size".to_string(), Value::Number(size));
                
                Ok(Value::String(format!("Tile size set to {} pixels", size as u32)))
            },
            _ => {
                Err(InterpreterError::TypeError(
                    "tilesize() argument must be a number".to_string()
                ))
            }
        }
    }

    fn call_font_size_function(&mut self, arguments: &[Expr]) -> Result<Value, InterpreterError> {
        if arguments.len() != 1 {
            return Err(InterpreterError::RuntimeError(
                "font_size() requires exactly one argument".to_string()
            ));
        }
        
        let size_value = self.evaluate_expression(&arguments[0])?;
        
        match size_value {
            Value::Number(size) => {
                if size < 8.0 || size > 48.0 {
                    return Err(InterpreterError::RuntimeError(
                        "Font size must be between 8 and 48 pixels".to_string()
                    ));
                }
                
                self.environment.insert("__font_size".to_string(), Value::Number(size));
                
                Ok(Value::String(format!("Font size set to {}px", size as u32)))
            },
            _ => {
                Err(InterpreterError::TypeError(
                    "font_size() argument must be a number".to_string()
                ))
            }
        }
    }

    fn call_sample_function(&mut self, arguments: &[Expr]) -> Result<Value, InterpreterError> {
        if arguments.is_empty() {
            return Err(InterpreterError::RuntimeError("sample expects at least 1 argument".to_string()));
        }

        // Evaluate the target argument
        let target_value = self.evaluate_expression(&arguments[0])?;
        
        // Determine the target ball based on the argument
        let target_ball_id = match target_value {
            // Direct coordinates: sample(0, 0)
            Value::Number(x) => {
                if arguments.len() < 2 {
                    return Err(InterpreterError::RuntimeError("sample with coordinates expects 2 arguments (x, y)".to_string()));
                }
                let y_value = self.evaluate_expression(&arguments[1])?;
                if let Value::Number(y) = y_value {
                    // Find ball at the specified coordinates
                    self.game_objects.find_ball_at_position(x as u32, y as u32)
                } else {
                    return Err(InterpreterError::TypeError("Y coordinate must be a number".to_string()));
                }
            },
            // Cursor position: sample(cursor)
            Value::String(ref s) if s == "cursor" => {
                self.game_objects.find_ball_at_position(self.cursor_x, self.cursor_y)
            },
            // Ball name: sample(ball1)
            Value::String(ref ball_name) => {
                self.game_objects.find_object_by_name(ball_name)
            },
            // Direct ball object reference
            Value::GameObject(id) => {
                // Verify it's actually a ball
                if self.game_objects.is_ball(id) {
                    Some(id)
                } else {
                    return Err(InterpreterError::RuntimeError("Object is not a ball".to_string()));
                }
            },
            _ => {
                return Err(InterpreterError::TypeError("Invalid target for sample command".to_string()));
            }
        };

        let ball_id = match target_ball_id {
            Some(id) => id,
            None => {
                return Err(InterpreterError::RuntimeError("No ball found at specified location".to_string()));
            }
        };

        // Open file dialog to select audio file
        let file_path = match self.open_audio_file_dialog() {
            Some(path) => path,
            None => {
                return Ok(Value::String("File selection cancelled".to_string()));
            }
        };

        // Load the audio file into the ball
        match self.game_objects.load_audio_into_ball(ball_id, &file_path) {
            Ok(_) => {
                let ball_name = self.game_objects.get_ball_name(ball_id)
                    .unwrap_or_else(|| format!("ball{}", ball_id));
                Ok(Value::String(format!("Loaded audio file '{}' into {}", 
                    std::path::Path::new(&file_path).file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&file_path), 
                    ball_name)))
            },
            Err(e) => {
                Err(InterpreterError::RuntimeError(format!("Failed to load audio: {}", e)))
            }
        }
    }

    fn open_audio_file_dialog(&self) -> Option<String> {
        use rfd::FileDialog;
        
        FileDialog::new()
            .add_filter("Audio Files", &["wav", "mp3", "ogg", "flac", "m4a", "aac"])
            .add_filter("WAV Files", &["wav"])
            .add_filter("MP3 Files", &["mp3"])
            .add_filter("OGG Files", &["ogg"])
            .add_filter("FLAC Files", &["flac"])
            .set_title("Select Audio Sample")
            .pick_file()
            .and_then(|path| path.to_str().map(|s| s.to_string()))
    }

    fn show_help(&self) -> String {
        r#"Available commands:
  grid(width, height) - Create a grid
  tilesize(size) - Set tile size
  ball() - Create a ball
  sample(target) - Load audio file into ball
    - sample(0, 0) - Load audio into ball at coordinates
    - sample(cursor) - Load audio into ball at cursor
    - sample(ball1) - Load audio into specific ball
  clear - Clear the grid
  help - Show this help
  
Controls:
  Arrow keys - Move cursor
  Space - Toggle cell
  Enter - Execute command"#.to_string()
    }

    pub fn get_game_objects(&self) -> &GameObjectManager {
        &self.game_objects
    }

    pub fn is_script_editor_active(&self) -> bool {
        self.script_editor.as_ref().map_or(false, |editor| editor.is_active())
    }

    pub fn get_script_editor_display_lines(&self) -> Vec<String> {
        if let Some(editor) = &self.script_editor {
            editor.get_display_lines()
        } else {
            Vec::new()
        }
    }

    pub fn handle_script_editor_key(&mut self, key: &str) -> bool {
        let mut editor_closed = false;
        let mut target_id = 0;
        let mut script_content = String::new();
        let mut is_memory_script = false;
        let mut filename: Option<String> = None;
        let mut result = false;
        
        if let Some(editor) = &mut self.script_editor {
            result = editor.handle_key(key);
            
            // If script editor was closed (save or cancel), collect the data we need
            if !editor.is_active() {
                editor_closed = true;
                target_id = editor.get_target_object_id();
                script_content = editor.get_script_content();
                is_memory_script = editor.is_memory_script();
                filename = editor.get_filename().cloned();
            }
        }
        
        // Handle the script saving after we're done with the editor borrow
        if editor_closed {
            // Remove the script editor first
            self.script_editor = None;
            
            if is_memory_script {
                // Save to memory
                if let Some(filename) = filename {
                    self.save_script_to_memory(filename, script_content.clone());
                } else {
                    // Generate script ID for unnamed memory scripts
                    let script_id = format!("script{}", self.next_script_id);
                    self.next_script_id += 1;
                    self.save_script_to_memory(script_id, script_content.clone());
                }
            } else if target_id > 0 {
                // Save script to the target square (existing behavior)
                if let Some(square) = self.game_objects.get_square_mut(target_id) {
                    square.set_script(script_content);
                }
            }
        }
        
        result
    }

    pub fn update_script_editor_cursor(&mut self) {
        if let Some(editor) = &mut self.script_editor {
            editor.update_cursor_blink();
        }
    }

    fn execute_set_direction(&mut self, object_name: &str, direction: &DirectionValue) -> Result<Value, InterpreterError> {
        let object_id = if object_name == "cursor" {
            // Find object at cursor position
            let object_names_at_cursor = self.game_objects.find_objects_at_grid_with_names(self.cursor_x, self.cursor_y);
            if object_names_at_cursor.is_empty() {
                return Err(InterpreterError::RuntimeError("No object found at cursor position".to_string()));
            }
            // Use the first object found at cursor position and get its ID
            let first_object_name = &object_names_at_cursor[0];
            self.game_objects.find_object_by_name(first_object_name)
                .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", first_object_name)))?
        } else {
            // Find the object by name
            self.game_objects.find_object_by_name(object_name)
                .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object_name)))?
        };
        
        // Convert direction to angle
        let angle = match direction {
            DirectionValue::Left => std::f64::consts::PI,
            DirectionValue::Right => 0.0,
            DirectionValue::Up => -std::f64::consts::PI / 2.0,  // Changed from 3π/2 to -π/2
            DirectionValue::Down => std::f64::consts::PI / 2.0,  // This one was correct
            DirectionValue::UpLeft => -3.0 * std::f64::consts::PI / 4.0,  // Changed from 5π/4 to -3π/4
            DirectionValue::UpRight => -std::f64::consts::PI / 4.0,  // Changed from 7π/4 to -π/4
            DirectionValue::DownLeft => 3.0 * std::f64::consts::PI / 4.0,  // This one was correct
            DirectionValue::DownRight => std::f64::consts::PI / 4.0,  // This one was correct
        };
        
        self.game_objects.set_ball_direction(object_id, angle)
            .map_err(|e| InterpreterError::RuntimeError(e))?;
        
        let target_name = if object_name == "cursor" {
            format!("object at cursor position")
        } else {
            object_name.to_string()
        };
        
        Ok(Value::String(format!("Set direction of {} to {:?}", target_name, direction)))
    }

    fn execute_clear_balls(&mut self) -> Result<Value, InterpreterError> {
        let count = self.game_objects.clear_all_balls();
        Ok(Value::String(format!("Cleared {} ball(s)", count)))
    }

    fn execute_clear_squares(&mut self) -> Result<Value, InterpreterError> {
        let count = self.game_objects.clear_all_squares();
        Ok(Value::String(format!("Cleared {} square(s)", count)))
    }

    fn execute_set_color(&mut self, object_name: &str, color: &ColorValue) -> Result<Value, InterpreterError> {
        let color_string = match color {
            ColorValue::Red => "red".to_string(),
            ColorValue::Green => "green".to_string(),
            ColorValue::Blue => "blue".to_string(),
            ColorValue::Yellow => "yellow".to_string(),
            ColorValue::White => "white".to_string(),
            ColorValue::Black => "black".to_string(),
            ColorValue::Purple => "purple".to_string(),
            ColorValue::Orange => "orange".to_string(),
            ColorValue::Pink => "pink".to_string(),
            ColorValue::Brown => "brown".to_string(),
            ColorValue::Gray => "gray".to_string(),
            ColorValue::Cyan => "cyan".to_string(),
            ColorValue::Magenta => "magenta".to_string(),
            ColorValue::Lime => "lime".to_string(),
        };
    
    let object_id = if object_name == "cursor" {
        // Find object at cursor position
        let object_names_at_cursor = self.game_objects.find_objects_at_grid_with_names(self.cursor_x, self.cursor_y);
        println!("Debug: Objects at cursor ({}, {}): {:?}", self.cursor_x, self.cursor_y, object_names_at_cursor);
        
        if object_names_at_cursor.is_empty() {
            return Err(InterpreterError::RuntimeError("No object found at cursor position".to_string()));
        }
        // Use the first object found at cursor position and get its ID
        let first_object_name = &object_names_at_cursor[0];
        println!("Debug: First object name: {}", first_object_name);
        
        let found_id = self.game_objects.find_object_by_name(first_object_name)
            .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", first_object_name)))?;
        println!("Debug: Found object ID: {}", found_id);
        found_id
    } else {
        // Find the object by name
        self.game_objects.find_object_by_name(object_name)
            .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object_name)))?
    };
    
    // Apply the color to the actual game object using the object_id we found
    if let Some(ball) = self.game_objects.get_ball_mut(object_id) {
        println!("Debug: Ball {} current color: {}", object_id, ball.get_color());
        println!("Debug: Setting color on ball {} to {}", object_id, color_string);
        ball.set_color(color_string.clone());
        println!("Debug: Ball {} new color: {}", object_id, ball.get_color());
    } else if let Some(square) = self.game_objects.get_square_mut(object_id) {
        println!("Debug: Square {} current color: {}", object_id, square.get_color());
        println!("Debug: Setting color on square {} to {}", object_id, color_string);
        square.set_color(color_string.clone());
        println!("Debug: Square {} new color: {}", object_id, square.get_color());
    } else {
        println!("Debug: Object {} is neither a ball nor a square", object_id);
        return Err(InterpreterError::RuntimeError(format!("Object {} is neither a ball nor a square", object_id)));
    }
    
    let target_name = if object_name == "cursor" {
        format!("object at cursor position")
    } else {
        object_name.to_string()
    };
    
    Ok(Value::String(format!("Set color of {} to {:?}", target_name, color)))
}

fn execute_set_speed(&mut self, object_name: &str, speed_mod: &SpeedModification) -> Result<Value, InterpreterError> {
    let object_id = if object_name == "cursor" {
        // Find object at cursor position
        let object_names_at_cursor = self.game_objects.find_objects_at_grid_with_names(self.cursor_x, self.cursor_y);
        if object_names_at_cursor.is_empty() {
            return Err(InterpreterError::RuntimeError("No object found at cursor position".to_string()));
        }
        // Use the first object found at cursor position and get its ID
        let first_object_name = &object_names_at_cursor[0];
        self.game_objects.find_object_by_name(first_object_name)
            .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", first_object_name)))?
    } else {
        // Find the object by name
        self.game_objects.find_object_by_name(object_name)
            .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object_name)))?
    };
    
    let final_speed = match speed_mod {
        SpeedModification::Absolute(speed) => *speed,
        SpeedModification::Relative(delta) => {
            let current_speed = self.game_objects.get_ball_speed(object_id)
                .map_err(|e| InterpreterError::RuntimeError(e))?;
            (current_speed + delta).max(0.0) // Ensure speed doesn't go negative
        }
    };
    
    self.game_objects.set_ball_speed(object_id, final_speed)
        .map_err(|e| InterpreterError::RuntimeError(e))?;
    
    let target_name = if object_name == "cursor" {
        format!("object at cursor position")
    } else {
        object_name.to_string()
    };
    
    let operation_desc = match speed_mod {
        SpeedModification::Absolute(speed) => format!("Set speed of {} to {}", target_name, speed),
        SpeedModification::Relative(delta) => {
            if *delta >= 0.0 {
                format!("Increased speed of {} by {} (new speed: {})", target_name, delta, final_speed)
            } else {
                format!("Decreased speed of {} by {} (new speed: {})", target_name, delta.abs(), final_speed)
            }
        }
    };
    
    Ok(Value::String(operation_desc))
}

fn execute_script_command(&mut self, object_name: &str, arguments: &[Expr]) -> Result<Value, InterpreterError> {
    // Handle script(new) for creating blank scripts
    if object_name == "new" {
        self.script_editor = Some(ScriptEditor::new_memory_script(None));
        return Ok(Value::String("Blank script editor opened".to_string()));
    }
    
    // First, check memory scripts
    if let Some(content) = self.get_script_from_memory(object_name) {
        self.script_editor = Some(ScriptEditor::new_memory_script(Some(content.clone())));
        return Ok(Value::String(format!("Script editor opened with memory script: {}", object_name)));
    }
    
    // Then check disk files
    let filename = if object_name.ends_with(".cant") {
        object_name.to_string()
    } else {
        format!("{}.cant", object_name)
    };
    
    if std::path::Path::new(&filename).exists() {
        match std::fs::read_to_string(&filename) {
            Ok(script_content) => {
                // Use the base name (without .cant) as the display filename
                let base_name = if filename.ends_with(".cant") {
                    filename.trim_end_matches(".cant").to_string()
                } else {
                    filename.clone()
                };
                self.script_editor = Some(ScriptEditor::new_with_file(base_name, Some(script_content)));
                return Ok(Value::String(format!("Script editor opened with file: {}", filename)));
            },
            Err(e) => {
                return Err(InterpreterError::RuntimeError(format!("Error reading script file '{}': {}", filename, e)));
            }
        }
    }
    
    // Finally, try to find a game object (for collision scripts)
    let object_id = if object_name == "cursor" {
        self.game_objects.find_object_at(self.cursor_x as f64, self.cursor_y as f64, 0.5)
            .ok_or_else(|| InterpreterError::RuntimeError("No object at cursor position".to_string()))?
    } else {
        self.game_objects.find_object_by_name(object_name)
            .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object_name)))?
    };
    
    if let Some(square) = self.game_objects.get_square_mut(object_id) {
        let existing_script = square.get_script().map(|s| s.to_string());
        self.script_editor = Some(ScriptEditor::new(object_id, existing_script));
        Ok(Value::String("Script editor opened".to_string()))
    } else {
        Err(InterpreterError::RuntimeError("Only squares can have scripts".to_string()))
    }
}

pub fn handle_collisions(&mut self) {
    let collisions = self.game_objects.check_collisions();
    
    for (id1, id2) in collisions {
        // Record hits for both objects
        if let Some(ball) = self.game_objects.get_ball_mut(id1) {
            ball.record_hit(id2);  // Pass the other object's ID
        }
        if let Some(square) = self.game_objects.get_square_mut(id1) {
            square.record_hit(id2);  // Pass the other object's ID
        }
        if let Some(ball) = self.game_objects.get_ball_mut(id2) {
            ball.record_hit(id1);  // Pass the other object's ID
        }
        if let Some(square) = self.game_objects.get_square_mut(id2) {
            square.record_hit(id1);  // Pass the other object's ID
        }
        
        // Print verbose collision information if enabled
        if self.verbose_mode {
            self.print_collision_info(id1, id2);
        }
        
        // Execute collision scripts
        self.execute_collision_script(id1, id2);
    }
}

fn print_collision_info(&self, id1: u32, id2: u32) {
    // Print information for first object
    if let Some(obj) = self.game_objects.get_object(id1) {
        match obj {
            GameObject::Ball(ball) => {
                println!("{}: {} hits", ball.get_friendly_name(), ball.get_hit_count(id2));
            },
            GameObject::Square(square) => {
                println!("{}: {} hits", square.get_friendly_name(), square.get_hit_count(id2));
            }
        }
    }
    
    // Print information for second object
    if let Some(obj) = self.game_objects.get_object(id2) {
        match obj {
            GameObject::Ball(ball) => {
                println!("{}: {} hits", ball.get_friendly_name(), ball.get_hit_count(id1));
            },
            GameObject::Square(square) => {
                println!("{}: {} hits", square.get_friendly_name(), square.get_hit_count(id1));
            }
        }
    }
}

fn execute_collision_script(&mut self, id1: u32, id2: u32) {
        // Check collision types first without borrowing
        let is_ball1 = self.game_objects.get_ball_mut(id1).is_some();
        let is_ball2 = self.game_objects.get_ball_mut(id2).is_some();
        
        // Check for ball-square collision with script
        let collision_info = if is_ball1 && !is_ball2 {
            // id1 is ball, check if id2 is square with script
            if let Some(GameObject::Square(sq)) = self.game_objects.get_object(id2) {
                if sq.get_script().is_some() {
                    println!("Debug: Ball {} collided with square {} that has a script", id1, id2);
                    Some((id1, id2))
                } else { 
                    println!("Debug: Ball {} collided with square {} but no script", id1, id2);
                    None 
                }
            } else { None }
        } else if is_ball2 && !is_ball1 {
            // id2 is ball, check if id1 is square with script
            if let Some(GameObject::Square(sq)) = self.game_objects.get_object(id1) {
                if sq.get_script().is_some() {
                    println!("Debug: Ball {} collided with square {} that has a script", id2, id1);
                    Some((id2, id1))
                } else { 
                    println!("Debug: Ball {} collided with square {} but no script", id2, id1);
                    None 
                }
            } else { None }
        } else { None };
        
        if let Some((ball_id, square_id)) = collision_info {
            // Set the script execution context
            self.current_script_owner = Some(square_id);
            
            // Get script content and hit counts
            let script_content = if let Some(square) = self.game_objects.get_square_mut(square_id) {
                square.get_script().map(|s| s.to_string())
            } else { None };
            
            if let Some(script) = script_content {
                println!("Debug: Executing script: {}", script);
                let total_hits = if let Some(square) = self.game_objects.get_square_mut(square_id) {
                    square.get_total_hits()
                } else { 0 };
                
                let ball_hits = if let Some(square) = self.game_objects.get_square_mut(square_id) {
                    square.get_hit_count(ball_id)
                } else { 0 };
                
                // Set up script environment
                self.environment.insert("hits".to_string(), Value::Number(total_hits as f64));
                self.environment.insert(format!("hits({})", ball_id), Value::Number(ball_hits as f64));
                // Add the specific ball-square hit count for proper "ball1 hits self 3" evaluation
                self.environment.insert(format!("hits({},{})", ball_id, square_id), Value::Number(ball_hits as f64));
                
                // Parse and execute script commands
                let cursor_x = self.cursor_x;
                let cursor_y = self.cursor_y;
                if let Err(e) = self.execute_script_block(&script, cursor_x, cursor_y) {
                    eprintln!("Script execution error: {}", e);
                }
                
                // Clean up environment and context
                self.environment.remove("hits");
                self.environment.remove(&format!("hits({})", ball_id));
                self.environment.remove(&format!("hits({},{})", ball_id, square_id));
                self.current_script_owner = None;  // Clear script context
            }
        }
    }

fn execute_script_block(&mut self, script_content: &str, cursor_x: u32, cursor_y: u32) -> Result<(), InterpreterError> {
    println!("Debug: Executing script content: {}", script_content);
    
    // Parse the entire script as proper AST statements instead of extracting string commands
    let mut lexer = Lexer::new(script_content);
    let tokens = lexer.tokenize().map_err(|e| {
        eprintln!("Script tokenization error: {}", e);
        InterpreterError::LexerError(e)
    })?;
    
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| {
        eprintln!("Script parsing error: {}", e);
        InterpreterError::ParseError(e)
    })?;
    
    // Execute each statement in the script
    for statement in program.statements {
        println!("Debug: Executing statement: {:?}", statement);
        if let Err(e) = self.execute_statement(&statement) {
            eprintln!("Error executing script statement: {}", e);
            // Continue executing other statements even if one fails
        } else {
            println!("Debug: Statement executed successfully");
        }
    }
    
    Ok(())
}

fn execute_verbose(&mut self) -> Result<Value, InterpreterError> {
        self.verbose_mode = !self.verbose_mode;
        let status = if self.verbose_mode { "enabled" } else { "disabled" };
        Ok(Value::String(format!("Verbose mode {}", status)))
    }

pub fn is_verbose_mode(&self) -> bool {
        self.verbose_mode
    }

    pub fn needs_graphics_update(&mut self) -> bool {
        let needs_update = self.graphics_update_needed;
        self.graphics_update_needed = false;  // Reset the flag
        needs_update
    }

    fn execute_run_command(&mut self, script_name: &str) -> Result<Value, InterpreterError> {
        // Add .cant extension if not present
        let filename = if script_name.ends_with(".cant") {
            script_name.to_string()
        } else {
            format!("{}.cant", script_name)
        };
        
        // Check if file exists
        if !std::path::Path::new(&filename).exists() {
            return Err(InterpreterError::RuntimeError(format!("Script file '{}' not found", filename)));
        }
        
        // Read and execute the script file
        match std::fs::read_to_string(&filename) {
            Ok(script_content) => {
                println!("Debug: Running script file: {}", filename);
                self.execute_script_block(&script_content, self.cursor_x, self.cursor_y)?;
                Ok(Value::String(format!("Executed script: {}", filename)))
            },
            Err(e) => Err(InterpreterError::RuntimeError(format!("Error reading script file '{}': {}", filename, e)))
        }
    }

fn execute_label(&mut self, object_name: &str, arguments: &[Expr], text: &str) -> Result<Value, InterpreterError> {
    let object_id = if object_name == "cursor" {
        // Find object at cursor position using find_object_at with tolerance
        self.game_objects.find_object_at(self.cursor_x as f64, self.cursor_y as f64, 0.5)
    } else if object_name == "square" {
        // Handle square(x, y) or square(id) syntax
        if arguments.len() == 2 {
            // square(x, y) - find square at position
            let x = self.evaluate_expression(&arguments[0])?.as_number()
                .ok_or_else(|| InterpreterError::TypeError("Expected number for x coordinate".to_string()))?;
            let y = self.evaluate_expression(&arguments[1])?.as_number()
                .ok_or_else(|| InterpreterError::TypeError("Expected number for y coordinate".to_string()))?;
            
            // Find object at position and check if it's a square
            if let Some(id) = self.game_objects.find_object_at(x, y, 0.5) {
                if let Some(GameObject::Square(_)) = self.game_objects.get_object(id) {
                    Some(id)
                } else {
                    None
                }
            } else {
                None
            }
        } else if arguments.len() == 1 {
            // square(id) - find square by sequence number, but we need to convert to friendly name
            let sequence_id = self.evaluate_expression(&arguments[0])?.as_number()
                .ok_or_else(|| InterpreterError::TypeError("Expected number for square ID".to_string()))?;
            
            // Convert sequence number to friendly name and find by name
            let friendly_name = format!("square{}", sequence_id as u32);
            self.game_objects.find_object_by_name(&friendly_name)
        } else {
            return Err(InterpreterError::RuntimeError(
                "Label square requires 1 or 2 arguments".to_string()
            ));
        }
    } else {
        // Handle direct object names like "square1", "ball2", etc.
        self.game_objects.find_object_by_name(object_name)
    };
    
    if let Some(id) = object_id {
        if let Some(square) = self.game_objects.get_square_mut(id) {
            square.set_label(text.to_string());
            Ok(Value::String(format!("Labeled square with: {}", text)))
        } else {
            Err(InterpreterError::RuntimeError(
                "Object is not a square".to_string()
            ))
        }
    } else {
        Err(InterpreterError::RuntimeError(
            "No square found with that name".to_string()
        ))
    }
}

}