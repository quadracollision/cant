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
mod audio_engine; // Add this line

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

const WIDTH: u32 = 400;
const HEIGHT: u32 = 400;

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
                        let action = input_handler.handle_keyboard_input(&input);
                        // In the main event loop where InputAction is handled
                        match action {
                            InputAction::MoveCursor(dx, dy) => {
                                graphics.move_cursor(dx, dy);
                                
                                // Output cursor info to console
                                let (cursor_x, cursor_y) = graphics.get_cursor_position();
                                let object_names = interpreter.get_game_objects().find_objects_at_grid_with_names(cursor_x, cursor_y);
                                
                                let coord_text = format!("Cursor: ({}, {})", cursor_x, cursor_y);
                                let info_text = if object_names.is_empty() {
                                    coord_text
                                } else {
                                    format!("{} - Objects: {}", coord_text, object_names.join(", "))
                                };
                                
                                console.add_line(info_text);
                                redraw_requested = true;
                            }
                            InputAction::ToggleCell => {
                                let (x, y) = graphics.get_cursor_position();
                                if let Some(grid_state) = interpreter.get_grid_state_mut() {
                                    grid_state.toggle_cell_at(x, y);
                                }
                                redraw_requested = true;
                            }
                            InputAction::HistoryPrevious => {
                                console.history_previous();
                                input_handler.set_command_buffer(console.get_current_command().to_string());
                                redraw_requested = true; // Add this line
                            }
                            InputAction::HistoryNext => {
                                console.history_next();
                                input_handler.set_command_buffer(console.get_current_command().to_string());
                                redraw_requested = true; // Add this line
                            }
                            InputAction::UpdateCommandBuffer(buffer) => {
                                console.set_current_command(buffer);
                                redraw_requested = true;
                            }
                            InputAction::UpdateCommandBufferAndResetHistory(buffer) => {
                                console.reset_history_navigation();
                                console.set_current_command(buffer);
                                redraw_requested = true;
                            }
                            InputAction::ExecuteCommand(command) => {
                                // Add the command to console output with prompt (like a real terminal)
                                console.add_line(format!("> {}", command));
                                
                                let (cursor_x, cursor_y) = graphics.get_cursor_position();
                                match interpreter.execute_command(&command, cursor_x, cursor_y) {
                                    Ok(result) => {
                                        if !result.is_empty() {
                                            console.add_line(result);
                                        }
                                        
                                        // Check if grid was created/modified and update graphics
                                        if let Some(grid_state) = interpreter.get_grid_state() {
                                            graphics.set_grid_size(grid_state.width, grid_state.height);
                                        }
                                    }
                                    Err(e) => {
                                        console.add_line(format!("Error: {}", e));
                                    }
                                }
                                
                                // Clear both the console and input handler command buffers
                                console.execute_command(command);
                                input_handler.set_command_buffer(String::new()); // Add this line
                                redraw_requested = true;
                            }
                            InputAction::None => {}
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
                
                // Always request redraw when playing to show ball movement
                if interpreter.is_playing() {
                    redraw_requested = true;
                }
                
                if redraw_requested {
                    let console_lines = console.get_display_lines(10);
                    graphics.render(interpreter.get_grid_state(), &console_lines, Some(interpreter.get_game_objects()));
                    
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