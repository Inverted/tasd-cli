use std::path::PathBuf;
use std::collections::HashSet;
use tasd_lib::TASD;
use tasd_lib::Packet;
use color_eyre::Result;
use ratatui::style::Color;
use crossterm::event::{KeyEvent, KeyModifiers};

/// Current view/mode of the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Normal navigation mode
    Normal,
    /// Inputting mode (unused but kept for future extension)
    Input,
    /// Help screen mode
    Help,
    /// Command mode
    Command,
}

/// Input position information
#[derive(Debug, Clone, Copy)]
pub struct InputCursor {
    /// Current input index
    pub input_index: usize,
    /// Total number of inputs
    pub total_inputs: usize,
}

impl InputCursor {
    pub fn new() -> Self {
        Self {
            input_index: 0,
            total_inputs: 0,
        }
    }

    pub fn next(&mut self) {
        if self.input_index < self.total_inputs.saturating_sub(1) {
            self.input_index += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.input_index > 0 {
            self.input_index -= 1;
        }
    }

    pub fn jump_to(&mut self, index: usize) {
        if index < self.total_inputs {
            self.input_index = index;
        } else if self.total_inputs > 0 {
            self.input_index = self.total_inputs - 1;
        }
    }

    /// Move cursor by specified number of steps
    pub fn move_by(&mut self, steps: isize) {
        if steps > 0 {
            let new_pos = self.input_index.saturating_add(steps as usize);
            self.jump_to(new_pos);
        } else if steps < 0 {
            let new_pos = self.input_index.saturating_sub((-steps) as usize);
            self.jump_to(new_pos);
        }
    }
}

/// Stores the application state
pub struct App {
    /// Path to the TASD file
    pub file_path: PathBuf,
    /// TASD data
    pub tasd: TASD,
    /// Current application mode
    pub mode: AppMode,
    /// Should the application exit
    pub exit: bool,
    /// Position in the input list
    pub cursor: InputCursor,
    /// Visible inputs window (start index)
    pub input_window_start: usize,
    /// Display settings
    pub display: DisplaySettings,
    /// Available ports (1-based port numbers)
    pub ports: Vec<u8>,
    /// Vim-style number prefix for commands
    pub number_buffer: Option<usize>,
    /// Command buffer
    pub command_buffer: String,
}

/// UI display settings
pub struct DisplaySettings {
    /// Show debug information
    pub show_debug: bool,
    /// Highlight color
    pub highlight_color: Color,
    /// Maximum inputs to show at once - dynamically updated based on window size
    pub max_visible_inputs: usize,
}

impl DisplaySettings {
    pub fn new() -> Self {
        Self {
            show_debug: false,
            highlight_color: Color::Yellow,
            max_visible_inputs: 20, // Default value, will be updated based on window size
        }
    }
}

impl App {
    pub fn new(tasd: TASD, file_path: PathBuf) -> Self {
        // Detect available ports
        let ports = App::detect_ports(&tasd);

        // Count total inputs
        let total_inputs = App::count_inputs(&tasd);

        let mut cursor = InputCursor::new();
        cursor.total_inputs = total_inputs;

        Self {
            file_path,
            tasd,
            mode: AppMode::Normal,
            exit: false,
            cursor,
            input_window_start: 0,
            display: DisplaySettings::new(),
            ports,
            number_buffer: None,
            command_buffer: String::new(),
        }
    }

    /// Detect all ports used in the TASD file
    fn detect_ports(tasd: &TASD) -> Vec<u8> {
        let mut port_set = HashSet::new();

        for packet in &tasd.packets {
            match packet {
                Packet::InputChunk(chunk) => {
                    port_set.insert(chunk.port);
                }
                Packet::InputMoment(moment) => {
                    port_set.insert(moment.port);
                }
                Packet::PortController(controller) => {
                    port_set.insert(controller.port);
                }
                _ => {}
            }
        }

        let mut ports: Vec<u8> = port_set.into_iter().collect();
        ports.sort(); // Sort ports numerically

        if ports.is_empty() {
            // Default to port 1 if no ports are found
            ports.push(1);
        }

        ports
    }

    /// Count total inputs in the TASD file - improved to be more accurate
    fn count_inputs(tasd: &TASD) -> usize {
        // First, check if there's a TotalFrames packet
        for packet in &tasd.packets {
            if let Packet::TotalFrames(tf) = packet {
                return tf.frames as usize;
            }
        }

        // If no TotalFrames packet, try to count frames from input chunks
        let mut max_inputs = 0;

        for port in 1..=4 { // Check common port numbers
            let mut inputs_for_port = 0;

            for packet in &tasd.packets {
                if let Packet::InputChunk(chunk) = packet {
                    if chunk.port == port {
                        // For NES, usually each byte is one input frame
                        inputs_for_port += chunk.inputs.len();
                    }
                }
            }

            max_inputs = max_inputs.max(inputs_for_port);
        }

        // If we have input chunks, return that count
        if max_inputs > 0 {
            return max_inputs;
        }

        // Fallback: count all input moments
        tasd.packets.iter()
            .filter(|p| matches!(p, Packet::InputMoment(_)))
            .count()
    }

    /// Update visible window to ensure cursor is visible
    pub fn update_input_window(&mut self) {
        // If cursor is before visible window, adjust window start
        if self.cursor.input_index < self.input_window_start {
            self.input_window_start = self.cursor.input_index;
        }
        // If cursor is past visible window, adjust window start to show cursor
        else if self.cursor.input_index >= self.input_window_start + self.display.max_visible_inputs {
            self.input_window_start = self.cursor.input_index.saturating_sub(self.display.max_visible_inputs) + 1;
        }

        // Ensure we don't scroll past the end
        let max_start = self.cursor.total_inputs.saturating_sub(self.display.max_visible_inputs);
        if self.input_window_start > max_start {
            self.input_window_start = max_start;
        }
    }

    /// Center the current input in the visible window
    pub fn center_cursor(&mut self) {
        let half_height = self.display.max_visible_inputs / 2;
        if self.cursor.input_index >= half_height {
            self.input_window_start = self.cursor.input_index - half_height;
        } else {
            self.input_window_start = 0;
        }

        // Ensure we don't scroll past the end
        let max_start = self.cursor.total_inputs.saturating_sub(self.display.max_visible_inputs);
        if self.input_window_start > max_start {
            self.input_window_start = max_start;
        }
    }

    /// Move cursor to top of visible window
    pub fn cursor_to_top(&mut self) {
        self.cursor.jump_to(self.input_window_start);
    }

    /// Move cursor to middle of visible window
    pub fn cursor_to_middle(&mut self) {
        let middle = self.input_window_start + (self.display.max_visible_inputs / 2);
        self.cursor.jump_to(middle);
    }

    /// Move cursor to bottom of visible window
    pub fn cursor_to_bottom(&mut self) {
        let bottom = (self.input_window_start + self.display.max_visible_inputs - 1).min(self.cursor.total_inputs - 1);
        self.cursor.jump_to(bottom);
    }

    /// Handle a digit input for number buffer
    pub fn handle_digit(&mut self, digit: u8) {
        let digit = digit as usize;
        self.number_buffer = Some(self.number_buffer.unwrap_or(0) * 10 + digit);
    }

    /// Get and clear the number buffer
    pub fn take_number_buffer(&mut self) -> usize {
        let count = self.number_buffer.unwrap_or(1);
        self.number_buffer = None;
        count
    }

    /// Handle key events
    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match self.mode {
            AppMode::Normal => self.handle_normal_key_event(key_event),
            AppMode::Input => self.handle_input_key_event(key_event),
            AppMode::Help => self.handle_help_key_event(key_event),
            AppMode::Command => self.handle_command_key_event(key_event),
        }
    }

    fn handle_normal_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;

        // Handle number prefixes for vim-style counts
        if let KeyCode::Char(c) = key_event.code {
            if c.is_ascii_digit() {
                let digit = c.to_digit(10).unwrap() as u8;
                self.handle_digit(digit);
                return Ok(());
            }
        }

        match key_event.code {
            KeyCode::Char('q') => self.exit(),

            // Basic navigation
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.take_number_buffer();
                self.cursor.move_by(count as isize);
                self.update_input_window();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.take_number_buffer();
                self.cursor.move_by(-(count as isize));
                self.update_input_window();
            }

            // Vim-like half-page movement
            KeyCode::Char('d') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                let count = self.take_number_buffer();
                let half_page = self.display.max_visible_inputs / 2;
                self.cursor.move_by((half_page * count) as isize);
                self.update_input_window();
            }
            KeyCode::Char('u') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                let count = self.take_number_buffer();
                let half_page = self.display.max_visible_inputs / 2;
                self.cursor.move_by(-((half_page * count) as isize));
                self.update_input_window();
            }

            // Full page movement
            KeyCode::Char('f') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                let count = self.take_number_buffer();
                let page = self.display.max_visible_inputs;
                self.cursor.move_by((page * count) as isize);
                self.update_input_window();
            }
            KeyCode::Char('b') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                let count = self.take_number_buffer();
                let page = self.display.max_visible_inputs;
                self.cursor.move_by(-((page * count) as isize));
                self.update_input_window();
            }
            KeyCode::PageDown => {
                let count = self.take_number_buffer();
                let page = self.display.max_visible_inputs;
                self.cursor.move_by((page * count) as isize);
                self.update_input_window();
            }
            KeyCode::PageUp => {
                let count = self.take_number_buffer();
                let page = self.display.max_visible_inputs;
                self.cursor.move_by(-((page * count) as isize));
                self.update_input_window();
            }

            // Go to start/end
            KeyCode::Char('g') => {
                if self.number_buffer.is_some() {
                    // Go to specific line if number is specified
                    let line = self.take_number_buffer();
                    self.cursor.jump_to(line.saturating_sub(1)); // Convert from 1-indexed to 0-indexed
                } else {
                    // Otherwise go to first line
                    self.cursor.jump_to(0);
                }
                self.update_input_window();
            }
            KeyCode::Char('G') => {
                if self.number_buffer.is_some() {
                    // Go to specific line if number is specified
                    let line = self.take_number_buffer();
                    self.cursor.jump_to(line.saturating_sub(1)); // Convert from 1-indexed to 0-indexed
                } else {
                    // Otherwise go to last line
                    self.cursor.jump_to(self.cursor.total_inputs.saturating_sub(1));
                }
                self.update_input_window();
            }

            // Position cursor within window (vim's H, M, L)
            KeyCode::Char('H') => {
                self.cursor_to_top();
            }
            KeyCode::Char('M') => {
                self.cursor_to_middle();
            }
            KeyCode::Char('L') => {
                self.cursor_to_bottom();
            }

            // Center current line (vim's zz)
            KeyCode::Char('z') => {
                self.center_cursor();
            }

            // Command mode
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.mode = AppMode::Command;
            }

            // Help & debug
            KeyCode::Char('?') => {
                self.mode = AppMode::Help;
            }
            KeyCode::Char('D') => {
                self.display.show_debug = !self.display.show_debug;
            }

            // Cancel number buffer
            KeyCode::Esc => {
                self.number_buffer = None;
            }

            _ => {}
        }
        Ok(())
    }

    fn handle_input_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;

        match key_event.code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_help_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;

        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_command_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        use crossterm::event::KeyCode;

        match key_event.code {
            KeyCode::Enter => {
                self.execute_command();
                self.mode = AppMode::Normal;
            }
            KeyCode::Esc => {
                self.command_buffer.clear();
                self.mode = AppMode::Normal;
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    /// Execute a command
    fn execute_command(&mut self) {
        let cmd = self.command_buffer.trim();

        // Parse commands similar to vim
        if cmd == "q" || cmd == "quit" {
            self.exit();
        } else if let Some(line_num) = cmd.parse::<usize>().ok() {
            // Go to specific line number (1-indexed)
            self.cursor.jump_to(line_num.saturating_sub(1));
            self.update_input_window();
        }

        self.command_buffer.clear();
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}