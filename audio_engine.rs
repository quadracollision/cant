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
}

// Remove the global static and make AudioEngine thread-local instead
thread_local! {
    static AUDIO_ENGINE: std::cell::RefCell<Option<AudioEngine>> = std::cell::RefCell::new(None);
}

pub struct AudioEngine {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    samples: HashMap<String, AudioSample>,
}

impl AudioEngine {
    pub fn new() -> Result<Self, AudioError> {
        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| AudioError::InitError(format!("Failed to create audio stream: {}", e)))?;
        
        Ok(Self {
            _stream,
            stream_handle,
            samples: HashMap::new(),
        })
    }
    
    pub fn load_audio_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<String, AudioError> {
        let path = file_path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        
        // Read the entire file into memory for fast playback
        let file = File::open(path)
            .map_err(|e| AudioError::LoadError(format!("Cannot open file {}: {}", path_str, e)))?;
        
        // Validate that the file can be decoded
        let buf_reader = BufReader::new(file);
        let _decoder = Decoder::new(buf_reader)
            .map_err(|e| AudioError::LoadError(format!("Cannot decode audio file {}: {}", path_str, e)))?;
        
        // Read file data into memory
        let file_data = std::fs::read(path)
            .map_err(|e| AudioError::LoadError(format!("Cannot read file {}: {}", path_str, e)))?;
        
        let sample = AudioSample {
            data: Arc::new(file_data),
            file_path: path_str.clone(),
        };
        
        // Store the sample
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