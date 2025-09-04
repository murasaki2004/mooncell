use std::path::PathBuf;
use std::fs::read_to_string;
use ratatui::{
    layout::{Constraint, Direction, Layout}, widgets::{Block, Paragraph}, Frame
};

pub struct Fileview {
    path: PathBuf,
    terminal_size: (u16, u16),
    start_number: usize,
}

impl Fileview {
    pub fn new() -> Self {
        Self {
            path: PathBuf::new(),
            terminal_size: (0, 0),
            start_number: 1,
        }
    }

    pub fn start_number_rezero(&mut self) {
        self.start_number = 1;
    }

    pub fn start_number_up(&mut self) {
        if self.start_number > 1 {
            self.start_number -= 1;
        }
    }

    pub fn start_number_down(&mut self) {
        self.start_number += 1;
    }

    pub fn set_path(&mut self, path: &str) {
        self.path = PathBuf::from(path);
    }

    pub fn refresh_termainal_size(&mut self, size: (u16, u16)) {
        self.terminal_size = size;
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
        let file_content_str = self.str_display_format(file_content_str);
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

    fn str_display_format(&self, str: String) -> String {
        // 添加行号
        let numbered_str = str
            .lines()                         // 将内容按行分割
            .enumerate()                    // 枚举行号（从0开始）
            .map(|(i, line)| format!("{}|   {}", Self::number_format(i + 1), line)) // 添加行号并格式化
            .collect::<Vec<String>>()       // 收集为 Vec<String>
            .join("\n");                    // 用换行符连接成最终字符串

        // 按照终端尺寸对内容分割
        let mut format_str = String::new();
        for line in numbered_str.split_inclusive('\n') {
            if line.len() > (self.terminal_size.0 - 9) as usize {
                let mut line_length = 0;
                let mut new_line = String::new();
                for ch in line.chars() {
                    line_length += 1;
                    if line_length == (self.terminal_size.0 - 9) {
                        new_line.push_str(&format!("\n       {}", ch));
                        line_length = 1;
                    } else {
                        new_line.push(ch);
                    }
                }
                format_str.push_str(&new_line);
            } else {
                format_str.push_str(line);
            }
        }

        // 删去起始前的内容
        let mut count: usize = 1;
        let mut neo_format_str = String::new();
        for line in format_str.split_inclusive('\n') {
            if count >= self.start_number {
                neo_format_str.push_str(line);
            }
            count += 1;
        }

        return neo_format_str;
    }

    fn number_format(number: usize) -> String {
        if number < 10 {
            return format!("{}  ", number);
        } else if number < 100 {
            return format!("{} ", number);
        } else {
            return format!("{}", number);
        }
    }
}