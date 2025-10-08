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
mod waveform_editor;
mod input_mapping; // Add this line

use winit::{
    event::{Event, WindowEvent, KeyboardInput, MouseButton, ElementState},
    event_loop::{EventLoop, ControlFlow},
    window::WindowBuilder,
    dpi::PhysicalPosition,
};
use std::time::Instant;

use crate::interpreter::Interpreter;
use crate::graphics::GraphicsRenderer;
use crate::input::{InputHandler, InputAction};
use crate::console::Console;
use crate::input_mapping::InputMapper; // Add this line

const WIDTH: u32 = 500;
const HEIGHT: u32 = 500;

// Helper function to copy audio files to the samples directory
fn copy_audio_file_to_samples(source_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::path::Path;
    use std::fs;
    
    let source = Path::new(source_path);
    
    // Get the filename from the source path
    let filename = source.file_name()
        .ok_or("Invalid file path")?
        .to_str()
        .ok_or("Invalid filename")?;
    
    // Create the destination path in the samples directory
    let dest_path = format!("samples/{}", filename);
    let dest = Path::new(&dest_path);
    
    // Copy the file
    fs::copy(source, dest)?;
    
    Ok(dest_path)
}

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
    
    // Waveform editor state - track if we're in waveform mode and store audio data
    let mut waveform_editor: Option<crate::waveform_editor::WaveformEditor> = None;
    let mut waveform_mode = false;
    let mut waveform_audio_samples: Vec<f32> = Vec::new();
    let mut waveform_filename: Option<String> = None;
    
    // No initial grid setup - wait for user to call grid(x, y)
    
    let mut input_handler = InputHandler::new();
    let mut console = Console::new(50);
    
    let mut last_update = Instant::now();
    let mut redraw_requested = false;
    let mut input_mapper = InputMapper::new();
    let mut mouse_position: PhysicalPosition<f64> = PhysicalPosition::new(0.0, 0.0);
    
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
                    WindowEvent::CursorMoved { position, .. } => {
                        mouse_position = position;
                    }
                    WindowEvent::MouseInput { 
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        console.add_output(&format!("Click coordinates: ({:.0}, {:.0})", mouse_position.x, mouse_position.y));
                        redraw_requested = true;
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        // Check if waveform editor is active first
                        if waveform_mode {
                            // Handle integrated waveform mode input
                            if let Some(key_code) = input.virtual_keycode {
                                if input.state == winit::event::ElementState::Pressed {
                                    match key_code {
                                        winit::event::VirtualKeyCode::Escape => {
                                            waveform_mode = false;
                                            waveform_editor = None;
                                            console.add_output("Waveform editor closed");
                                            redraw_requested = true;
                                        }
                                        winit::event::VirtualKeyCode::Space => {
                                            // Add slice marker at cursor position
                                            if let Some(ref mut editor) = waveform_editor {
                                                // Get cursor position from graphics module and sync it with waveform editor
                                                let (cursor_pos, _, _) = graphics.get_waveform_state();
                                                editor.set_cursor_position(cursor_pos);
                                                editor.add_slice_marker();
                                                let message = format!("Slice marker added at position: {}", cursor_pos);
                                                console.add_output(&message);
                                                redraw_requested = true;
                                            } else {
                                                console.add_output("No waveform editor available");
                                                redraw_requested = true;
                                            }
                                        }
                                        _ => {
                                            // Delegate waveform input handling to graphics module
                                            if let Some(message) = graphics.handle_waveform_input(key_code, &waveform_audio_samples) {
                                                console.add_output(&message);
                                                redraw_requested = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Check if file selection mode is active
                        else if interpreter.is_file_selection_mode() {
                            if let Some(key_code) = input.virtual_keycode {
                                if input.state == winit::event::ElementState::Pressed {
                                    if let Some(message) = interpreter.handle_file_selection_input(key_code) {
                                        console.add_output(&message);
                                        redraw_requested = true;
                                    }
                                }
                            }
                        }
                        // Check if script editor is active
                        else if interpreter.is_script_editor_active() {
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
                
                // Check if waveform mode is requested
                if interpreter.is_waveform_mode_requested() && !waveform_mode {
                    let file_path = interpreter.get_waveform_file_path();
                    console.add_output(&format!("Activating waveform editor for: {:?}", file_path));
                    
                    // Store the filename for display
                    waveform_filename = file_path.clone();
                    
                    // Load audio samples if file path is provided
                    if let Some(path) = &file_path {
                        // Copy file to samples directory and get local path
                        match copy_audio_file_to_samples(path) {
                            Ok(local_path) => {
                                console.add_output(&format!("Copied audio file to: {}", local_path));
                                match crate::waveform_editor::WaveformEditor::load_samples_from_file(&local_path) {
                                    Ok(samples) => {
                                        waveform_audio_samples = samples;
                                        console.add_output(&format!("Loaded {} audio samples", waveform_audio_samples.len()));
                                    }
                                    Err(e) => {
                                        console.add_output(&format!("Failed to load audio file: {}", e));
                                        waveform_audio_samples.clear();
                                    }
                                }
                            }
                            Err(e) => {
                                console.add_output(&format!("Failed to copy audio file: {}", e));
                                // Try loading from original path as fallback
                                match crate::waveform_editor::WaveformEditor::load_samples_from_file(path) {
                                    Ok(samples) => {
                                        waveform_audio_samples = samples;
                                        console.add_output(&format!("Loaded {} audio samples from original path", waveform_audio_samples.len()));
                                    }
                                    Err(e) => {
                                        console.add_output(&format!("Failed to load audio file: {}", e));
                                        waveform_audio_samples.clear();
                                    }
                                }
                            }
                        }
                    }
                    
                    waveform_mode = true;
                    
                    // Initialize waveform editor with loaded samples
                    if !waveform_audio_samples.is_empty() {
                        waveform_editor = Some(crate::waveform_editor::WaveformEditor::new_integrated());
                        if let Some(ref mut editor) = waveform_editor {
                            editor.load_audio(waveform_audio_samples.clone());
                        }
                        console.add_output("Waveform mode activated (integrated mode) with editor");
                    } else {
                        console.add_output("Waveform mode activated (integrated mode) - no audio loaded");
                    }
                    
                    interpreter.clear_waveform_request();
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
                    // Check if waveform editor is active
                    if waveform_mode {
                        let display_lines = console.get_display_lines(6);
                        graphics.render_waveform_mode(&display_lines, &waveform_audio_samples);
                        
                        // Render filename in top left if available
                        if let Some(ref filename) = waveform_filename {
                            graphics.render_waveform_filename(filename);
                        }
                        
                        // Render slice markers if waveform editor exists
                        if let Some(ref editor) = waveform_editor {
                            let slice_markers = editor.get_slice_markers();
                            let (_, zoom_level, scroll_position) = graphics.get_waveform_state();
                            graphics.render_slice_markers(slice_markers, zoom_level, scroll_position, &waveform_audio_samples);
                        }
                    }
                    // Check if file selection mode is active
                    else if interpreter.is_file_selection_mode() {
                        let display_lines = interpreter.get_file_selection_display_lines();
                        graphics.render(interpreter.get_grid_state(), &display_lines, Some(interpreter.get_game_objects()));
                    }
                    // Check if script editor is active
                    else if interpreter.is_script_editor_active() {
                        let display_lines = interpreter.get_script_editor_display_lines();
                        graphics.render(interpreter.get_grid_state(), &display_lines, Some(interpreter.get_game_objects()));
                    } else {
                        let display_lines = console.get_display_lines(6);  // Reduced from 10 to 6 to account for input line
                        graphics.render(interpreter.get_grid_state(), &display_lines, Some(interpreter.get_game_objects()));
                    }
                    
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