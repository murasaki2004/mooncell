use std::fs::File;
use std::io::{self, prelude::*, BufRead};
use std::path::Path;
use std::result::Result::Ok;
use std::net::{UdpSocket, IpAddr};
use chrono::{DateTime, Local};
use std::process::Command;

use crate::app::TopError;

pub struct Info {
    pub os_name: String,
    pub sys_date: String,
    pub cpu_info: CPU,
    pub memory_info: Memory,
    pub ipv4: String,
}


pub struct CPU {
    pub name: String,    // 名称
    pub usage: Result<Vec<f64>, TopError>,    // 0:总的占用率，剩下的为每个核心的占用率
    pub usage_history: Vec<u64>,    // CPU总占用率历史记录，只保留50条记录
    pub temp: Result<f64, TopError>,    // 温度，安全版
    pub power: Result<f64, TopError>,    // 功耗，安全版
    pub siblings: Result<u8, TopError>,    // 核心数，安全版
}

pub struct Memory {
    // 单位GB，usage表示已使用的内存量
    pub total: Result<f64, TopError>,
    pub usage: Result<f64, TopError>,
    // 内存占用率历史记录，只保留50条记录
    pub usage_history: Vec<u64>,
}

pub struct CpuStatData {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub guest: u64,
    pub guest_nice: u64,

    user_isok: bool,
    nice_isok: bool,
    system_isok: bool,
    idle_isok: bool,
    iowait_isok: bool,
    irq_isok: bool,
    softirq_isok: bool,
    steal_isok: bool,
    guest_isok: bool,
    guest_nice_isok: bool,
}


impl Info {
    pub fn new() -> Self {
        let ip_str = match Info::get_loacl_ipadder() {
            Some(ip) => ip.to_string(),
            None => String::from("can`t find"),
        };

        let local: DateTime<Local> = Local::now();

        Self {
            sys_date: local.format("%Y-%m-%d %H:%M").to_string(),
            os_name: Info::get_os_name(),
            cpu_info: CPU::new(),
            memory_info: Memory::new(),
            ipv4: ip_str,
        }
    }

    /*
    * @概述      读取/proc/version文件获取系统名称(版本)，仅限linux
    * @返回值    String
    */
    pub fn get_os_name() -> String {
        let path = Path::new("/proc/version");
        let mut file = match File::open(&path) {
            Err(why) => return why.to_string(),
            Ok(file) => file,
        };
        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(why) => return why.to_string(),
            Ok(_) => {
                match s.find("(") {
                    None => return s,
                    Some(t) => return s[..t].to_string(),
                }
            },
        }
    }

    /*
     * @概述      刷新日期时间
     * @返回值    String
     */
    pub fn refresh_date(&mut self) {
        let local: DateTime<Local> = Local::now();
        self.sys_date = local.format("%Y-%m-%d %H:%M").to_string();
    }

    /*
     * @概述      获取本机ipv4地址
     * @返回值    Option<IpAddr>
     */
    fn get_loacl_ipadder() -> Option<IpAddr> {
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:80").ok()?;

        let local_addr = socket.local_addr().ok()?;
        Some(local_addr.ip())
    }
}

impl CPU {
    pub fn new() -> Self {
        let siblings_tmp = CPU::get_cpu_siblings_safe();
        let power_tmp = CPU::get_cpu_power();
        let temp_safe_tmp = CPU::get_cpu_temp_safe();
        let name_tmp = CPU::get_cpu_name();
        let usage_tmp: Result<Vec<f64>, TopError>;
        match siblings_tmp {
            Err(_) => usage_tmp = Err(TopError::MissingDependentData),
            Ok(num) => usage_tmp = CPU::cpu_usage_init(num),
        }

        Self {
            usage_history: Vec::new(),
            power: power_tmp,
            siblings: siblings_tmp,
            temp: temp_safe_tmp,
            name: name_tmp,
            usage: usage_tmp,
        }
    }

    /*
     * @概述      读取/proc/cpuinfo文件获取cpu名称
     * @返回值    String
     */
    pub fn get_cpu_name() -> String {
        if let Ok(lines) = read_lines("/proc/cpuinfo") {
            for line in lines {
                if let Ok(str) = line {
                    if str.find("model name") != None {
                        match str.find(": ") {
                            None => return String::from("not find"),
                            Some(t2) => return str[t2+2..].to_string(),
                        }
                    }
                }
            }
        }
        String::from("not find")
    }

    /*
     * @概述      返回一个全零的Result<f64, TopError>容器，用于初始化usage
     * @参数1     核心数
     * @返回值    f64容器，size是siblings+1
     */
    fn cpu_usage_init(siblings: u8) -> Result<Vec<f64>, TopError> {
        let mut f64_vec: Vec<f64> = Vec::new();
        for _i in 0..siblings+1 {
            f64_vec.push(0.0);
        }
        return Ok(f64_vec);
    }

    /*
     * @概述      向usage_history中添加数据，满50条数据后会顶掉前面的
     * @参数1     data
     */
    pub fn usage_history_push(&mut self, data: u64) {
        if self.usage_history.len() < 50 {
            self.usage_history.push(data);
        } else {
            self.usage_history.remove(0);
            self.usage_history.push(data);
        }
    }

    /*
    * @概述      读取/proc/stat文件获取cpu信息
    * @返回值    Result<Vec<CpuStatData>, TopError>
    */
    pub fn get_new_cpustat() -> Result<Vec<CpuStatData>, TopError> {
        let file: File;
        match File::open("/proc/stat") {
            Ok(file_read) => file = file_read,
            Err(_) => return Err(TopError::OpenError),
        }
        let mut read_data_all: Vec<CpuStatData> = Vec::new();
        let mut read_data: CpuStatData = CpuStatData::new();

        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            let str: String;
            match line {
                Ok(str_line) => str = str_line,
                Err(_) => return Err(TopError::ReadError),
            }
            
            if str.contains("cpu") {
                let parts: Vec<&str> = str.split_whitespace().collect();
                    for part in &parts[1..] {
                    match part.parse::<u64>() {
                        Ok(num) => read_data.enter_data(num),
                        Err(_) => return Err(TopError::ParseError),
                    }
                }
                read_data_all.push(read_data.clone());
                read_data.clear();
            }
        }
        return Ok(read_data_all);
    }

    /*
     * @概述      根据cpustat的数据计算每个核心的占用率
     * @返回值    Result<Vec<f64>, TopError>
     */
    pub fn calculate_usage(now_stat: &[CpuStatData], last_stat: &[CpuStatData]) -> Result<Vec<f64>, TopError> {
        if now_stat.is_empty() {
            return Err(TopError::EmptyError);
        } else if last_stat.is_empty() {
            return Err(TopError::EmptyError);
        } else {
            let mut return_usage: Vec<f64> = Vec::new();
            let len = last_stat.len();
            for i in 0..len {
                let deal_data_now: &CpuStatData;
                let deal_data_last: &CpuStatData;

                match last_stat.get(i as usize) {
                    None => return Err(TopError::EmptyError),
                    Some(data) => deal_data_last = data,
                }
                match now_stat.get(i as usize) {
                    None => return Err(TopError::EmptyError),
                    Some(data) => deal_data_now = data,
                }

                let prev_idle = deal_data_last.idle + deal_data_last.iowait;
                let prev_nonidle = deal_data_last.user + deal_data_last.nice + deal_data_last.system + deal_data_last.irq + deal_data_last.softirq + deal_data_last.steal;
                let prev_total = prev_idle + prev_nonidle;
                let prev_idle2 = deal_data_now.idle + deal_data_now.iowait;

                let prev_nonidle2 = deal_data_now.user + deal_data_now.nice + deal_data_now.system + deal_data_now.irq + deal_data_now.softirq + deal_data_now.steal;
                let prev_total2 = prev_idle2 + prev_nonidle2;
                let totald = prev_total2 - prev_total;
                let idled = prev_idle2 - prev_idle;

                let usage = ((totald - idled) as f64 / totald as f64) * 100.0;

                return_usage.push(usage);
            }
            return Ok(return_usage);
        }
    }

    /*
     * @概述      读取文件数据返回cpu温度信息
     * @返回值    Result<f64, TopError>
     */
    pub fn get_cpu_temp_safe() -> Result<f64, TopError> {
        let path = Path::new("/sys/class/thermal/thermal_zone0/temp");
        let mut file = match File::open(path) {
            Err(_) => return Err(TopError::OpenError),
            Ok(file) => file,
        };

        let mut str = String::new();
        match file.read_to_string(&mut str) {
            Err(_) => Err(TopError::ReadError),
            Ok(_) => {
                match str.trim().parse::<u32>() {
                    Ok(num) => {
                        let temp = num as f64 / 1000.0;
                        return Ok(temp);
                    },
                    Err(_) => return Err(TopError::ParseError),
                }
            },
        }
    }

    /*
    * @概述      读取文件数据返回cpu核心数信息
    * @返回值    Result<u8, TopError>
    */
    pub fn get_cpu_siblings_safe() -> Result<u8, TopError> {
        let lines = match read_lines("/proc/cpuinfo") {
            Ok(lines) => lines,
            Err(_) => return Err(TopError::OpenError),
        };

        for line in lines {
            match line {
                Err(_) => {
                    return Err(TopError::ReadError)
                },
                Ok(str) => {
                    if str.contains("siblings") {
                        match str.find(": ") {
                            None => return Err(TopError::NotFindError),
                            Some(t) => {
                                let siblings_str = str[t+2..].to_string().clone();
                                match siblings_str.trim().parse::<u8>() {
                                    Ok(num) => return Ok(num),
                                    Err(_) => return Err(TopError::ParseError),
                                }
                            },
                        }
                    }
                },
            };
        };
        return Err(TopError::NotFindError);
    }

    /*
     * @概述      调用upower读取功耗信息
     * @返回值    Result<f64, TopError>
     */
    pub fn get_cpu_power() -> Result<f64, TopError> {
        // 执行 upower --dump 命令
        let output = Command::new("upower")
            .arg("--dump")
            .output()
            .map_err(|e| TopError::ErrorInformation(e.to_string()))?;

        // 检查命令是否成功执行
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TopError::ErrorInformation(format!("upower command failed: {}", stderr)));
        }

        // 将命令输出转换为 UTF-8 字符串
        let output_str = String::from_utf8(output.stdout)
            .map_err(|_| TopError::ParseError)?;

        // 如果输出为空，返回 EmptyError
        if output_str.is_empty() {
            return Err(TopError::EmptyError);
        }

        // 寻找包含 energy-rate 的行
        if let Some(line) = output_str.lines().find(|l| l.contains("energy-rate:")) {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 2 {
                return Err(TopError::ParseError);
            }

            // 提取数值部分
            let value_part = parts[1].trim();
            let value_str = value_part
                .split_whitespace()
                .next()
                .ok_or(TopError::ParseError)?;

            // 解析为 f64
            let value: f64 = value_str
                .parse()
                .map_err(|_| TopError::ParseError)?;

            Ok(value)
        } else {
            Err(TopError::NotFindError)
        }
    }
}

impl Memory {
    pub fn new() -> Self {
        Self {
            total: Memory::get_memory_total(),
            usage: Memory::get_memory_usage_safe(),
            usage_history: Vec::new(),
        }
    }

    /*
     * @概述      读取meminfo文件获得内存总量
     * @返回值    Result<f64, TopError>
     */
    pub fn get_memory_total() -> Result<f64, TopError> {
        let path = Path::new("/proc/meminfo");
        let mut file = match File::open(&path) {
            Err(_) => return Err(TopError::OpenError),
            Ok(file) => file,
        };
        let mut s = String::new();
        match file.read_to_string(&mut s) {
            Err(_) => return Err(TopError::OpenError),
            Ok(_) => {
                match s.find("MemTotal:") {
                    None => return Err(TopError::EmptyError),
                    Some(start_pos) => {
                        match s.find(" k") {
                            None => return Err(TopError::EmptyError),
                            Some(over_pos) => {
                                let total_str = s[start_pos+9..over_pos].to_string().clone();
                                match total_str.trim().parse::<u32>() {
                                    Err(_) => return Err(TopError::ParseError),
                                    Ok(num) => {
                                        return Ok(num as f64 / 1000.0 / 1000.0);
                                    }
                                }
                            }
                        }
                    }
                }
            },
        }
    }

    /*
     * @概述      读取meminfo文件获得当前内存占用量
     * @返回值    Result<f64, TopError>
     */
    pub fn get_memory_usage_safe() -> Result<f64, TopError> {
        let mut memory_total: u64 = 0;
        let mut memory_free: u64 = 0;

        if let Ok(lines) = read_lines("/proc/meminfo") {
            for line in lines {
                if let Ok(str) = line {
                    if str.contains("MemTotal:") {
                        match str.find(":") {
                            None => return Err(TopError::EmptyError),
                            Some(start_pos) => {
                                match str.find("k") {
                                    None => return Err(TopError::EmptyError),
                                    Some(over_pos) => {
                                        let str_read = str[start_pos+1..over_pos].to_string().clone();
                                        match str_read.trim().parse::<u64>() {
                                            Err(_) => memory_total = 0,
                                            Ok(num) => memory_total = num,
                                        }
                                    }
                                }
                            }
                        }
                    } else if str.contains("MemFree:") || str.contains("Buffers:") || str.contains("Cached:") {
                        match str.find(":") {
                            None => return Err(TopError::EmptyError),
                            Some(start_pos) => {
                                let value: u64;
                                match str.find("k") {
                                    None => return Err(TopError::EmptyError),
                                    Some(over_pos) => {
                                        let str_read = str[start_pos+1..over_pos].to_string().clone();
                                        match str_read.trim().parse::<u64>() {
                                            Err(_) => value = 0,
                                            Ok(num) => value = num,
                                        }
                                        memory_free += value;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            return Err(TopError::OpenError);
        }

        let memory_use = (memory_total - memory_free) as f64 / 1024.0 / 1024.0;
        return Ok(memory_use);
    }

    /*
     * @概述      向usage_history中添加数据，满50条数据后会顶掉前面的
     * @参数1     data
     */
    pub fn usage_history_push(&mut self, data: u64) {
        if self.usage_history.len() < 50 {
            self.usage_history.push(data);
        } else {
            self.usage_history.remove(0);
            self.usage_history.push(data);
        }
    }
}

impl CpuStatData {
    pub fn new() -> Self {
        Self {
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
            user_isok: false,
            nice_isok: false,
            system_isok: false,
            idle_isok: false,
            iowait_isok: false,
            irq_isok: false,
            softirq_isok: false,
            steal_isok: false,
            guest_isok: false,
            guest_nice_isok: false, 
        }
    }

    /*
     * @概述      向CpuStatData结构中输入数据
     * 如果结构中数据已满，值会被忽略
     */
    pub fn enter_data(&mut self, value: u64) {
        if self.user_isok == false {
            self.user = value;
            self.user_isok = true;
        } else if self.nice_isok == false {
            self.nice = value;
            self.nice_isok = true;
        } else if self.system_isok == false {
            self.system = value;
            self.system_isok = true;
        } else if self.idle_isok == false {
            self.idle = value;
            self.idle_isok = true;
        } else if self.iowait_isok == false {
            self.iowait = value;
            self.iowait_isok = true;
        } else if self.irq_isok == false {
            self.irq = value;
            self.irq_isok = true;
        } else if self.softirq_isok == false {
            self.softirq = value;
            self.softirq_isok = true;
        } else if self.steal_isok == false {
            self.steal = value;
            self.steal_isok = true;
        } else if self.guest_isok == false {
            self.guest = value;
            self.guest_isok = true;
        } else if self.guest_nice_isok == false {
            self.guest_nice = value;
            self.guest_nice_isok = true;
        }
    }
    
    pub fn clone(&self) -> Self {
        Self {
            user: self.user.clone(),
            nice: self.nice.clone(),
            system: self.system.clone(),
            idle: self.idle.clone(),
            iowait: self.iowait.clone(),
            irq: self.irq.clone(),
            softirq: self.softirq.clone(),
            steal: self.steal.clone(),
            guest: self.guest.clone(),
            guest_nice: self.guest_nice.clone(), 
            
            user_isok: self.user_isok.clone(),
            nice_isok: self.nice_isok.clone(),
            system_isok: self.system_isok.clone(),
            idle_isok: self.idle_isok.clone(),
            iowait_isok: self.iowait_isok.clone(),
            irq_isok: self.irq_isok.clone(),
            softirq_isok: self.softirq_isok.clone(),
            steal_isok: self.steal_isok.clone(),
            guest_isok: self.guest_isok.clone(),
            guest_nice_isok: self.guest_nice_isok.clone(),
        }
    }

    pub fn clear(&mut self) {
        self.user = 0;
        self.nice = 0;
        self.system = 0;
        self.idle = 0;
        self.iowait = 0;
        self.irq = 0;
        self.softirq = 0;
        self.steal = 0;
        self.guest = 0;
        self.guest_nice = 0; 

        self.user_isok = false;
        self.nice_isok = false;
        self.system_isok = false;
        self.idle_isok = false;
        self.iowait_isok = false;
        self.irq_isok = false;
        self.softirq_isok = false;
        self.steal_isok = false;
        self.guest_isok = false;
        self.guest_nice_isok = false; 
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
