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

/// Game state enum to track different states
#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Stopped,  // Not playing, will reset to original state on play
    Playing,  // Currently running physics
    Paused,   // Paused, will resume from current state on play
}

/// Manages game state transitions and play mode
#[derive(Debug)]
pub struct GameStateManager {
    pub saved_state: Option<SavedGameState>,  // Original state before first play
    pub paused_state: Option<SavedGameState>, // Current state when paused
    state: GameState,
}

impl GameStateManager {
    /// Creates a new GameStateManager in stopped state
    pub fn new() -> Self {
        Self {
            saved_state: None,
            paused_state: None,
            state: GameState::Stopped,
        }
    }

    /// Saves the original game state for later restoration (used before first play)
    pub fn save_original_state(
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

    /// Saves the current paused state
    pub fn save_paused_state(
        &mut self,
        game_objects: &GameObjectManager,
        grid_state: &Option<GridState>,
        environment: &HashMap<String, Value>,
    ) {
        self.paused_state = Some(SavedGameState {
            game_objects: game_objects.clone(),
            grid_state: grid_state.clone(),
            environment: environment.clone(),
        });
    }

    /// Restores the original saved game state and returns it
    pub fn restore_original_state(&mut self) -> Option<SavedGameState> {
        self.saved_state.take()
    }

    /// Gets the paused state for resuming
    pub fn get_paused_state(&self) -> Option<&SavedGameState> {
        self.paused_state.as_ref()
    }

    /// Retrieves the saved game state if available
    pub fn get_saved_state(&self) -> Option<&SavedGameState> {
        self.saved_state.as_ref()
    }

    /// Starts play mode
    pub fn start_play(&mut self) {
        self.state = GameState::Playing;
    }

    /// Pauses the game (preserves current state for resume)
    pub fn pause_play(&mut self) {
        self.state = GameState::Paused;
    }

    /// Stops the game (will reset to original state on next play)
    pub fn stop_play(&mut self) {
        self.state = GameState::Stopped;
        self.paused_state = None; // Clear paused state when stopping
    }

    /// Returns whether the game is currently in play mode
    pub fn is_playing(&self) -> bool {
        self.state == GameState::Playing
    }

    /// Returns whether the game is currently paused
    pub fn is_paused(&self) -> bool {
        self.state == GameState::Paused
    }

    /// Returns whether the game is stopped
    pub fn is_stopped(&self) -> bool {
        self.state == GameState::Stopped
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