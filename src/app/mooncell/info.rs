use std::fs::File;
use std::io::Read;
use sysinfo::Disks;
use std::path::Path;
use sysinfo::System;
use std::process::Command;
use chrono::{DateTime, Local};
use std::net::{UdpSocket, IpAddr};

use super::TopError; 

pub struct Info {
    sys: System,
    pub ipv4: String,
    pub date: String,
    pub os_name: String,
    pub cpu_info: CpuInfo,
    pub host_name: String,
    pub disks: Vec<DiskInfo>,
    pub mem_info: MemoryInfo,
}

pub struct CpuInfo {
    pub temp: f32,    // 温度
    pub power: f32,     // 功耗
    pub name: String,    // 名称
    pub siblings: u8,    // 核心数
    pub usage: Vec<f32>,     // 0:总的占用率，剩下的为每个核心的占用率
    pub usage_history: Vec<u64>,    // CPU总占用率历史记录，只保留50条记录
}

pub struct MemoryInfo {
    // 单位GB，usage表示已使用的内存量
    pub total: f32,
    pub usage: f32,
    // 内存占用率历史记录，只保留50条记录
    pub usage_history: Vec<u64>,
}

pub struct DiskInfo {
    pub name: String,
    pub all_space: f64,    // 总空间
    pub available_space: f64,    // 可用空间
}

impl Clone for DiskInfo {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            all_space: self.all_space.clone(),
            available_space: self.available_space.clone()
        }
    }
}

impl Info {
    pub fn new() -> Self {
        let os_name: String;
        let sys = System::new();
        if let Some(name) = sysinfo::System::name() {
            os_name = name;
        } else {
            os_name = "unkown".to_string();
        }
        
        let ip_str = match Info::get_loacl_ipadder() {
            Some(ip) => ip.to_string(),
            None => String::from("can`t find"),
        };

        let local: DateTime<Local> = Local::now();
        let sys_date = local.format("%Y-%m-%d %H:%M").to_string();

        let host_name = match System::host_name() {
            Some(str) => str,
            None => "unkown".to_string(),
        };

        Self {
            sys: sys,
            ipv4: ip_str,
            date: sys_date,
            disks: Vec::new(),
            os_name: os_name,
            host_name: host_name,
            cpu_info: CpuInfo::new(),
            mem_info: MemoryInfo::new(),
        }
    }

    /*
     * @概述        刷新日期时间
     * @返回值      String
     */
    pub fn refresh_date(&mut self) {
        let local: DateTime<Local> = Local::now();
        self.date = local.format("%Y-%m-%d %H:%M").to_string();
    }

    /*
     * @概述        获取本机ipv4地址
     * @返回值      Option<IpAddr>
     */
    pub fn get_loacl_ipadder() -> Option<IpAddr> {
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("8.8.8.8:80").ok()?;

        let local_addr = socket.local_addr().ok()?;
        Some(local_addr.ip())
    }

    /* 
     * @概述        通过sysinfo查询系统下的所有硬盘
     */
    pub fn refresh_disks(&mut self) {
        self.disks.clear();
        let disks = Disks::new_with_refreshed_list();

        for disk in disks.list() {
            let mut data = DiskInfo::new();

            // 获取磁盘名称（挂载点）
            match disk.name().to_str() {
                Some(str) => data.name = str.to_string(),
                None => continue,
            };
            // 获取总容量（字节）
            let total = disk.total_space();
            // 获取可用空间（字节）
            let available = disk.available_space();

            // 转换为 GB 单位
            data.all_space = total as f64 / (1024.0 * 1024.0 * 1024.0);
            data.available_space = available as f64 / (1024.0 * 1024.0 * 1024.0);

            self.disks.push(data);
        }
    }

    /*
     * @概述        刷新内存数据
     */
    pub fn refresh_memory_data(&mut self) {
        self.sys.refresh_memory();

        let total = self.sys.total_memory() as f32;
        self.mem_info.total =  total  / (1024.0 * 1024.0 * 1024.0);
        let usage = self.sys.total_memory() - self.sys.available_memory();
        self.mem_info.usage = usage as f32 / (1024.0 * 1024.0 * 1024.0);

        self.mem_info.usage_history_push();
    }

    /*
     * @概述        刷新cpu数据
     */
    pub fn refresh_cpu_data(&mut self) {
        self.sys.refresh_cpu_all();

        // cpu 占用率
        self.cpu_info.usage.clear();
        self.cpu_info.usage.push(self.sys.global_cpu_usage());
        for cpu in self.sys.cpus() {
            self.cpu_info.usage.push(cpu.cpu_usage());
        }
        self.cpu_info.usage_history_push();

        self.cpu_info.refresh_temp();
        self.cpu_info.refresh_power();
    }
}

impl CpuInfo {
    fn new() -> Self {
        let sys = System::new_all();
        let mut count: u8  = 0;
        let mut name_str = String::new();
        for cpu in sys.cpus() {
            name_str = cpu.brand().to_string();
            count = count + 1;
        }

        Self {
            temp: 0.0,
            power: 0.0,
            siblings: count,
            name: name_str,
            usage: Vec::new(),
            usage_history: Vec::new(),
        }
    }

    /*
     * @概述        读取文件数据返回cpu温度信息
     * @返回值      Result<f64, TopError>
     */
    fn refresh_temp(&mut self) {
        // 开文件
        let path = Path::new("/sys/class/thermal/thermal_zone0/temp");
        let mut file = match File::open(path) {
            Err(_) => return,
            Ok(file) => file,
        };

        // 读取并转f32
        let mut str = String::new();
        match file.read_to_string(&mut str) {
            Err(_) => { },
            Ok(_) => {
                match str.trim().parse::<u32>() {
                    Ok(num) => {
                        self.temp = num as f32 / 1000.0;
                    },
                    Err(_) => return,
                }
            },
        }
    }

    /*
     * @概述        调用upower读取功耗信息
     * @返回值      Result<f64, TopError>
     * @碎碎念      真抽象，因为去掉返回值直接赋值self.power,导致整了一堆嵌套，烦死了
     */
    fn refresh_power(&mut self) {
        // 执行 upower --dump 命令
        if let Ok(output) = Command::new("upower").arg("--dump").output() {
            // 检查命令是否成功执行
            if !output.status.success() { return; }

            // 将命令输出转换为 UTF-8 字符串
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                // 寻找包含 energy-rate 的行
                if let Some(line) = output_str.lines().find(|l| l.contains("energy-rate:")) {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() < 2 { return; }

                    // 提取数值部分
                    let value_part = parts[1].trim();
                    if let Some(value_str) = value_part.split_whitespace().next() {
                        self.power = match value_str.parse() {
                            Ok(num) => num,
                            Err(_) => return,
                        };
                    }
                }
            }
        }
    }

    /*
     * @概述        向usage_history中添加数据，满50条数据后会顶掉前面的
     */
    fn usage_history_push(&mut self) {
        if let Some(data) = self.usage.get(0) {
            if *data != 0.0 {
                self.usage_history.push(*data as u64);
            }
        }
    }
}

impl MemoryInfo {
    fn new() -> Self {
        Self {
            usage_history: Vec::new(),
            total: 0.0,
            usage: 0.0,
        }
    }

    /*
     * @概述        向usage_history中添加数据，满50条数据后会顶掉前面的
     */
    pub fn usage_history_push(&mut self) {
        if self.usage_history.len() < 50 {
            self.usage_history.push(self.usage as u64);
        } else {
            self.usage_history.remove(0);
            self.usage_history.push(self.usage as u64);
        }
    }
}

impl DiskInfo {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            all_space: 0.0,
            available_space: 0.0,
        }
    }
}