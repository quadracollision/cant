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
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            grid_state: None,
            globals: HashMap::new(),
            environment: HashMap::new(),
            game_objects: GameObjectManager::new(),
            game_state_manager: GameStateManager::new(),
            physics_engine: PhysicsEngine::new(1.0, 1.0, 1.0), // Minimal placeholder values
        };
        interpreter.register_builtins();
        interpreter
    }

    fn execute_play(&mut self) -> Result<Value, InterpreterError> {
        if !self.game_state_manager.is_playing() {
            // First time playing - save current state
            if self.game_state_manager.saved_state.is_none() {
                self.game_state_manager.save_state(
                    &self.game_objects,
                    &self.grid_state,
                    &self.environment
                );
            } else {
                // Restore to saved state
                if let Some(saved) = self.game_state_manager.restore_state() {
                    self.game_objects = saved.game_objects.clone();
                    self.grid_state = saved.grid_state.clone();
                    self.environment = saved.environment.clone();
                }
            }
        }
        
        self.game_state_manager.start_play();
        Ok(Value::String("Game started".to_string()))
    }

    fn execute_pause(&mut self) -> Result<Value, InterpreterError> {
        self.game_state_manager.pause_play();
        Ok(Value::String("Game paused".to_string()))
    }

    fn execute_stop(&mut self) -> Result<Value, InterpreterError> {
        // Stop the physics simulation
        self.game_state_manager.stop_play();
        
        // Restore the saved state if it exists
        if let Some(saved) = self.game_state_manager.restore_state() {
            self.game_objects = saved.game_objects;
            self.grid_state = saved.grid_state;
            self.environment = saved.environment;
            Ok(Value::String("Game stopped and state restored".to_string()))
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

    pub fn execute_command(&mut self, input: &str) -> Result<String, InterpreterError> {
        if input.trim().is_empty() {
            return Ok(String::new());
        }

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
            Stmt::Play => {
                self.execute_play()
            },
            Stmt::Pause => {
                self.execute_pause()
            },
            Stmt::Stop => {
                self.execute_stop()
            },
            Stmt::ClearBalls => {
                self.execute_clear_balls()
            },
            Stmt::ClearSquares => {
                self.execute_clear_squares()
            },
        }
    }

    fn evaluate_expression(&mut self, expr: &Expr) -> Result<Value, InterpreterError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Identifier(name) => {
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
                        // Require a grid to exist before creating a ball
                        if let Some(ref grid) = self.grid_state {
                            let (start_x, start_y) = if arguments.len() >= 2 {
                                // Use provided x,y coordinates as grid cell indices
                                let x = self.evaluate_expression(&arguments[0])?.as_number()
                                    .ok_or_else(|| InterpreterError::TypeError("Ball x coordinate must be a number".to_string()))?;
                                let y = self.evaluate_expression(&arguments[1])?.as_number()
                                    .ok_or_else(|| InterpreterError::TypeError("Ball y coordinate must be a number".to_string()))?;
                                // Add 0.5 to center the ball in the grid cell
                                (x + 0.5, y + 0.5)
                            } else {
                                // Default to center of grid
                                ((grid.width as f64 / 2.0), (grid.height as f64 / 2.0))
                            };
                            let id = self.game_objects.create_ball(start_x, start_y, 5.0, 0.0);
                            return Ok(Value::GameObject(id));
                        } else {
                            return Err(InterpreterError::RuntimeError(
                                "Cannot create ball: No grid exists. Create a grid first with grid(x, y)".to_string()
                            ));
                        }
                    },
                    "square" => {
                        let x = if arguments.len() > 0 {
                            self.evaluate_expression(&arguments[0])?.as_number()
                                .ok_or_else(|| InterpreterError::TypeError("x must be a number".to_string()))?
                        } else { 0.0 };
                        
                        let y = if arguments.len() > 1 {
                            self.evaluate_expression(&arguments[1])?.as_number()
                                .ok_or_else(|| InterpreterError::TypeError("y must be a number".to_string()))?
                        } else { 0.0 };
                        
                        let id = self.game_objects.create_square(x, y);
                        Ok(Value::GameObject(id))
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
                return Ok(Value::GameObject(id));
            },
            "destroy" => {
                // Handle destroy function inline
                if arguments.len() != 1 {
                    return Err(InterpreterError::RuntimeError("destroy expects 1 argument".to_string()));
                }
                let obj_id = self.evaluate_expression(&arguments[0])?;
                if let Value::GameObject(id) = obj_id {
                    self.game_objects.destroy_object(id);
                    return Ok(Value::String("Object destroyed".to_string()));
                } else {
                    return Err(InterpreterError::TypeError("destroy expects a game object".to_string()));
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

    fn show_help(&self) -> String {
        "CANT Language Commands:\n\
         grid(width, height) - Create a new grid\n\
         tilesize(pixels) - Set tile size (4-100 pixels)\n\
         clear - Clear the current grid\n\
         help - Show this help message\n\
         \n\
         Navigation:\n\
         Arrow keys - Move cursor\n\
         Space - Toggle cell\n\
         Enter - Execute command\n\
         Escape - Clear command line".to_string()
    }

    pub fn get_game_objects(&self) -> &GameObjectManager {
        &self.game_objects
    }

    fn execute_set_direction(&mut self, object_name: &str, direction: &DirectionValue) -> Result<Value, InterpreterError> {
        // Find the object by name
        let object_id = self.game_objects.find_object_by_name(object_name)
            .ok_or_else(|| InterpreterError::RuntimeError(format!("Object '{}' not found", object_name)))?;
        
        // Convert direction to angle
        let angle = match direction {
            DirectionValue::Left => std::f64::consts::PI,
            DirectionValue::Right => 0.0,
            DirectionValue::Up => 3.0 * std::f64::consts::PI / 2.0,
            DirectionValue::Down => std::f64::consts::PI / 2.0,
            DirectionValue::UpLeft => 5.0 * std::f64::consts::PI / 4.0,
            DirectionValue::UpRight => 7.0 * std::f64::consts::PI / 4.0,
            DirectionValue::DownLeft => 3.0 * std::f64::consts::PI / 4.0,
            DirectionValue::DownRight => std::f64::consts::PI / 4.0,
        };
        
        self.game_objects.set_ball_direction(object_id, angle)
            .map_err(|e| InterpreterError::RuntimeError(e))?;
        
        Ok(Value::String(format!("Set direction of {} to {:?}", object_name, direction)))
    }

    fn execute_clear_balls(&mut self) -> Result<Value, InterpreterError> {
        let count = self.game_objects.clear_all_balls();
        Ok(Value::String(format!("Cleared {} ball(s)", count)))
    }

    fn execute_clear_squares(&mut self) -> Result<Value, InterpreterError> {
        let count = self.game_objects.clear_all_squares();
        Ok(Value::String(format!("Cleared {} square(s)", count)))
    }
}