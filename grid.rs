#[derive(Debug, Clone)]
pub struct GridState {
    pub width: u32,
    pub height: u32,
    pub cursor_x: u32,
    pub cursor_y: u32,
    pub cells: Vec<Vec<bool>>, // 2D grid of cells
}

impl GridState {
    pub fn new(width: u32, height: u32) -> Self {
        let cells = vec![vec![false; width as usize]; height as usize];
        
        Self {
            width,
            height,
            cursor_x: 0,
            cursor_y: 0,
            cells,
        }
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        let new_x = (self.cursor_x as i32 + dx).max(0) as u32;
        let new_y = (self.cursor_y as i32 + dy).max(0) as u32;
        
        self.cursor_x = new_x.min(self.width - 1);
        self.cursor_y = new_y.min(self.height - 1);
    }

    pub fn toggle_cell_at(&mut self, x: u32, y: u32) {
        if x < self.width && y < self.height {
            let row = y as usize;
            let col = x as usize;
            if row < self.cells.len() && col < self.cells[row].len() {
                self.cells[row][col] = !self.cells[row][col];
                self.cursor_x = x;
                self.cursor_y = y;
            }
        }
    }

    pub fn toggle_cell(&mut self) {
        self.toggle_cell_at(self.cursor_x, self.cursor_y);
    }

    pub fn get_cell(&self, x: u32, y: u32) -> bool {
        if x < self.width && y < self.height {
            let row = y as usize;
            let col = x as usize;
            if row < self.cells.len() && col < self.cells[row].len() {
                return self.cells[row][col];
            }
        }
        false
    }
}