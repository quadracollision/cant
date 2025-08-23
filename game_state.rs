use crate::game_objects::GameObjectManager;
use crate::grid::GridState;
use std::collections::HashMap;
use crate::interpreter::Value;

/// Represents a saved snapshot of the game state
#[derive(Clone, Debug)]
pub struct SavedGameState {
    pub game_objects: GameObjectManager,
    pub grid_state: Option<GridState>,
    pub environment: HashMap<String, Value>,
}

/// Manages game state transitions and play mode
#[derive(Debug)]
pub struct GameStateManager {
    pub saved_state: Option<SavedGameState>,
    is_playing: bool,
}

impl GameStateManager {
    /// Creates a new GameStateManager in paused state
    pub fn new() -> Self {
        Self {
            saved_state: None,
            is_playing: false,
        }
    }

    /// Saves the current game state for later restoration
    pub fn save_state(
        &mut self,
        game_objects: &GameObjectManager,
        grid_state: &Option<GridState>,
        environment: &HashMap<String, Value>,
    ) {
        self.saved_state = Some(SavedGameState {
            game_objects: game_objects.clone(),
            grid_state: grid_state.clone(),
            environment: environment.clone(),
        });
    }

    /// Restores the saved game state and returns it
    pub fn restore_state(&mut self) -> Option<SavedGameState> {
        self.saved_state.take()
    }

    /// Retrieves the saved game state if available
    pub fn get_saved_state(&self) -> Option<&SavedGameState> {
        self.saved_state.as_ref()
    }

    /// Starts play mode
    pub fn start_play(&mut self) {
        self.is_playing = true;
    }

    /// Stops play mode (alias for stop_play)
    pub fn pause_play(&mut self) {
        self.is_playing = false;
    }

    /// Stops play mode
    pub fn stop_play(&mut self) {
        self.is_playing = false;
    }

    /// Toggles play mode
    pub fn toggle_play(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Returns whether the game is currently in play mode
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Returns whether there is a saved state available
    pub fn has_saved_state(&self) -> bool {
        self.saved_state.is_some()
    }
}

impl Default for GameStateManager {
    fn default() -> Self {
        Self::new()
    }
}