use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState, MouseButton, ModifiersState},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use std::collections::HashMap;
use crate::audio_engine::{AudioEngine, with_audio_engine};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 600;
const WAVEFORM_HEIGHT: u32 = 400;
const MARKER_HEIGHT: u32 = 200;

pub struct WaveformEditor {
    pixels: Option<Pixels>,
    window: Option<Window>,
    audio_samples: Vec<f32>,
    markers: Vec<f32>,
    slice_markers: Vec<f32>,  // New field for slice markers
    zoom_level: f32,
    scroll_position: f32,
    cursor_position: f32,
    mouse_x: f32,
    mouse_y: f32,
    is_dragging: bool,
    selected_marker: Option<usize>,
    loaded_sample_key: Option<String>,  // Track the loaded sample key for audio playback
    sample_rate: f32,  // Store sample rate for time calculations
}

impl WaveformEditor {
    pub fn new_with_window(window: Window, file_path: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture)?;

        let mut editor = WaveformEditor {
            pixels: Some(pixels),
            window: Some(window),
            audio_samples: Vec::new(),
            markers: Vec::new(),
            slice_markers: Vec::new(),  // Initialize slice markers
            zoom_level: 1.0,
            scroll_position: 0.0,
            cursor_position: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            is_dragging: false,
            selected_marker: None,
            loaded_sample_key: None,
            sample_rate: 44100.0,  // Default sample rate
        };

        // Load audio file if provided
        if let Some(path) = file_path {
            println!("Loading audio file: {}", path);
            if let Err(e) = editor.load_audio_from_file(&path) {
                eprintln!("Failed to load audio file: {}", e);
            }
        }

        Ok(editor)
    }

    pub fn new(event_loop: &EventLoop<()>, file_path: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let window = WindowBuilder::new()
            .with_title("Waveform Editor")
            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
            .with_resizable(true)
            .build(event_loop)?;

        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture)?;

        let mut editor = WaveformEditor {
            pixels: Some(pixels),
            window: Some(window),
            audio_samples: Vec::new(),
            markers: Vec::new(),
            slice_markers: Vec::new(),
            zoom_level: 1.0,
            scroll_position: 0.0,
            cursor_position: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            is_dragging: false,
            selected_marker: None,
            loaded_sample_key: None,
            sample_rate: 44100.0,  // Default sample rate
        };

        // Load audio file if provided
        if let Some(path) = file_path {
            println!("Loading audio file: {}", path);
            if let Err(e) = editor.load_audio_from_file(&path) {
                eprintln!("Failed to load audio file: {}", e);
            }
        }

        Ok(editor)
    }

    // New constructor for integrated mode (no separate window)
    pub fn new_integrated() -> Self {
        WaveformEditor {
            pixels: None,
            window: None,
            audio_samples: Vec::new(),
            markers: Vec::new(),
            slice_markers: Vec::new(),
            zoom_level: 1.0,
            scroll_position: 0.0,
            cursor_position: 0.0,
            mouse_x: 0.0,
            mouse_y: 0.0,
            is_dragging: false,
            selected_marker: None,
            loaded_sample_key: None,
            sample_rate: 44100.0,  // Default sample rate
        }
    }

    pub fn load_audio(&mut self, samples: Vec<f32>) {
        self.audio_samples = samples;
        self.markers.clear();
        // Add initial markers at start and end
        if !self.audio_samples.is_empty() {
            self.markers.push(0.0);
            self.markers.push(self.audio_samples.len() as f32);
        }
    }

    pub fn load_audio_from_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("Loading audio file: {}", file_path);
        
        // Load the audio file into the audio engine
        let sample_key = with_audio_engine(|engine| {
            engine.load_audio_file(file_path)
        })?;
        
        // Store the sample key for playback
        self.loaded_sample_key = Some(sample_key);
        
        // Load samples for waveform display and get the actual sample rate
        let (samples, sample_rate) = Self::load_samples_from_file(file_path)?;
        self.sample_rate = sample_rate;
        self.load_audio(samples);
        
        println!("Audio file loaded successfully: {} samples at {} Hz", self.audio_samples.len(), self.sample_rate);
        Ok(())
    }

    // Static function to load audio samples without needing a WaveformEditor instance
    pub fn load_samples_from_file(file_path: &str) -> Result<(Vec<f32>, f32), Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io::BufReader;
        use std::path::Path;
        use rodio::{Decoder, Source};

        // First, try to find the file in the samples directory
        let path = Path::new(file_path);
        let actual_path = if let Some(filename) = path.file_name() {
            let samples_file = format!("samples/{}", filename.to_string_lossy());
            if std::fs::metadata(&samples_file).is_ok() {
                samples_file
            } else {
                file_path.to_string()
            }
        } else {
            file_path.to_string()
        };

        // Open and decode the audio file
        let file = File::open(&actual_path)?;
        let buf_reader = BufReader::new(file);
        let decoder = Decoder::new(buf_reader)?;
        
        // Get the sample rate before consuming the decoder
        let sample_rate = decoder.sample_rate() as f32;
        
        // Convert to f32 samples
        let samples: Vec<f32> = decoder
            .convert_samples::<f32>()
            .collect();
        
        Ok((samples, sample_rate))
    }

    pub fn run(mut self, event_loop: EventLoop<()>) -> Result<std::collections::HashMap<String, Vec<usize>>, Box<dyn std::error::Error>> {
        let mut slice_arrays = std::collections::HashMap::new();
        let mut modifiers = ModifiersState::empty();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        // Create a default slice array from markers
                        if self.markers.len() >= 2 {
                            let mut indices = Vec::new();
                            for i in 0..(self.markers.len() - 1) {
                                indices.push(i);
                            }
                            slice_arrays.insert("default_slice_array".to_string(), indices);
                        }
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        if let Some(ref mut pixels) = self.pixels {
                            if let Err(err) = pixels.resize_surface(size.width, size.height) {
                                eprintln!("Failed to resize surface: {}", err);
                                *control_flow = ControlFlow::Exit;
                            }
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        self.mouse_x = position.x as f32;
                        self.mouse_y = position.y as f32;
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if button == MouseButton::Left {
                            match state {
                                ElementState::Pressed => {
                                    self.handle_mouse_press();
                                }
                                ElementState::Released => {
                                    self.is_dragging = false;
                                    self.selected_marker = None;
                                }
                            }
                        } else if button == MouseButton::Right && state == ElementState::Pressed {
                            self.add_marker_at_cursor();
                        }
                    }
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        modifiers = new_modifiers;
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if input.state == ElementState::Pressed {
                            if let Some(keycode) = input.virtual_keycode {
                                match keycode {
                                    VirtualKeyCode::P => {
                                        self.preview_current_slice();
                                    }
                                    VirtualKeyCode::Space => {
                                        if modifiers.shift() {
                                            self.reset_view();
                                        } else {
                                            self.preview_slice_at_cursor();
                                        }
                                    }
                                    VirtualKeyCode::Delete => {
                                        self.delete_selected_marker();
                                    }
                                    VirtualKeyCode::Left => {
                                        self.move_cursor_left();
                                    }
                                    VirtualKeyCode::Right => {
                                        self.move_cursor_right();
                                    }
                                    VirtualKeyCode::Return => {
                                        self.add_marker_at_cursor_position();
                                    }
                                    VirtualKeyCode::Equals => {
                                        self.zoom_in();
                                    }
                                    VirtualKeyCode::Minus => {
                                        self.zoom_out();
                                    }
                                    VirtualKeyCode::Escape => {
                                        *control_flow = ControlFlow::Exit;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
                Event::MainEventsCleared => {
                    if self.is_dragging {
                        self.update_marker_position();
                    }
                    self.render();
                    if let Some(ref window) = self.window {
                        window.request_redraw();
                    }
                }
                Event::RedrawRequested(_) => {
                    if let Some(ref mut pixels) = self.pixels {
                        if let Err(err) = pixels.render() {
                            eprintln!("Failed to render: {}", err);
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
                _ => {}
            }
        });

        Ok(slice_arrays)
    }

    fn handle_mouse_press(&mut self) {
        // Check if clicking in the waveform area to position cursor
        if self.mouse_y >= 0.0 && self.mouse_y <= WAVEFORM_HEIGHT as f32 {
            let time_position = self.mouse_position_to_time(self.mouse_x);
            if time_position >= 0.0 && time_position <= self.audio_samples.len() as f32 {
                self.cursor_position = time_position;
                println!("Cursor positioned at: {} via mouse click", self.cursor_position);
                return; // Exit early if we positioned the cursor
            }
        }
        
        // Check if clicking on a marker (existing functionality)
        let marker_y_start = WAVEFORM_HEIGHT;
        let marker_y_end = WAVEFORM_HEIGHT + MARKER_HEIGHT;
        
        if self.mouse_y >= marker_y_start as f32 && self.mouse_y <= marker_y_end as f32 {
            let time_position = self.mouse_position_to_time(self.mouse_x);
            
            // Find closest marker
            let mut closest_marker = None;
            let mut closest_distance = f32::INFINITY;
            
            for (i, &marker_time) in self.markers.iter().enumerate() {
                let marker_x = self.time_to_screen_x(marker_time);
                let distance = (marker_x - self.mouse_x).abs();
                
                if distance < 10.0 && distance < closest_distance {
                    closest_distance = distance;
                    closest_marker = Some(i);
                }
            }
            
            if let Some(marker_index) = closest_marker {
                self.selected_marker = Some(marker_index);
                self.is_dragging = true;
            }
        }
    }

    fn add_marker_at_cursor(&mut self) {
        let time_position = self.mouse_position_to_time(self.mouse_x);
        
        // Ensure marker is within bounds
        if time_position >= 0.0 && time_position <= self.audio_samples.len() as f32 {
            self.markers.push(time_position);
            self.markers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        }
    }

    fn delete_selected_marker(&mut self) {
        if let Some(marker_index) = self.selected_marker {
            // Don't delete first or last marker
            if marker_index > 0 && marker_index < self.markers.len() - 1 {
                self.markers.remove(marker_index);
                self.selected_marker = None;
            }
        }
    }

    fn update_marker_position(&mut self) {
        if let Some(marker_index) = self.selected_marker {
            let new_time = self.mouse_position_to_time(self.mouse_x);
            
            // Ensure marker stays within bounds and doesn't cross other markers
            let min_time = if marker_index > 0 { self.markers[marker_index - 1] } else { 0.0 };
            let max_time = if marker_index < self.markers.len() - 1 { 
                self.markers[marker_index + 1] 
            } else { 
                self.audio_samples.len() as f32 
            };
            
            self.markers[marker_index] = new_time.clamp(min_time, max_time);
        }
    }

    fn mouse_position_to_time(&self, mouse_x: f32) -> f32 {
        let samples_per_pixel = (self.audio_samples.len() as f32) / (WIDTH as f32 * self.zoom_level);
        (mouse_x + self.scroll_position) * samples_per_pixel
    }

    fn time_to_screen_x(&self, time: f32) -> f32 {
        let samples_per_pixel = (self.audio_samples.len() as f32) / (WIDTH as f32 * self.zoom_level);
        (time / samples_per_pixel) - self.scroll_position
    }

    fn preview_current_slice(&self) {
        if let Some(ref sample_key) = self.loaded_sample_key {
            if self.markers.len() >= 2 {
                // Markers are stored as sample indices, convert them to time
                let start_sample_index = self.markers[0] as f64;
                let end_sample_index = self.markers[1] as f64;
                
                // Convert sample indices to time in seconds
                let start_time = start_sample_index / self.sample_rate as f64;
                let end_time = end_sample_index / self.sample_rate as f64;
                
                println!("Previewing current slice: sample indices {} to {} (time: {:.3}s to {:.3}s)", 
                         start_sample_index, end_sample_index, start_time, end_time);
                
                // Play the slice using the audio engine
                if let Err(e) = with_audio_engine(|engine| {
                    engine.play_sample_slice_public(sample_key, start_time, end_time)
                }) {
                    eprintln!("Failed to play slice: {}", e);
                }
            } else {
                println!("Need at least 2 markers to preview a slice");
            }
        } else {
            println!("No audio sample loaded");
        }
    }

    fn preview_slice_at_cursor(&self) {
        if let Some(ref sample_key) = self.loaded_sample_key {
            let cursor_sample_index = self.mouse_position_to_time(self.mouse_x);
            
            // Find the slice that contains the cursor
            for i in 0..self.markers.len().saturating_sub(1) {
                if cursor_sample_index >= self.markers[i] && cursor_sample_index <= self.markers[i + 1] {
                    let start_sample_index = self.markers[i] as f64;
                    let end_sample_index = self.markers[i + 1] as f64;
                    
                    // Convert sample indices to time in seconds
                    let start_time = start_sample_index / self.sample_rate as f64;
                    let end_time = end_sample_index / self.sample_rate as f64;
                    
                    println!("Previewing slice at cursor: sample indices {} to {} (time: {:.3}s to {:.3}s)", 
                             start_sample_index, end_sample_index, start_time, end_time);
                    
                    // Play the slice using the audio engine
                    if let Err(e) = with_audio_engine(|engine| {
                        engine.play_sample_slice_public(sample_key, start_time, end_time)
                    }) {
                        eprintln!("Failed to play slice: {}", e);
                    }
                    return;
                }
            }
            
            // If no slice found, play from cursor to end of sample
            if !self.audio_samples.is_empty() {
                let cursor_sample_index = cursor_sample_index as f64;
                let end_sample_index = self.audio_samples.len() as f64;
                
                let start_time = cursor_sample_index / self.sample_rate as f64;
                let end_time = end_sample_index / self.sample_rate as f64;
                
                println!("Previewing from cursor to end: sample indices {} to {} (time: {:.3}s to {:.3}s)", 
                         cursor_sample_index, end_sample_index, start_time, end_time);
                
                if let Err(e) = with_audio_engine(|engine| {
                    engine.play_sample_slice_public(sample_key, start_time, end_time)
                }) {
                    eprintln!("Failed to play from cursor: {}", e);
                }
            }
        } else {
            println!("No audio sample loaded");
        }
    }

    fn move_cursor_left(&mut self) {
        if !self.audio_samples.is_empty() {
            // Calculate step size based on screen pixels for more precise movement when zoomed in
            let samples_per_pixel = (self.audio_samples.len() as f32) / (WIDTH as f32 * self.zoom_level);
            let step_size = samples_per_pixel.max(1.0); // Move by one pixel worth of samples, minimum 1 sample
            
            let old_position = self.cursor_position;
            self.cursor_position = (self.cursor_position - step_size).max(0.0);
            
            // Debug output
            let cursor_screen_x = self.time_to_screen_x(self.cursor_position);
            println!("Cursor moved left: {} -> {}, screen_x: {}, step_size: {:.2}", 
                     old_position, self.cursor_position, cursor_screen_x, step_size);
            
            // Auto-scroll if cursor goes off-screen
            let cursor_screen_x = (self.cursor_position / samples_per_pixel) - self.scroll_position;
            
            if cursor_screen_x < 0.0 {
                self.scroll_position = (self.cursor_position / samples_per_pixel) - (WIDTH as f32 * 0.1);
                self.scroll_position = self.scroll_position.max(0.0);
                println!("Auto-scrolled left: scroll_position = {}", self.scroll_position);
            }
        }
    }

    fn move_cursor_right(&mut self) {
        if !self.audio_samples.is_empty() {
            // Calculate step size based on screen pixels for more precise movement when zoomed in
            let samples_per_pixel = (self.audio_samples.len() as f32) / (WIDTH as f32 * self.zoom_level);
            let step_size = samples_per_pixel.max(1.0); // Move by one pixel worth of samples, minimum 1 sample
            
            let max_position = self.audio_samples.len() as f32;
            let old_position = self.cursor_position;
            self.cursor_position = (self.cursor_position + step_size).min(max_position);
            
            // Debug output
            let cursor_screen_x = self.time_to_screen_x(self.cursor_position);
            println!("Cursor moved right: {} -> {}, screen_x: {}, step_size: {:.2}", 
                     old_position, self.cursor_position, cursor_screen_x, step_size);
            
            // Auto-scroll if cursor goes off-screen
            let cursor_screen_x = (self.cursor_position / samples_per_pixel) - self.scroll_position;
            
            if cursor_screen_x > WIDTH as f32 {
                self.scroll_position = (self.cursor_position / samples_per_pixel) - (WIDTH as f32 * 0.9);
                println!("Auto-scrolled right: scroll_position = {}", self.scroll_position);
            }
        }
    }

    fn add_marker_at_cursor_position(&mut self) {
        self.markers.push(self.cursor_position);
        self.markers.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!("Added marker at position: {}", self.cursor_position);
    }

    fn zoom_in(&mut self) {
        self.zoom_level = (self.zoom_level * 1.2).min(100.0);

        if !self.audio_samples.is_empty() {
            let samples_per_pixel = (self.audio_samples.len() as f32) / (WIDTH as f32 * self.zoom_level);
            let center_x = WIDTH as f32 / 2.0;
            let mut desired_scroll = (self.cursor_position / samples_per_pixel) - center_x;
            let max_scroll = ((self.audio_samples.len() as f32) / samples_per_pixel) - WIDTH as f32;
            let max_scroll = max_scroll.max(0.0);
            self.scroll_position = desired_scroll.clamp(0.0, max_scroll);
        }

        println!("Zoomed in: zoom_level = {}, centered on cursor", self.zoom_level);
    }

    fn zoom_out(&mut self) {
        // Calculate minimum zoom level to show entire waveform
        let min_zoom = 1.0; // Minimum zoom is 1.0 to show the entire waveform
        self.zoom_level = (self.zoom_level / 1.2).max(min_zoom);

        // Center the view on the cursor when zooming out, clamped to bounds
        if !self.audio_samples.is_empty() {
            let samples_per_pixel = (self.audio_samples.len() as f32) / (WIDTH as f32 * self.zoom_level);
            let center_x = WIDTH as f32 / 2.0;
            let mut desired_scroll = (self.cursor_position / samples_per_pixel) - center_x;
            let max_scroll = ((self.audio_samples.len() as f32) / samples_per_pixel) - WIDTH as f32;
            let max_scroll = max_scroll.max(0.0);
            self.scroll_position = desired_scroll.clamp(0.0, max_scroll);
        }

        println!("Zoomed out: zoom_level = {}, centered on cursor", self.zoom_level);
    }

    fn reset_view(&mut self) {
        self.zoom_level = 1.0;
        self.scroll_position = 0.0;
        println!("View reset to default: zoom_level = {}, scroll_position = {}", 
                 self.zoom_level, self.scroll_position);
    }

    pub fn render(&mut self) {
        // Calculate values we need before any borrowing
        let cursor_x = if !self.audio_samples.is_empty() {
            Some(self.time_to_screen_x(self.cursor_position) as u32)
        } else {
            None
        };
        
        // Calculate marker positions first to avoid borrowing issues
        let marker_positions: Vec<(usize, u32, bool)> = self.markers.iter().enumerate()
            .map(|(i, &marker_time)| {
                let marker_x = self.time_to_screen_x(marker_time) as u32;
                let is_selected = self.selected_marker == Some(i);
                (i, marker_x, is_selected)
            })
            .collect();

        // Calculate slice marker positions
        let slice_marker_positions: Vec<u32> = self.slice_markers.iter()
            .map(|&marker_time| self.time_to_screen_x(marker_time) as u32)
            .collect();

        // Only render if we have pixels (windowed mode)
        if let Some(ref mut pixels) = self.pixels {
            // Get frame buffer and perform all drawing operations
            let frame = pixels.frame_mut();
        
        // Clear frame
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 20;  // R
            pixel[1] = 20;  // G
            pixel[2] = 30;  // B
            pixel[3] = 255; // A
        }

        // Draw waveform
        if !self.audio_samples.is_empty() {
            let samples_per_pixel = (self.audio_samples.len() as f32) / (WIDTH as f32 * self.zoom_level);
            let waveform_center = WAVEFORM_HEIGHT / 2;
            let waveform_scale = (WAVEFORM_HEIGHT / 2) as f32 * 0.8;

            for x in 0..WIDTH {
                let sample_start = ((x as f32 + self.scroll_position) * samples_per_pixel) as usize;
                let sample_end = (((x + 1) as f32 + self.scroll_position) * samples_per_pixel) as usize;
                
                if sample_start >= self.audio_samples.len() {
                    break;
                }
                
                let sample_end = sample_end.min(self.audio_samples.len());
                
                // Find min and max in this pixel range
                let mut min_val = 0.0f32;
                let mut max_val = 0.0f32;
                
                for i in sample_start..sample_end {
                    let sample = self.audio_samples[i];
                    min_val = min_val.min(sample);
                    max_val = max_val.max(sample);
                }
                
                // Convert to screen coordinates
                let min_y = (waveform_center as f32 - min_val * waveform_scale) as u32;
                let max_y = (waveform_center as f32 - max_val * waveform_scale) as u32;
                
                // Draw vertical line for this pixel
                let start_y = min_y.min(max_y).min(WAVEFORM_HEIGHT - 1);
                let end_y = min_y.max(max_y).min(WAVEFORM_HEIGHT - 1);
                
                for y in start_y..=end_y {
                    let pixel_index = ((y * WIDTH + x) * 4) as usize;
                    if pixel_index + 3 < frame.len() {
                        frame[pixel_index] = 100;     // R
                        frame[pixel_index + 1] = 150; // G
                        frame[pixel_index + 2] = 255; // B
                        frame[pixel_index + 3] = 255; // A
                    }
                }
            }
        }

        // Draw markers using pre-calculated positions
        for (_i, marker_x, is_selected) in marker_positions {
            if marker_x < WIDTH {
                // Draw marker line
                for y in 0..HEIGHT {
                    let pixel_index = ((y * WIDTH + marker_x) * 4) as usize;
                    if pixel_index + 3 < frame.len() {
                        frame[pixel_index] = if is_selected { 255 } else { 255 };     // R
                        frame[pixel_index + 1] = if is_selected { 100 } else { 50 };  // G
                        frame[pixel_index + 2] = if is_selected { 100 } else { 50 };  // B
                        frame[pixel_index + 3] = 255; // A
                    }
                }
            }
        }

        // Draw cursor - make it more visible as a thick vertical line
        if let Some(cursor_x) = cursor_x {
            println!("Drawing cursor at screen_x: {}, cursor_position: {}, within bounds: {}", 
                     cursor_x, self.cursor_position, cursor_x < WIDTH);
            if cursor_x < WIDTH {
                // Draw a thick cursor line (3 pixels wide)
                for offset in -1..=1i32 {
                    let draw_x = (cursor_x as i32 + offset) as u32;
                    if draw_x < WIDTH {
                        for y in 0..WAVEFORM_HEIGHT {
                            let pixel_index = ((y * WIDTH + draw_x) * 4) as usize;
                            if pixel_index + 3 < frame.len() {
                                frame[pixel_index] = 255;     // R - bright yellow cursor
                                frame[pixel_index + 1] = 255; // G
                                frame[pixel_index + 2] = 0;   // B - yellow for high visibility
                                frame[pixel_index + 3] = 255; // A
                            }
                        }
                    }
                }
            }
        } else {
            println!("Cursor not drawn - no audio samples loaded");
        }
        }
    }

    pub fn create_slice_array(&self) -> Vec<f32> {
        self.markers.clone()
    }

    // Slice marker methods
    pub fn add_slice_marker(&mut self) {
        self.slice_markers.push(self.cursor_position);
        // Sort markers by position and renumber them
        self.reorder_slice_markers();
        println!("Added slice marker at position: {}", self.cursor_position);
    }

    // Reorder slice markers by position and maintain sequential numbering
    fn reorder_slice_markers(&mut self) {
        // Sort markers by their position
        self.slice_markers.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }

    pub fn get_slice_markers(&self) -> &Vec<f32> {
        &self.slice_markers
    }

    pub fn remove_slice_marker(&mut self, index: usize) {
        if index < self.slice_markers.len() {
            let removed_marker = self.slice_markers.remove(index);
            println!("Removed slice marker at position: {}", removed_marker);
        }
    }

    pub fn clear_slice_markers(&mut self) {
        self.slice_markers.clear();
    }
    
    pub fn load_slice_markers(&mut self, markers: Vec<f32>) {
        let marker_count = markers.len();
        self.slice_markers = markers;
        self.reorder_slice_markers();
        println!("Loaded {} slice markers", marker_count);
    }

    // Method to update cursor position from external source (for integrated mode)
    pub fn set_cursor_position(&mut self, position: f32) {
        self.cursor_position = position;
    }

    pub fn get_cursor_position(&self) -> f32 {
        self.cursor_position
    }

    pub fn get_sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn get_loaded_sample_key(&self) -> Option<&str> {
        self.loaded_sample_key.as_deref()
    }
}