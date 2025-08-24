use crate::interpreter::Value;

#[derive(Debug, Clone)]
pub struct Menu {
    pub title: String,
    pub options: Vec<MenuOption>,
    pub selected_index: usize,
    pub context_object_id: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MenuOption {
    pub label: String,
    pub action: MenuAction,
}

#[derive(Debug, Clone)]
pub enum MenuAction {
    LoadSample, // Will execute sample(self)
    Close,
}

impl Menu {
    pub fn new_object_menu(object_id: u32) -> Self {
        Self {
            title: format!("Object {} Menu", object_id),
            options: vec![
                MenuOption {
                    label: "Load Sample".to_string(),
                    action: MenuAction::LoadSample,
                },
            ],
            selected_index: 0,
            context_object_id: Some(object_id),
        }
    }
    
    pub fn new_coordinate_menu(x: u32, y: u32) -> Self {
        Self {
            title: format!("Position ({}, {}) Menu", x, y),
            options: vec![
                MenuOption {
                    label: "Load Sample".to_string(),
                    action: MenuAction::LoadSample,
                },
            ],
            selected_index: 0,
            context_object_id: None,
        }
    }
    
    pub fn execute_selected_action(&self) -> Option<String> {
        if let Some(option) = self.options.get(self.selected_index) {
            match option.action {
                MenuAction::LoadSample => {
                    if let Some(object_id) = self.context_object_id {
                        Some(format!("sample({})", object_id))
                    } else {
                        Some("sample(self)".to_string())
                    }
                },
                MenuAction::Close => None,
            }
        } else {
            None
        }
    }
}