use std::io::stdout;

use ratatui::{
    prelude::CrosstermBackend, style::{Style, Stylize}, text::{Line, Span, Text}, widgets::Cell
};


fn ui_table_snaps() {
    let mut stdout = stdout();
    let backend = CrosstermBackend::new(stdout)
}
