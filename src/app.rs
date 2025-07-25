use core::{error, num};
use std::fmt::format;
use std::fs::{FileTimes, FileType};
use std::isize;
use std::{clone, default, io, process::exit};
use std::sync::mpsc;
use std::time::{Duration, Instant};
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
    mooncell: Mooncell,
    model: DisplayModel,

    input_history: Vec<String>,    // 显示cpu占用历史
    list_state: ListState,    // 文件管理列表的转中状态
    file_manage_tips: String,    // 用于显示文件管理状态的提示
    disk_show_list: Vec<String>,    // 临时存放disk显示字符串的容器
    last_enter_time: Option<Instant>,
}
impl App {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select_first();

        Self {
            list_state: state,
            last_enter_time: None,
            user_input: String::new(),
            input_history: Vec::new(),
            mooncell: Mooncell::new(),
            disk_show_list: Vec::new(),
            model: DisplayModel::Top, 
            file_manage_tips: String::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut count: u16 = 0;
    
        while self.mooncell.run {
            // 绘制&捕获事件
            terminal.draw(|frame| self.draw(frame))?;
            let _ = self.handle_events();
            
            match self.model {
                DisplayModel::FileManage => {
                    self.mooncell.file_manage.refresh_file_list();
                },
                DisplayModel::Top => {
                    // 刷新数据
                    if count == 10 {
                        count = 0;
                        self.mooncell.info.refresh_all();
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
            // ************************** 文件管理模式 ************************** //
            DisplayModel::FileManage => {
                let layout_all = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Fill(1),
                        Constraint::Length(2),
                    ])
                    .split(frame.area());
                let layout_filemanage = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![
                        Constraint::Percentage(70),
                        Constraint::Fill(1),
                    ])
                    .split(layout_all[0]);
                let file_message = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Length(1),    // 分割
                        Constraint::Length(1),    // 名字
                        Constraint::Length(1),    // 类型
                        Constraint::Length(1),    // 占用空间
                        Constraint::Length(2),    // 分割
                        Constraint::Fill(0),    // 选中的文件名
                        Constraint::Length(1),    // 进行文件操作的提示
                    ])
                    .split(layout_filemanage[1]);
                
                // tips
                let tips_str = String::from("switch to top[tab]    exit[esc]\r\nReturn to the previous directory[backspace]    Enter folder[enter]");
                let tips_p = Paragraph::new(tips_str.clone())
                        .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(tips_p, layout_all[1]);

                // 文件列表
                let tree_str_list = self.mooncell.file_manage.create_name_list();
                let path_str = match self.mooncell.file_manage.get_path_str() {
                    Some(str) => str.to_string(),
                    None => "...".to_string(),
                };
                let file_tree_list = List::new(tree_str_list)
                    .block(Block::bordered().title(path_str))
                    .highlight_style(
                        Style::default()
                            .bg(Color::LightBlue)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");
                frame.render_stateful_widget(file_tree_list, layout_filemanage[0], &mut self.list_state);
                
                // 文件名、文件类型、文件大小
                let mut str_file_name = String::new();
                let mut str_file_type = String::new();
                let mut str_file_size = String::new();
                if let Some(file_list_pos) = self.list_state.selected() {
                    if let Some(file_select) = self.mooncell.file_manage.get_file_list().get(file_list_pos) {
                        str_file_name = file_select.name.clone();
                        str_file_type = Mooncell::filetype_to_string(&file_select.file_type);
                        if file_select.occupy < 1024.0 {
                            str_file_size = Mooncell::float_to_string(file_select.occupy as f32) + &"KB".to_string();
                        } else if file_select.occupy < 1048576.0 {
                            str_file_size = Mooncell::float_to_string((file_select.occupy / 1024.0) as f32) + &"MB".to_string();
                        } else {
                            str_file_size = Mooncell::float_to_string((file_select.occupy / 1024.0 / 1024.0) as f32) + &"GB".to_string();
                        }
                    }
                } else {
                    str_file_name = String::from("get name error");
                    str_file_type = String::from("get type error");
                    str_file_size = String::from("get file occupy error");
                }
                let file_name_p = Paragraph::new(str_file_name.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(file_name_p, file_message[1]);

                let file_type_p = Paragraph::new(str_file_type.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(file_type_p, file_message[2]);
                
                let file_size_p = Paragraph::new(str_file_size.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(file_size_p, file_message[3]);

                // 选中文件信息
                let select_file_name_p = Paragraph::new(self.mooncell.create_select_str())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(select_file_name_p, file_message[5]);

                // 提示str
                let tips_p = Paragraph::new(self.file_manage_tips.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(tips_p, file_message[6]);
            }
            
            // ************************** 资源管理模式 ************************** //
            DisplayModel::Top => {
                let layout_top = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(vec![
                        Constraint::Min(8),
                        Constraint::Percentage(100),
                        Constraint::Length(1),
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
                        Constraint::Length(1),    // 空闲
                        Constraint::Length(1),    // OS@host name
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
                    .split(systeam_message[5]);

                // logo
                let logo_str = format!("{}\nmoooncell version {}", self.mooncell.logo, self.mooncell.version);
                let logo_p = Paragraph::new(logo_str.clone())
                        .alignment(ratatui::layout::Alignment::Center)
                        .style(Style::default())
                        .fg(Color::Cyan)
                        .block(Block::bordered());
                frame.render_widget(logo_p, logo_systeam[0]);

                // tips
                let tips_str = String::from("switch to filemanage[tab]    exit[esc]");
                let tips_p = Paragraph::new(tips_str.clone())
                        .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(tips_p, layout_top[2]);

                // 系统名称+host名称
                let os_name_p = Paragraph::new(format!("{}@{}", self.mooncell.info.os_name, self.mooncell.info.host_name))
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(os_name_p, systeam_message[1]);
                
                // 日期
                let os_date_p = Paragraph::new(self.mooncell.info.date.clone())
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(os_date_p, systeam_message[2]);

                // 本机ip
                let os_name_p = Paragraph::new(format!("IP:{}", self.mooncell.info.ipv4))
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(os_name_p, systeam_message[3]);

                // cpu名称
                let cpu_name_p = Paragraph::new(format!("CPU:{}", self.mooncell.info.cpu_info.name))
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(cpu_name_p, systeam_message[4]);

                // cpu温度、功耗、核心数
                let cpu_temp_str = format!("temp: {}C", self.mooncell.info.cpu_info.temp);
                let cpu_temp_p = Paragraph::new(cpu_temp_str.clone())
                    .alignment(ratatui::layout::Alignment::Right);

                let cpu_power_str = format!("power: {}W", self.mooncell.info.cpu_info.power);
                let cpu_power_p = Paragraph::new(cpu_power_str.clone())
                    .alignment(ratatui::layout::Alignment::Center);

                let cpu_cpu_siblings_str = format!("cpu(s): {}", self.mooncell.info.cpu_info.siblings);
                let cpu_siblings_p = Paragraph::new(cpu_cpu_siblings_str.clone())
                    .alignment(ratatui::layout::Alignment::Left);

                frame.render_widget(cpu_temp_p, cpu_message[0]);
                frame.render_widget(cpu_power_p, cpu_message[1]);
                frame.render_widget(cpu_siblings_p, cpu_message[2]);

                // cpu占用率
                let cpu_usage_s = Sparkline::default()
                    .block(
                        Block::new().borders(Borders::ALL).title("cpu global usage"),
                    )
                    .max(100)
                    .data(&self.mooncell.info.cpu_info.usage_history)
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(cpu_usage_s, cpu_usage[0]);

                // 核心占用率
                let cpu_core_usage_str = Mooncell::deal_cpu_usage(self.mooncell.info.cpu_info.usage.clone());
                let cpu_core_usage_p = Paragraph::new(format!("\n{}", cpu_core_usage_str))
                    .block(
                        Block::new().borders(Borders::ALL).title("cpu core usage"),
                    )
                    .alignment(layout::Alignment::Center);
                frame.render_widget(cpu_core_usage_p, cpu_usage[1]);

                // 内存占用率
                let memory_usage_number_str = Mooncell::float_to_string(self.mooncell.info.mem_info.usage);
                let memory_total_number_str = Mooncell::float_to_string(self.mooncell.info.mem_info.total);
                let memory_usage_str = format!("Memory: {}/{}GB", memory_total_number_str, memory_usage_number_str);

                let memory_usage_s = Sparkline::default()
                    .block(
                        Block::new()
                            .borders(Borders::ALL)
                            .title(memory_usage_str),
                    )
                    .max(self.mooncell.info.mem_info.total as u64)
                    .data(&self.mooncell.info.mem_info.usage_history)
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(memory_usage_s, memory_message[0]);

                // 硬盘信息
                let disk_usage_list = self.create_disk_list();
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
                            self.handle_key_event_top(key_event);
                        }
                        _ => {}
                    };
                };
            }
            DisplayModel::FileManage => {
                match event::read()? {
                    Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                        self.handle_key_event_fm(key_event);
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
            KeyCode::Esc => self.exit(),
            KeyCode::Up => self.file_list_previous(),
            KeyCode::Down => self.file_list_next(),
            KeyCode::Tab => {
                self.model = DisplayModel::Top;
                self.file_manage_tips.clear();
            }
            KeyCode::Backspace => { let _ = self.mooncell.file_manage.back_upper_layer(); }
            /*使用一个列表存储选中的文件，改变目录后清除
             * 回车第一下选中，双击——进入文件夹、预览文件(待开发)
             * c—准备复制、x—准备剪切、v—执行
             */
            KeyCode::Enter => {
                let now = Instant::now();

                if let Some(last_time) = self.last_enter_time {
                    if now.duration_since(last_time) <= Duration::from_millis(300) {
                        // 双击 Enter
                        if let Some(pos) = self.list_state.selected() {
                            if let Some(file) = self.mooncell.file_manage.get_file_list().get(pos) {
                                self.mooncell.enter_file(&file.clone());
                                self.mooncell.clear_select();
                            }
                        }
                    } else {
                        // 单击 Enter
                        if let Some(pos) = self.list_state.selected() {
                            if let Some(file) = self.mooncell.file_manage.get_file_list().get(pos) {
                                self.mooncell.file_manage.select_push(file.clone());
                            }
                        }
                    }
                } else {
                    // 第一次按 Enter
                    if let Some(pos) = self.list_state.selected() {
                        if let Some(file) = self.mooncell.file_manage.get_file_list().get(pos) {
                            self.mooncell.file_manage.select_push(file.clone());
                        }
                    }
                }

                // 更新最后按 Enter 的时间
                self.last_enter_time = Some(now);
            }
            KeyCode::Char('c') => self.mooncell.fm_copy_ready(),
            KeyCode::Char('x') => self.mooncell.fm_move_ready(),
            KeyCode::Char('v') => self.mooncell.fm_perform_operations(),
            
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
                if i >= self.mooncell.file_manage.get_file_list().len().saturating_sub(1) {
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
                if i > self.mooncell.file_manage.get_file_list().len().saturating_sub(1) {
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

    /*
     * @概述        根据内部的disks创建(&str, u64)的容器
     * @返回值    Vec<(&str, u64)>
     */
    pub fn create_disk_list(&mut self) -> Vec<(&str, u64)> {
        self.disk_show_list.clear();
        let mut usage_list: Vec<(&str, u64)> = Vec::new();

        for disk in &self.mooncell.info.disks {
            let usage = if disk.all_space == 0.0 {
                0
            } else {
                let used = disk.all_space - disk.available_space;
                ((used * 100.0) / disk.all_space).min(100.0) as u64
            };
            let tmp_data = (disk.name.as_str(), usage);
            usage_list.push(tmp_data.clone());
        }
        return usage_list;
    }
}

