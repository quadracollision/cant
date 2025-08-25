mod ast;
mod lexer;
mod parser;
mod interpreter;
mod grid;
mod graphics;
mod input;
mod console;
mod font;
mod game_objects;
mod ball;
mod square;
mod physics_engine;
mod game_state;
mod audio_engine;
mod script_editor;
mod input_mapping; // Add this line

use winit::{
    event::{Event, WindowEvent, KeyboardInput},
    event_loop::{EventLoop, ControlFlow},
    window::WindowBuilder,
};
use std::time::Instant;

use crate::interpreter::Interpreter;
use crate::graphics::GraphicsRenderer;
use crate::input::{InputHandler, InputAction};
use crate::console::Console;
use crate::input_mapping::InputMapper; // Add this line

const WIDTH: u32 = 500;
const HEIGHT: u32 = 500;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Quadracollision Canticle")
        .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT))
        .with_resizable(true)
        .build(&event_loop)?;

    let mut graphics = GraphicsRenderer::new(&window, WIDTH, HEIGHT)?;
    let mut interpreter = Interpreter::new();
    
    // No initial grid setup - wait for user to call grid(x, y)
    
    let mut input_handler = InputHandler::new();
    let mut console = Console::new(50);
    
    let mut last_update = Instant::now();
    let mut redraw_requested = false;
    let mut input_mapper = InputMapper::new(); // Add this line
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        graphics.resize(size.width, size.height);
                        redraw_requested = true;
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        // Check if script editor is active first
                        if interpreter.is_script_editor_active() {
                            // Handle script editor input
                            let key_str = input_mapper.map_script_editor_key(&input);
                            
                            if !key_str.is_empty() {
                                interpreter.handle_script_editor_key(&key_str);
                                redraw_requested = true;
                            }
                        } else {
                            // Handle normal console input
                            let script_editor_active = interpreter.is_script_editor_active();
                            let action = input_handler.handle_keyboard_input(&input, script_editor_active);
                            
                            // Process the input action
                            match action {
                                InputAction::ExecuteCommand(command) => {
                                    // Add command to history and clear buffers
                                    console.execute_command(command.clone());
                                    input_handler.set_command_buffer(String::new());
                                    
                                    // Get current cursor position from grid state
                                    let (cursor_x, cursor_y) = if let Some(grid_state) = interpreter.get_grid_state() {
                                        (grid_state.cursor_x, grid_state.cursor_y)
                                    } else {
                                        (0, 0)
                                    };
                                    
                                    match interpreter.execute_command(&command, cursor_x, cursor_y) {
                                        Ok(result) => {
                                            if !result.is_empty() {
                                                console.add_output(&result);
                                            }
                                            // Update graphics renderer with new grid dimensions if grid was created
                                            if let Some(grid_state) = interpreter.get_grid_state() {
                                                graphics.set_grid_size(grid_state.width, grid_state.height);
                                                // Sync graphics renderer cursor with grid state cursor
                                                let (grid_cursor_x, grid_cursor_y) = (grid_state.cursor_x, grid_state.cursor_y);
                                                graphics.move_cursor(grid_cursor_x as i32 - graphics.get_cursor_position().0 as i32, 
                                                                   grid_cursor_y as i32 - graphics.get_cursor_position().1 as i32);
                                            }
                                            
                                            // Sync font size from interpreter to graphics renderer
                                            if let Some(font_size) = interpreter.get_environment_value("__font_size") {
                                                if let Ok(size) = font_size.parse::<f32>() {
                                                    graphics.set_font_size(size);
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            console.add_error(&format!("{}", err));
                                        }
                                    }
                                    redraw_requested = true;
                                }
                                InputAction::UpdateCommandBuffer(buffer) => {
                                    console.set_current_command(buffer);
                                    redraw_requested = true;
                                }
                                InputAction::UpdateCommandBufferAndResetHistory(buffer) => {
                                    console.set_current_command(buffer);
                                    console.reset_history_navigation(); // Add this line!
                                    redraw_requested = true;
                                }
                                InputAction::MoveCursor(dx, dy) => {
                                    // Move cursor in both grid state and graphics renderer
                                    if let Some(grid_state) = interpreter.get_grid_state_mut() {
                                        grid_state.move_cursor(dx, dy);
                                        
                                        // Get cursor position after movement
                                        let cursor_x = grid_state.cursor_x;
                                        let cursor_y = grid_state.cursor_y;
                                        
                                        // Display cursor position
                                        console.add_output(&format!("Cursor: ({}, {})", cursor_x, cursor_y));
                                        
                                        // Check for objects at cursor position and display them
                                        let objects_at_cursor = interpreter.get_game_objects().find_objects_at_grid_with_names(cursor_x, cursor_y);
                                        if !objects_at_cursor.is_empty() {
                                            console.add_output(&format!("Objects at ({}, {}): {}", cursor_x, cursor_y, objects_at_cursor.join(", ")));
                                        }
                                    }
                                    graphics.move_cursor(dx, dy);
                                    redraw_requested = true;
                                }
                                InputAction::HistoryPrevious => {
                                    console.history_previous();
                                    input_handler.set_command_buffer(console.get_current_command().to_string());
                                    redraw_requested = true;
                                }
                                InputAction::HistoryNext => {
                                    console.history_next();
                                    input_handler.set_command_buffer(console.get_current_command().to_string());
                                    redraw_requested = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::MainEventsCleared => {
                // Calculate delta time and update physics
                let now = Instant::now();
                let dt = now.duration_since(last_update).as_secs_f64();
                last_update = now;
                
                // Update physics if game is playing
                interpreter.update_physics(dt);
                
                // Update script editor cursor blink if active
                if interpreter.is_script_editor_active() {
                    interpreter.update_script_editor_cursor();
                    redraw_requested = true;
                }
                
                // Check if graphics need updating after script execution
                if interpreter.needs_graphics_update() {
                    if let Some(grid_state) = interpreter.get_grid_state() {
                        graphics.set_grid_size(grid_state.width, grid_state.height);
                    }
                    redraw_requested = true;
                }
                
                // Always request redraw when playing to show ball movement
                if interpreter.is_playing() {
                    redraw_requested = true;
                }
                
                if redraw_requested {
                    // Check if script editor is active
                    let display_lines = if interpreter.is_script_editor_active() {
                        interpreter.get_script_editor_display_lines()
                    } else {
                        console.get_display_lines(6)  // Reduced from 10 to 6 to account for input line
                    };
                    
                    graphics.render(interpreter.get_grid_state(), &display_lines, Some(interpreter.get_game_objects()));
                    
                    if let Err(err) = graphics.present() {
                        log::error!("Render error: {}", err);
                        *control_flow = ControlFlow::Exit;
                    }
                    redraw_requested = false;
                }
            }
            _ => {}
        }
    });
}