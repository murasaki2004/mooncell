use super::TopError;

use std::path::PathBuf;
use std::fs::{File, read_to_string};
use ratatui::{
    Frame,
    layout::{Layout, Direction, Constraint},
    widgets::{Paragraph, Block},
};

pub struct Fileview {
    path: PathBuf,
}

impl Fileview {
    pub fn new() -> Self {
        Self {
            path: PathBuf::new()
        }
    }

    pub fn set_path(&mut self, path: &str) {
        self.path = PathBuf::from(path);
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let layout_fileview = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Fill(1),
            ])
            .split(frame.area());

        let file_content_str = match read_to_string(self.path.clone()) {
            Err(e) => String::from(format!("file error\n{}", e.to_string())),
            Ok(str) => str,
        };
        let file_content_p = Paragraph::new(file_content_str.clone())
            .block(Block::bordered().title(self.get_path_str()));
        frame.render_widget(file_content_p, layout_fileview[0]);
    }

    pub fn get_path_str(&self) -> String {
        return match self.path.to_str() {
            None => String::from("get path error"),
            Some(str) => str.to_string(),
        }
    }
}