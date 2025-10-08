use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to load audio file: {0}")]
    LoadError(String),
    #[error("Audio playback error: {0}")]
    PlaybackError(String),
    #[error("Audio system initialization error: {0}")]
    InitError(String),
}

#[derive(Clone)]
pub struct AudioSample {
    pub data: Arc<Vec<u8>>,
    pub file_path: String,
    pub slice_markers: Vec<f64>, // Time positions in seconds for slice markers
}

#[derive(Clone)]
pub struct SliceArray {
    pub sample_key: String,
    pub sequence: Vec<usize>, // Sequence of marker indices to play
    pub current_index: usize,
}

// Remove the global static and make AudioEngine thread-local instead
thread_local! {
    static AUDIO_ENGINE: std::cell::RefCell<Option<AudioEngine>> = std::cell::RefCell::new(None);
}

pub struct AudioEngine {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    samples: HashMap<String, AudioSample>,
    slice_arrays: HashMap<String, SliceArray>, // Store slice arrays by name
}

impl AudioEngine {
    pub fn new() -> Result<Self, AudioError> {
        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| AudioError::InitError(format!("Failed to create audio stream: {}", e)))?;
        
        Ok(Self {
            _stream,
            stream_handle,
            samples: HashMap::new(),
            slice_arrays: HashMap::new(),
        })
    }
    
    pub fn load_audio_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<String, AudioError> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        
        // First, try to find the file in the samples directory
        let samples_path = if let Some(filename) = path.file_name() {
            let samples_file = format!("samples/{}", filename.to_string_lossy());
            if std::fs::metadata(&samples_file).is_ok() {
                Some(samples_file)
            } else {
                None
            }
        } else {
            None
        };
        
        // Use samples path if available, otherwise use original path
        let actual_path = samples_path.as_ref().map(|s| Path::new(s)).unwrap_or(path);
        let actual_path_str = actual_path.to_string_lossy().to_string();
        
        // Read the entire file into memory for fast playback
        let file = File::open(actual_path)
            .map_err(|e| AudioError::LoadError(format!("Cannot open file {}: {}", actual_path_str, e)))?;
        
        // Validate that the file can be decoded
        let buf_reader = BufReader::new(file);
        let _decoder = Decoder::new(buf_reader)
            .map_err(|e| AudioError::LoadError(format!("Cannot decode audio file {}: {}", actual_path_str, e)))?;
        
        // Read file data into memory
        let file_data = std::fs::read(actual_path)
            .map_err(|e| AudioError::LoadError(format!("Cannot read file {}: {}", actual_path_str, e)))?;
        
        let sample = AudioSample {
            data: Arc::new(file_data),
            file_path: actual_path_str.clone(),
            slice_markers: Vec::new(), // Initialize with empty markers
        };
        
        // Store the sample using the original path as key for consistency
        self.samples.insert(path_str.clone(), sample);
        
        Ok(path_str)
    }
    
    pub fn play_sample(&self, sample_key: &str) -> Result<(), AudioError> {
        let sample = self.samples.get(sample_key)
            .ok_or_else(|| AudioError::PlaybackError(format!("Sample not found: {}", sample_key)))?;
        
        // Create a cursor from the in-memory data
        let cursor = std::io::Cursor::new(sample.data.as_ref().clone());
        let decoder = Decoder::new(cursor)
            .map_err(|e| AudioError::PlaybackError(format!("Failed to decode sample: {}", e)))?;
        
        // Create a new sink for this playback
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(format!("Failed to create sink: {}", e)))?;
        
        sink.append(decoder);
        sink.detach(); // Let it play independently
        
        Ok(())
    }
    
    pub fn play_sample_with_volume(&self, sample_key: &str, volume: f32) -> Result<(), AudioError> {
        let sample = self.samples.get(sample_key)
            .ok_or_else(|| AudioError::PlaybackError(format!("Sample not found: {}", sample_key)))?;
        
        let cursor = std::io::Cursor::new(sample.data.as_ref().clone());
        let decoder = Decoder::new(cursor)
            .map_err(|e| AudioError::PlaybackError(format!("Failed to decode sample: {}", e)))?;
        
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(format!("Failed to create sink: {}", e)))?;
        
        sink.set_volume(volume.clamp(0.0, 1.0));
        sink.append(decoder);
        sink.detach();
        
        Ok(())
    }
    
    pub fn get_loaded_samples(&self) -> Vec<String> {
        self.samples.keys().cloned().collect()
    }
    
    pub fn remove_sample(&mut self, sample_key: &str) -> bool {
        self.samples.remove(sample_key).is_some()
    }
    
    // Slice array methods
    pub fn create_slice_array(&mut self, name: String, sample_key: String, sequence: Vec<usize>) -> Result<(), AudioError> {
        // Verify the sample exists
        if !self.samples.contains_key(&sample_key) {
            return Err(AudioError::PlaybackError(format!("Sample not found: {}", sample_key)));
        }
        
        let slice_array = SliceArray {
            sample_key,
            sequence,
            current_index: 0,
        };
        
        self.slice_arrays.insert(name, slice_array);
        Ok(())
    }
    
    pub fn set_sample_markers(&mut self, sample_key: &str, markers: Vec<f64>) -> Result<(), AudioError> {
        let sample = self.samples.get_mut(sample_key)
            .ok_or_else(|| AudioError::PlaybackError(format!("Sample not found: {}", sample_key)))?;
        
        sample.slice_markers = markers;
        Ok(())
    }
    
    pub fn play_slice_array(&mut self, array_name: &str) -> Result<(), AudioError> {
        // First, extract all the needed values without holding mutable references
        let (sample_key, current_marker_index, sequence_len) = {
            let slice_array = self.slice_arrays.get(array_name)
                .ok_or_else(|| AudioError::PlaybackError(format!("Slice array not found: {}", array_name)))?;
            
            if slice_array.sequence.is_empty() {
                return Err(AudioError::PlaybackError("Slice array sequence is empty".to_string()));
            }
            
            let current_marker_index = slice_array.sequence[slice_array.current_index];
            (slice_array.sample_key.clone(), current_marker_index, slice_array.sequence.len())
        };
        
        // Get the sample
        let sample = self.samples.get(&sample_key)
            .ok_or_else(|| AudioError::PlaybackError(format!("Sample not found: {}", sample_key)))?;
        
        // If no markers are set, play the whole sample
        if sample.slice_markers.is_empty() {
            self.play_sample(&sample_key)?;
        } else {
            // Validate marker index
            if current_marker_index >= sample.slice_markers.len() {
                return Err(AudioError::PlaybackError(format!("Invalid marker index: {}", current_marker_index)));
            }
            
            let start_time = sample.slice_markers[current_marker_index];
            let end_time = if current_marker_index + 1 < sample.slice_markers.len() {
                sample.slice_markers[current_marker_index + 1]
            } else {
                // Play to the end of the sample
                f64::INFINITY
            };
            
            // Play the slice from start_time to end_time
            self.play_sample_slice(&sample_key, start_time, end_time)?;
        }
        
        // Now update the slice array's current index
        let slice_array = self.slice_arrays.get_mut(array_name)
            .ok_or_else(|| AudioError::PlaybackError(format!("Slice array not found: {}", array_name)))?;
        slice_array.current_index = (slice_array.current_index + 1) % sequence_len;
        
        Ok(())
    }
    
    fn play_sample_slice(&self, sample_key: &str, start_time: f64, end_time: f64) -> Result<(), AudioError> {
        let sample = self.samples.get(sample_key)
            .ok_or_else(|| AudioError::PlaybackError(format!("Sample not found: {}", sample_key)))?;
        
        let cursor = std::io::Cursor::new(sample.data.as_ref().clone());
        let mut decoder = Decoder::new(cursor)
            .map_err(|e| AudioError::PlaybackError(format!("Failed to decode sample: {}", e)))?;
        
        // Skip to start time (approximate)
        let sample_rate = decoder.sample_rate() as f64;
        let channels = decoder.channels() as f64;
        let samples_to_skip = (start_time * sample_rate * channels) as usize;
        
        // Create a new decoder and skip samples
        let cursor = std::io::Cursor::new(sample.data.as_ref().clone());
        let decoder = Decoder::new(cursor)
            .map_err(|e| AudioError::PlaybackError(format!("Failed to decode sample: {}", e)))?;
        
        let skipped_decoder = decoder.skip_duration(std::time::Duration::from_secs_f64(start_time));
        
        // If we have an end time, take only the duration we need
        let final_decoder = if end_time != f64::INFINITY {
            let duration = end_time - start_time;
            Box::new(skipped_decoder.take_duration(std::time::Duration::from_secs_f64(duration))) as Box<dyn Source<Item = i16> + Send>
        } else {
            Box::new(skipped_decoder) as Box<dyn Source<Item = i16> + Send>
        };
        
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(format!("Failed to create sink: {}", e)))?;
        
        sink.append(final_decoder);
        sink.detach();
        
        Ok(())
    }
}

// Helper functions to work with the thread-local audio engine
pub fn with_audio_engine<F, R>(f: F) -> Result<R, AudioError>
where
    F: FnOnce(&mut AudioEngine) -> Result<R, AudioError>,
{
    AUDIO_ENGINE.with(|engine_cell| {
        let mut engine_opt = engine_cell.borrow_mut();
        if engine_opt.is_none() {
            *engine_opt = Some(AudioEngine::new()?);
        }
        
        if let Some(ref mut engine) = *engine_opt {
            f(engine)
        } else {
            Err(AudioError::InitError("Failed to initialize audio engine".to_string()))
        }
    })
}

pub fn play_audio_sample(sample_key: &str, volume: f32) -> Result<(), AudioError> {
    with_audio_engine(|engine| {
        engine.play_sample_with_volume(sample_key, volume)
    })
}

pub fn load_audio_file<P: AsRef<Path>>(file_path: P) -> Result<String, AudioError> {
    with_audio_engine(|engine| {
        engine.load_audio_file(file_path)
    })
}

pub fn create_slice_array(name: String, sample_key: String, sequence: Vec<usize>) -> Result<(), AudioError> {
    with_audio_engine(|engine| {
        engine.create_slice_array(name, sample_key, sequence)
    })
}

pub fn set_sample_markers(sample_key: &str, markers: Vec<f64>) -> Result<(), AudioError> {
    with_audio_engine(|engine| {
        engine.set_sample_markers(sample_key, markers)
    })
}

pub fn play_slice_array(array_name: &str) -> Result<(), AudioError> {
    with_audio_engine(|engine| {
        engine.play_slice_array(array_name)
    })
}