use std::collections::HashMap;
use thiserror::Error;
use crate::grid::GridState;
use crate::lexer::{Lexer, LexerError};
use crate::parser::{Parser, ParseError};
use crate::ast::*;
use crate::game_objects::{GameObjectManager, GameObject};
use crate::physics_engine::PhysicsEngine;
use crate::game_state::GameStateManager;

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
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            grid_state: None,
            globals: HashMap::new(),
            environment: HashMap::new(),
            game_objects: GameObjectManager::new(),
            game_state_manager: GameStateManager::new(),
            physics_engine: PhysicsEngine::new(1.0, 1.0, 1.0),
            cursor_x: 0,
            cursor_y: 0,
        };
        interpreter.register_builtins();
        interpreter
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
            
            for ball_id in self.game_objects.get_all_ball_ids() {
                if let Some(ball) = self.game_objects.get_ball_mut(ball_id) {
                    self.physics_engine.update_ball(ball, dt, &squares);
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
            Stmt::Label { object_name, arguments, text } => {
                self.execute_label(object_name, arguments, text)
            },
            Stmt::Play => self.execute_play(),
            Stmt::Pause => self.execute_pause(),
            Stmt::Stop => self.execute_stop(),
            Stmt::ClearBalls => self.execute_clear_balls(),
            Stmt::ClearSquares => self.execute_clear_squares(),
            Stmt::Destroy { object_type, arguments } => {  // Add this
                self.execute_destroy(object_type, arguments)
            },
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
    
    pub fn get_environment_value(&self, key: &str) -> Option<&Value> {
        self.environment.get(key)
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
            "sample" => return self.call_sample_function(arguments),
            "clear" => {
                self.grid_state = None;
                return Ok(Value::String("Grid cleared".to_string()));
            },
            "help" => return Ok(Value::String(self.show_help())),
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
        if arguments.len() != 2 {
            return Err(InterpreterError::RuntimeError(
                "grid() requires exactly 2 arguments (x, y)".to_string()
            ));
        }

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
        // ADD THIS LINE: Update physics engine boundaries to match the new grid
        self.physics_engine.update_grid_size(x as f64, y as f64);
        Ok(Value::String(format!("Created {}x{} grid", x, y)))
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
        println!("Debug: Setting color on ball {}", object_id);
        ball.set_color(color_string.clone());
    } else if let Some(square) = self.game_objects.get_square_mut(object_id) {
        println!("Debug: Setting color on square {}", object_id);
        square.set_color(color_string.clone());
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