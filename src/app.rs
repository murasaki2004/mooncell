use core::{error, num};
use std::fs::{FileTimes, FileType};
use std::isize;
use std::{clone, default, io, process::exit};
use std::sync::mpsc;
use std::time::Duration;
// 
mod mooncell;
use mooncell::Mooncell;
// rataui
use color_eyre::{eyre, owo_colors::OwoColorize, Result};
use crossterm::{cursor::Show, event::{self, Event, KeyCode, KeyEvent, KeyEventKind}, terminal};
use ratatui::{
    buffer::Buffer,
    layout::{self, Constraint, Direction, Flex, Layout, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Bar, BarChart, BarGroup, Block, BorderType, Borders, Gauge, List, ListItem, ListState, Paragraph, Sparkline, Widget},
    DefaultTerminal, Frame,
};

pub enum TopError {
    OpenError,    // 打开文件失败
    ParseError,    // 数据转换类型失败
    ReadError,    // 读取失败
    NotFindError,    // 找不到某一事物
    EmptyError,    // 谋一数据为空
    MissingDependentData,    // 缺少某一依赖
    ErrorInformation(String),    // 万用，包含错误信息
}

enum DisplayModel {
    Top,
    FileManage,
}

pub struct App {
    user_input: String,
    input_history: Vec<String>,
    mooncell: Mooncell,
    model: DisplayModel,
    list_state: ListState,
}
impl App {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select_first();

        Self {
            user_input: String::new(),
            input_history: Vec::new(),
            mooncell: Mooncell::new(),
            model: DisplayModel::Top, 
            list_state: state,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let (tx_usage, rx_usage) = mpsc::channel();
        let (tx_temp, rx_temp) = mpsc::channel();
        let (tx_memory, rx_memory) = mpsc::channel();
        let (tx_power, rx_power) = mpsc::channel();

        Mooncell::refresh_cpu_usage(&mut self.mooncell, tx_usage);
        Mooncell::refresh_mem_usage(&mut self.mooncell, tx_memory);
        Mooncell::refresh_cpu_temp(&mut self.mooncell, tx_temp);
        Mooncell::refresh_cpu_power(&mut self.mooncell, tx_power);

        let mut count: u16 = 0;
    
        while self.mooncell.run {
            // 绘制&捕获事件
            terminal.draw(|frame| self.draw(frame))?;
            let _ = self.handle_events();
            
            match self.model {
                DisplayModel::FileManage => {
                    self.mooncell.refresh_file_tree();
                },
                DisplayModel::Top => {
                    // 刷新数据
                    if count == 10 {
                        count = 0;
                        self.mooncell.info.refresh_date();
                        self.mooncell.refresh_disk_list();
                        
                        self.mooncell.info.cpu_info.temp = rx_temp.recv().unwrap();
                        self.mooncell.info.cpu_info.power = rx_power.recv().unwrap();
                        
                        self.mooncell.info.cpu_info.usage = rx_usage.recv().unwrap();
                        match &self.mooncell.info.cpu_info.usage {
                            Ok(data) => {
                                self.mooncell.info.cpu_info.usage_history_push(data[0] as u64);
                            },
                            Err(_) => {},
                        };

                        self.mooncell.info.memory_info.usage = rx_memory.recv().unwrap();
                        match self.mooncell.info.memory_info.usage {
                            Ok(data) => {
                                self.mooncell.info.memory_info.usage_history_push(data as u64);
                            },
                            Err(_) => {},
                        }
                    } else {
                        count = count + 1;
                    }
                }
            }
        }
        Ok(())
    }

    // 绘制ui
    fn draw(&mut self, frame: &mut Frame) {
// ************************************** 根据工作模式绘制ui ************************************** //
        match self.model {
            DisplayModel::FileManage => {
                let tree_str_list = match self.mooncell.get_file_three_str() {
                    Ok(str_list) => str_list,
                    Err(top_error) => vec![Mooncell::toperror_to_string(&top_error)],
                };
                let file_tree_list = List::new(tree_str_list)
                    .block(Block::bordered().title("file manage"))
                    .highlight_style(
                        Style::default()
                            .bg(Color::LightBlue)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");
                let list_size = frame.area();
                frame.render_stateful_widget(file_tree_list, list_size, &mut self.list_state);
            }
            
            DisplayModel::Top => {
                let layout_top = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Min(8),
                        Constraint::Percentage(100),
                    ])
                    .split(frame.area());
                let logo_systeam = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![
                        Constraint::Length(50),
                        Constraint::Fill(2),
                    ])
                    .split(layout_top[0]);
                let cpu_memory = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![
                        Constraint::Fill(1),
                        Constraint::Fill(1),
                    ])
                    .split(layout_top[1]);

                let systeam_message = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(1),    // OS
                        Constraint::Length(1),    // 系统日期
                        Constraint::Length(1),    // IP address
                        Constraint::Length(1),    // CPU 型号
                        Constraint::Length(1),    // CPU temp & power & CPU(s)
                    ])
                    .split(logo_systeam[1]);

                let cpu_usage = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(5),    // cpu 总百分比占用
                        Constraint::Length(10),    // cpu 每核心占用
                        Constraint::Fill(1),
                    ])
                    .split(cpu_memory[0]);
                let memory_message = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(5),    // 内存占用百分比
                        Constraint::Length(10),    // 硬盘信息
                        Constraint::Fill(1),
                    ])
                    .split(cpu_memory[1]);
                let cpu_message = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![
                        Constraint::Percentage(33),
                        Constraint::Percentage(33),
                        Constraint::Percentage(33),
                    ])
                    .split(systeam_message[4]);

                // logo
                let logo_str = self.mooncell.logo.clone() + &"\nmoooncell version ".to_string() + &self.mooncell.version.clone();
                let logo_p = Paragraph::new(logo_str.clone())
                        .alignment(ratatui::layout::Alignment::Center)
                        .style(Style::default())
                        .fg(Color::Cyan)
                        .block(Block::bordered());
                frame.render_widget(logo_p, logo_systeam[0]);

                // 系统名称
                let os_name_p = Paragraph::new(self.mooncell.info.os_name.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(os_name_p, systeam_message[0]);
                
                // 日期
                let os_date_p = Paragraph::new(self.mooncell.info.sys_date.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(os_date_p, systeam_message[1]);

                // 本机ip
                let os_name_p = Paragraph::new(String::from("IP:") + &self.mooncell.info.ipv4.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(os_name_p, systeam_message[2]);

                // cpu名称
                let cpu_name_p = Paragraph::new(String::from("CPU:") + &self.mooncell.info.cpu_info.name.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(cpu_name_p, systeam_message[3]);

                // cpu温度、功耗、核心数
                let cpu_temp_str: String;
                match &self.mooncell.info.cpu_info.temp {
                    Ok(temp) => cpu_temp_str = String::from("temp: ") + &temp.to_string() + &String::from("C"),
                    Err(top_error) => cpu_temp_str = Mooncell::toperror_to_string(top_error),
                };
                let cpu_temp_p = Paragraph::new(cpu_temp_str.clone())
                    .alignment(ratatui::layout::Alignment::Right);

                let cpu_power_str: String;
                match &self.mooncell.info.cpu_info.power {
                    Ok(power) => cpu_power_str = String::from("power: ") + &power.to_string() + &String::from("W"),
                    Err(top_error) => cpu_power_str = Mooncell::toperror_to_string(top_error),
                }
                let cpu_power_p = Paragraph::new(cpu_power_str.clone())
                    .alignment(ratatui::layout::Alignment::Center);

                let cpu_cpu_siblings_str: String;
                match &self.mooncell.info.cpu_info.siblings {
                    Err(top_error) => cpu_cpu_siblings_str = Mooncell::toperror_to_string(&top_error),
                    Ok(siblings) => cpu_cpu_siblings_str = String::from("CPU(s): ") + &siblings.to_string(),
                };
                let cpu_siblings_p = Paragraph::new(cpu_cpu_siblings_str.clone())
                    .alignment(ratatui::layout::Alignment::Left);

                frame.render_widget(cpu_temp_p, cpu_message[0]);
                frame.render_widget(cpu_power_p, cpu_message[1]);
                frame.render_widget(cpu_siblings_p, cpu_message[2]);

                // cpu占用率
                match &self.mooncell.info.cpu_info.usage {
                    Ok(usage) => {
                        match usage.get(0) {
                            Some(_num) => {
                                let cpu_usage_s = Sparkline::default()
                                    .block(
                                        Block::new()
                                            .borders(Borders::ALL)
                                            .title("cpu usage"),
                                    )
                                    .max(100)
                                    .data(&self.mooncell.info.cpu_info.usage_history)
                                    .style(Style::default().fg(Color::Yellow));
                                frame.render_widget(cpu_usage_s, cpu_usage[0]);
                            },
                            None => {
                                let cpu_usage_error_str = String::from("what fuck this bug?");
                                let cpu_usage_error_p = Paragraph::new(cpu_usage_error_str)
                                    .alignment(ratatui::layout::Alignment::Left);
                                frame.render_widget(cpu_usage_error_p, cpu_usage[0]);
                            },
                        }
                    },
                    Err(top_error) => {
                        let cpu_usage_error_str = Mooncell::toperror_to_string(&top_error);
                        let cpu_usage_error_p = Paragraph::new(cpu_usage_error_str)
                            .alignment(ratatui::layout::Alignment::Left);
                        frame.render_widget(cpu_usage_error_p, cpu_usage[0]);
                    },
                }
                
                // cpu 核心占用率
                match &self.mooncell.info.cpu_info.usage {
                    Ok(usage) => {
                        let cpu_core_usage_str = Self::deal_cpu_usage(usage.clone());

                        let cpu_core_usage_p = Paragraph::new(cpu_core_usage_str.clone())
                            .alignment(layout::Alignment::Center);
                        frame.render_widget(cpu_core_usage_p, cpu_usage[1]);
                    },
                    Err(top_error) => {
                        let cpu_usage_error_str = Mooncell::toperror_to_string(&top_error);
                        let cpu_usage_error_p = Paragraph::new(cpu_usage_error_str)
                            .alignment(ratatui::layout::Alignment::Left);
                        frame.render_widget(cpu_usage_error_p, cpu_usage[1]);
                    },
                }

                // 内存占用率
                let mut memory_total_use:f64 = 0.0;
                let mut memory_usage_use:f64 = 0.0;

                match &self.mooncell.info.memory_info.total {
                    Err(top_error) => {
                        let mut memory_total_error_str = Mooncell::toperror_to_string(top_error);
                        memory_total_error_str = memory_total_error_str + &String::from("memory total");
                        let memory_total_error_p = Paragraph::new(memory_total_error_str)
                            .alignment(ratatui::layout::Alignment::Center);
                        frame.render_widget(memory_total_error_p, memory_message[0]);
                    },
                    Ok(memory_total) => {
                        memory_total_use = memory_total.clone();
                    },
                }
                match &self.mooncell.info.memory_info.usage {
                    Err(top_error) => {
                        let mut memory_usage_error_str = Mooncell::toperror_to_string(top_error);
                        memory_usage_error_str = memory_usage_error_str + &String::from("memory usage");
                        let memory_usage_error_p = Paragraph::new(memory_usage_error_str)
                            .alignment(ratatui::layout::Alignment::Left);
                        frame.render_widget(memory_usage_error_p, memory_message[0]);
                    },
                    Ok(memory_usage) => {
                        memory_usage_use = memory_usage.clone();
                    },
                }
                if memory_total_use != 0.0 && memory_usage_use != 0.0 {
                    let memory_usage_number_str = Self::float_to_string(memory_usage_use);
                    let memory_total_number_str = Self::float_to_string(memory_total_use);
                    let memory_usage_str = String::from("Memory: ") + &memory_total_number_str + &'/'.to_string() + &memory_usage_number_str + &"GB".to_string();

                    let memory_usage_s = Sparkline::default()
                        .block(
                            Block::new()
                                .borders(Borders::ALL)
                                .title(memory_usage_str),
                        )
                        .max(memory_total_use as u64)
                        .data(&self.mooncell.info.memory_info.usage_history)
                        .style(Style::default().fg(Color::Yellow));
                    frame.render_widget(memory_usage_s, memory_message[0]);
                }

                // 硬盘信息
                let disk_usage_list = self.mooncell.create_disk_list();
                let disk_barchart = BarChart::default()
                    .block(Block::default().title("disk infomation").borders(Borders::ALL))
                    .data(&disk_usage_list)
                    .bar_width(5)
                    .bar_gap(2)
                    .max(100)
                    .value_style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow))
                    .label_style(ratatui::style::Style::default().fg(ratatui::style::Color::Green));
                frame.render_widget(disk_barchart, memory_message[1]);
            },
        }
    }

    // 处理事件，如果属于键盘事件就调用handle_key_event
    fn handle_events(&mut self) -> io::Result<()> {
        match self.model {
            DisplayModel::Top => {
                if event::poll(Duration::from_millis(100))? {
                    match event::read()? {
                        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                            self.handle_key_event_top(key_event)
                        }
                        _ => {}
                    };
                };
            }
            DisplayModel::FileManage => {
                match event::read()? {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_key_event_fm(key_event)
                    }
                    _ => {}
                };
            }
        }
        Ok(())
    }

    // 处理键盘事件，属于目标按键时调用对应函数
    fn handle_key_event_top(&mut self, key_event: KeyEvent) {
        if !self.key_event_to_char(key_event) {
            match key_event.code {
                KeyCode::Backspace => {
                    self.user_input.pop();
                }
                KeyCode::Esc => {
                    self.exit();
                }
                KeyCode::Enter => {
                    self.mooncell.command_deal(self.user_input.clone());
                    self.input_history.push(self.user_input.clone());
                    self.user_input = String::new();
                }
                KeyCode::Tab => {
                    self.model = DisplayModel::FileManage;
                }
                _ => {}
            }
        }
    }
    
    fn handle_key_event_fm(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Esc => {
                self.exit();
            }
            KeyCode::Up => {
                self.file_list_previous();
            }
            KeyCode::Down => {
                self.file_list_next();
            }
            KeyCode::Tab => {
                self.model = DisplayModel::Top;
            }
            KeyCode::Enter => {
                if let Some(file_list_pos) = self.list_state.selected() {
                    if let Some(file_select) = self.mooncell.file_manage.file_tree.get(file_list_pos) {
                        self.mooncell.enter_file(&file_select.clone());
                    }
                }
            }
            KeyCode::Backspace => {
                self.mooncell.back_layer();
            }
            _ => {}
        }
    }

    fn key_event_to_char(&mut self, key_event: KeyEvent) -> bool{
        for i in ' '..'~' {
            if key_event.code == KeyCode::Char(i) {
                self.user_input += &i.to_string();
                return true;
            }
        }

        return false;
    }

    fn file_list_next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.mooncell.file_manage.file_tree.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn file_list_previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.mooncell.file_manage.file_tree.len().saturating_sub(1) {
                    0
                } else {
                    if i == 0 {
                        i
                    } else {
                        i - 1
                    }
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn exit(&mut self) {
        self.mooncell.exit();
    }

    fn deal_cpu_usage(core_usage_data: Vec<f64>) -> String {
        let mut siblings:u8 = 0;
        let mut str = String::new();
        if core_usage_data.is_empty() {
            return String::from("data is empty");
        }
        
        for usage in core_usage_data {
            if usage.is_nan() {
                return  String::from("Data Error");
            }

            if siblings != 0 {
                let core_number = siblings - 1;
                str.push_str(&("cpu".to_string() + &core_number.to_string()));
                if core_number < 10 {
                    str.push_str("  :");
                } else if core_number < 100{
                    str.push_str(" :");
                } else {
                    str.push_str(":");
                }

                let usage_str:String;
                let tmp_usage_str = &mut usage.to_string();
                match tmp_usage_str.find(".") {
                    Some(pos) => {
                        let number_len = tmp_usage_str.len() - (pos + 1);
                        if number_len >= 2 {
                            usage_str = tmp_usage_str[0..pos+3].to_string();
                        } else {
                            for _i in 0..(2 - number_len) {
                                tmp_usage_str.push('0');
                            }
                            usage_str = tmp_usage_str[0..pos+3].to_string();
                        }
                    },
                    None => {
                        usage_str = tmp_usage_str.clone() + &String::from(".00");
                    },
                }
                if usage > 9.9 && usage < 100.0 {
                    str.push_str(" ");
                } else if usage < 10.0 {
                    str.push_str("  ");
                }

                str.push_str(&usage_str);

                if core_number % 2 != 0 {
                    str.push('\n');
                } else {
                    str.push_str("    ");
                }
            }
            siblings += 1;
        }
        return str;
    }

    fn float_to_string(value: f64) -> String {
        let str = value.to_string();
        match str.find('.') {
            None => return str.clone(),
            Some(pos) => str[..pos+2].to_string(),
        }
    }
}

