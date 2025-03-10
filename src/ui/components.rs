use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Table, Row, Cell, Widget},
};
use tasd_lib::Packet;
use std::collections::HashMap;

use crate::app::{App, AppMode};

/// Render the sidebar with metadata
pub fn render_sidebar(app: &App, area: Rect, buf: &mut Buffer) {
    // First, let's debug what packets we actually have
    let mut debug_info = Vec::new();
    for (i, packet) in app.tasd.packets.iter().enumerate() {
        if i < 100 { // Just show first 100 packets to avoid overwhelming
            debug_info.push(format!("Packet {}: {:?}", i, packet));
        }
    }

    // Extract all metadata from all relevant packets
    let mut metadata = Vec::new();

    // Always show file path
    metadata.push(("File", app.file_path.to_string_lossy().to_string()));

    // Go through all packets and collect metadata
    for packet in &app.tasd.packets {
        match packet {
            // Skip input chunks and moments
            Packet::InputChunk(_) | Packet::InputMoment(_) => continue,

            // Extract data from known packet types
            Packet::ConsoleType(ct) => {
                let console_names = [
                    (1, "NES"),
                    (2, "SNES"),
                    (3, "N64"),
                    (4, "GameCube"),
                    (5, "Game Boy"),
                    (6, "Game Boy Color"),
                    (7, "Game Boy Advance"),
                    (8, "Sega Genesis"),
                    (9, "Atari 2600")
                ];

                let console_name = console_names
                    .iter()
                    .find(|(code, _)| *code == ct.console as u8)
                    .map(|(_, name)| *name)
                    .unwrap_or("Unknown");

                if !ct.name.is_empty() {
                    metadata.push(("Console", format!("{} ({})", console_name, ct.name)));
                } else {
                    metadata.push(("Console", console_name.to_string()));
                }
            }
            Packet::ConsoleRegion(cr) => {
                let region = match cr.video_signal as u8 {
                    1 => "NTSC",
                    2 => "PAL",
                    _ => "Other",
                };
                metadata.push(("Region", region.to_string()));
            }
            Packet::GameTitle(gt) => {
                metadata.push(("Game Title", gt.title.clone()));
            }
            Packet::RomName(rn) => {
                metadata.push(("ROM", rn.name.clone()));
            }
            Packet::Attribution(at) => {
                let attr_type = match at.attribution_type as u8 {
                    1 => "Author",
                    2 => "Verifier",
                    3 => "File Creator",
                    4 => "File Editor",
                    _ => "Other",
                };
                metadata.push((attr_type, at.name.clone()));
            }
            Packet::Category(cat) => {
                metadata.push(("Category", cat.category.clone()));
            }
            Packet::EmulatorName(en) => {
                metadata.push(("Emulator", en.name.clone()));
            }
            Packet::EmulatorVersion(ev) => {
                metadata.push(("Emulator Version", ev.version.clone()));
            }
            // Packet::EmulatorCore(ec) => {
            //     metadata.push(("Emulator Core", ec.core.clone()));
            // }
            Packet::TotalFrames(tf) => {
                metadata.push(("Total Frames", tf.frames.to_string()));
            }
            Packet::TotalRerecords(tr) => {
                metadata.push(("Rerecords", tr.rerecords.to_string()));
            }
            Packet::SourceLink(sl) => {
                metadata.push(("Source", sl.link.clone()));
            }
            Packet::BlankFrames(bf) => {
                metadata.push(("Blank Frames", bf.blank_frames.to_string()));
            }
            Packet::Verified(v) => {
                metadata.push(("Verified", if v.verified { "Yes" } else { "No" }.to_string()));
            }
            Packet::MovieLicense(ml) => {
                metadata.push(("License", ml.license.clone()));
            }
            Packet::Comment(c) => {
                metadata.push(("Comment", c.comment.clone()));
            }
            // Add any other packet types you want to show
            _ => {}
        }
    }

    // Add UI information
    metadata.push(("Total Inputs", app.cursor.total_inputs.to_string()));
    metadata.push(("Current Input", app.cursor.input_index.to_string()));
    metadata.push(("Ports", format!("{:?}", app.ports)));
    metadata.push(("Debug", if app.display.show_debug { "On".to_string() } else { "Off".to_string() }));

    // Add number buffer if active
    if let Some(num) = app.number_buffer {
        metadata.push(("Repeat", num.to_string()));
    }

    // Create block and calculate inner area
    let block = Block::default()
        .title("TASD Metadata")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    // Calculate inner area BEFORE rendering the block
    let inner_area = block.inner(area);

    // Render the block
    block.render(area, buf);

    // Show normal metadata or debug info based on debug toggle
    if app.display.show_debug {
        // Show packet debug info
        let items: Vec<ListItem> = debug_info.iter()
            .map(|info| ListItem::new(Line::from(Span::raw(info))))
            .collect();

        Widget::render(
            List::new(items)
                .highlight_style(Style::default().fg(Color::Yellow)),
            inner_area,
            buf,
        );
    } else {
        // Show normal metadata
        let items: Vec<ListItem> = metadata.iter()
            .map(|(key, value)| {
                let content = Line::from(vec![
                    Span::styled(format!("{}: ", key), Style::default().fg(Color::Blue)),
                    Span::raw(value),
                ]);
                ListItem::new(content)
            })
            .collect();

        Widget::render(
            List::new(items)
                .highlight_style(Style::default().fg(Color::Yellow)),
            inner_area,
            buf,
        );
    }
}

/// Format NES controller input for display
fn format_nes_input(input_data: &[u8], input_idx: usize, debug: bool) -> String {
    if input_data.is_empty() {
        return if debug { format!("[{}] Empty", input_idx) } else { "· · · · · · · ·".to_string() };
    }

    let input_byte = input_data[0];

    if debug {
        // Debug display showing hex and binary
        format!(
            "[{}] 0x{:02X} {:08b}",
            input_idx,
            input_byte,
            input_byte
        )
    } else {
        // User-friendly display for normal view
        // NES controller bits are active LOW - 0 means pressed
        let a = (input_byte & 0x01) == 0;
        let b = (input_byte & 0x02) == 0;
        let select = (input_byte & 0x04) == 0;
        let start = (input_byte & 0x08) == 0;
        let up = (input_byte & 0x10) == 0;
        let down = (input_byte & 0x20) == 0;
        let left = (input_byte & 0x40) == 0;
        let right = (input_byte & 0x80) == 0;

        // Use consistent fixed-width formatting with spaces between buttons
        format!(
            "{} {} {} {} {} {} {} {}",
            if up { "↑" } else { "·" },
            if down { "↓" } else { "·" },
            if left { "←" } else { "·" },
            if right { "→" } else { "·" },
            if a { "A" } else { "·" },
            if b { "B" } else { "·" },
            if select { "S" } else { "·" },
            if start { "T" } else { "·" }
        )
    }
}

/// Simple function to collect all inputs from all chunks for a specific port
fn collect_port_inputs(packets: &[Packet], port: u8) -> Vec<u8> {
    let mut inputs = Vec::new();

    for packet in packets {
        if let Packet::InputChunk(chunk) = packet {
            if chunk.port == port {
                inputs.extend_from_slice(&chunk.inputs);
            }
        }
    }

    inputs
}

/// Render the main panel with inputs in a table format
pub fn render_inputs(app: &mut App, area: Rect, buf: &mut Buffer) {
    // Create the block with title based on mode
    let title = match app.mode {
        AppMode::Normal => format!("Inputs (Current: {})", app.cursor.input_index),
        AppMode::Command => format!("Command: {}", app.command_buffer),
        _ => format!("Inputs (Current: {})", app.cursor.input_index),
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    // Calculate inner area BEFORE rendering
    let inner_area = block.inner(area);

    // Render block
    block.render(area, buf);

    // Skip if no space
    if inner_area.width < 10 || inner_area.height < 2 {
        return;
    }

    // Update max visible inputs based on available height
    app.display.max_visible_inputs = inner_area.height.saturating_sub(2) as usize;

    // Ensure the current input is visible
    app.update_input_window();

    // Collect all inputs for each port - simple approach for debugging
    let mut all_port_inputs: HashMap<u8, Vec<u8>> = HashMap::new();
    for port in &app.ports {
        all_port_inputs.insert(*port, collect_port_inputs(&app.tasd.packets, *port));
    }

    // Create table rows with raw data for each port
    let mut rows = Vec::new();

    // Start from app.input_window_start and show as many as we can fit
    let start_idx = app.input_window_start;
    let end_idx = (start_idx + app.display.max_visible_inputs).min(app.cursor.total_inputs);

    for idx in start_idx..end_idx {
        let is_current = idx == app.cursor.input_index;

        // Define style for line number
        let idx_style = if is_current {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let mut cells = vec![
            Cell::from(Span::styled(
                format!("{:04}", idx),
                idx_style
            ))
        ];

        // Add a cell for each port
        for port in &app.ports {
            let empty_vec = Vec::new(); // Create a longer-lived value
            let port_inputs = all_port_inputs.get(port).unwrap_or(&empty_vec);

            let cell_content = if idx < port_inputs.len() {
                format_nes_input(&[port_inputs[idx]], idx, app.display.show_debug)
            } else {
                if app.display.show_debug {
                    format!("[{}] Out of range", idx)
                } else {
                    "· · · · · · · ·".to_string()
                }
            };

            // Define cell style
            let cell_style = if is_current {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            cells.push(Cell::from(Span::styled(cell_content, cell_style)));
        }

        rows.push(Row::new(cells));
    }

    // Create table header with port numbers
    let mut header = vec![
        Cell::from(Span::styled(
            "Input #",
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        ))
    ];

    for port in &app.ports {
        header.push(Cell::from(Span::styled(
            format!("Port {}", port),
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
        )));
    }

    // Calculate constraints for the table columns
    let mut constraints = vec![Constraint::Length(8)]; // Input number column
    for _ in &app.ports {
        constraints.push(Constraint::Min(20)); // Input data columns - wider for debug info
    }

    // Create and render the table
    let table = Table::new(rows, constraints)
        .header(Row::new(header))
        .row_highlight_style(Style::default().bg(app.display.highlight_color))
        .highlight_symbol("> ");

    Widget::render(table, inner_area, buf);
}

/// Render the status bar
pub fn render_status_bar(app: &App, area: Rect, buf: &mut Buffer) {
    let mode_text = match app.mode {
        AppMode::Normal => "NORMAL",
        AppMode::Input => "INPUT",
        AppMode::Help => "HELP",
        AppMode::Command => "COMMAND",
    };

    // Create elements based on app state
    let mut elements = vec![
        Span::styled(format!(" {} ", mode_text),
                     Style::default().bg(Color::Blue).fg(Color::White)),
        Span::raw(" | "),
        Span::styled(format!(" Input: {}/{} ", app.cursor.input_index, app.cursor.total_inputs),
                     Style::default().fg(Color::Yellow)),
    ];

    // Show number buffer if active
    if let Some(num) = app.number_buffer {
        elements.push(Span::raw(" | "));
        elements.push(Span::styled(format!(" Count: {} ", num),
                                   Style::default().fg(Color::Magenta)));
    }

    // Add keyboard shortcuts
    elements.extend_from_slice(&[
        Span::raw(" | "),
        Span::styled(" j/k: Navigate ", Style::default().fg(Color::Gray)),
        Span::raw(" | "),
        Span::styled(" D: Debug ", Style::default().fg(Color::Gray)),
        Span::raw(" | "),
        Span::styled(" ?: Help ", Style::default().fg(Color::Gray)),
    ]);

    let status = Line::from(elements);

    Paragraph::new(status)
        .style(Style::default().bg(Color::Black))
        .render(area, buf);
}

/// Render help dialog
pub fn render_help(area: Rect, buf: &mut Buffer) {
    let help_text = vec![
        "Navigation",
        "j/↓: Next input",
        "k/↑: Previous input",
        "g: Go to first input",
        "G: Go to last input",
        "H: Go to first visible line",
        "M: Go to middle visible line",
        "L: Go to last visible line",
        "z: Center current line",
        "Ctrl+d: Half page down",
        "Ctrl+u: Half page up",
        "Ctrl+f/PageDown: Full page down",
        "Ctrl+b/PageUp: Full page up",
        "NUMBER: Repeat next command N times",
        "",
        "Commands",
        ":q or :quit: Exit application",
        ":NUMBER: Jump to line number",
        "",
        "Other",
        "D: Toggle debug info",
        "Esc: Cancel operation",
        "q: Quit",
        "?: Show/hide help",
    ];

    let text = Text::from(
        help_text
            .iter()
            .map(|&line| {
                if line.is_empty() {
                    Line::raw(line)
                } else if !line.contains(':') {
                    Line::styled(line, Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD))
                } else {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    Line::from(vec![
                        Span::styled(format!("{}: ", parts[0]), Style::default().fg(Color::Yellow)),
                        Span::raw(parts[1]),
                    ])
                }
            })
            .collect::<Vec<Line>>(),
    );

    // Calculate dialog position (centered)
    let width = 50;
    let height = text.height() as u16 + 2;
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    // Create dialog block and calculate inner area BEFORE rendering
    let dialog_block = Block::default()
        .borders(Borders::ALL)
        .title("Help")
        .style(Style::default().fg(Color::White));

    let inner_dialog_area = dialog_block.inner(dialog_area);

    // Render a dark background behind the dialog
    Clear.render(dialog_area, buf);

    // Render the dialog block
    dialog_block.render(dialog_area, buf);

    // Render the text inside the dialog
    Paragraph::new(text)
        .render(inner_dialog_area, buf);
}

/// Render the entire UI
pub fn render(app: &mut App, frame: &mut ratatui::Frame) {
    // Split the screen into sidebar and main content
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(75),
        ])
        .split(frame.area());

    // Split main content into input panel and status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(chunks[1]);

    // Render the sidebar
    render_sidebar(app, chunks[0], frame.buffer_mut());

    // Render the input panel
    render_inputs(app, main_chunks[0], frame.buffer_mut());

    // Render the status bar
    render_status_bar(app, main_chunks[1], frame.buffer_mut());

    // Render help dialog if in help mode
    if app.mode == AppMode::Help {
        render_help(frame.area(), frame.buffer_mut());
    }
}