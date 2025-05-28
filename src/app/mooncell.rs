use std::thread;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::result::Result::Ok;

use super::TopError;

mod info;
use info::{CPU, Memory, Info, CpuStatData};

mod filemanage;
use filemanage::{FileType, FileUnit, FileManage};



pub struct Mooncell {
    pub logo: String,
    pub version: String,
    pub run: bool,
    pub thread_run: Arc<AtomicBool>,
    pub info: Info,
    pub file_manage: FileManage,
    pub thread_sleep_time: u16,
}


impl Mooncell {
    pub fn new() -> Self {
        Self {
            run: true,
            info: Info::new(),
            thread_sleep_time: 1000,
            file_manage: FileManage::new(),
            version: String::from("test-v_0.0.4"),
            thread_run: Arc::new(AtomicBool::new(true)),
            logo: String::from("    __  ___                  ______     ____\n   /  |/  /___  ____  ____  / ____/__  / / /\n  / /|_/ / __ \\/ __ \\/ __ \\/ /   / _ \\/ / / \n / /  / / /_/ / /_/ / / / / /___/  __/ / /  \n/_/  /_/\\____/\\____/_/ /_/\\____/\\___/_/_/   "),
        }
    }

/**********************************************文件管理**********************************************/
    pub fn refresh_file_tree(&mut self) {
        self.file_manage.refresh_file_tree();
    }

    pub fn refresh_disk_list(&mut self) {
        self.file_manage.refresh_disks();
    }

    pub fn get_file_three_str(&self) -> Result<Vec<String>, TopError> {
        self.file_manage.create_name_list()
    }

    pub fn enter_file(&mut self, file: &FileUnit) {
        match file.file_type {
            FileType::Folder => {
                let _ = self.file_manage.enter_new_folder(file);
            },
            _ => {
                self.file_manage.select_some_file(file);
            },
        }
    }

    pub fn create_disk_list(&mut self) -> Vec<(&str, u64)> {
        let mut usage_list: Vec<(&str, u64)> = Vec::new();
        for disk in &self.file_manage.sys_disks {
            let usage: u64 = (disk.all_space / disk.available_space) as u64;
            let tmp_data = (disk.name.as_str(), usage);
            usage_list.push(tmp_data);
        }
        return usage_list;
    }

/**********************************************资源监控**********************************************/
    pub fn refresh_cpu_usage(&mut self, tx: mpsc::Sender<Result<Vec<f64>, TopError>>) {
        let sleep_time = self.thread_sleep_time.clone() as u64;

        // 刷新usage
        thread::spawn(move|| {
            let mut last_cpu_stat: Vec<CpuStatData> = Vec::new();
            let mut now_cpu_stat: Vec<CpuStatData>;
            
            loop {
                if last_cpu_stat.is_empty() {
                    last_cpu_stat = match CPU::get_new_cpustat() {
                        Ok(cpu_stat) => cpu_stat,
                        Err(top_error) => {
                            let _ = tx.send(Err(top_error));
                            continue;
                        },
                    };
                    let _ = tx.send(Err(TopError::EmptyError));
                } else {
                    now_cpu_stat = match CPU::get_new_cpustat() {
                        Ok(cpu_stat) => cpu_stat,
                        Err(top_error) => {
                            let _ = tx.send(Err(top_error));
                            continue;
                        }
                    };

                    let usage_data = CPU::calculate_usage(&now_cpu_stat, &last_cpu_stat);
                    let _ = tx.send(usage_data);

                    last_cpu_stat = now_cpu_stat;
                }

                // wait
                thread::sleep(Duration::from_millis(sleep_time));
            }
        });
    }

    pub fn refresh_cpu_temp(&mut self, tx: mpsc::Sender<Result<f64, TopError>>) {
        let sleep_time = self.thread_sleep_time.clone() as u64;

        thread::spawn(move|| {
            loop {
                let temp_data = CPU::get_cpu_temp_safe();
                let _ = tx.send(temp_data);

                thread::sleep(Duration::from_millis(sleep_time));
            }
        });
    }

    pub fn refresh_cpu_power(&mut self, tx: mpsc::Sender<Result<f64, TopError>>) {
        let sleep_time = self.thread_sleep_time.clone() as u64;

        thread::spawn(move|| {
            loop {
                let power_data = CPU::get_cpu_power();
                let _ = tx.send(power_data);

                thread::sleep(Duration::from_millis(sleep_time));
            }
        });
    }

    pub fn refresh_mem_usage(&mut self, tx: mpsc::Sender<Result<f64, TopError>>) {
        let sleep_time = self.thread_sleep_time.clone() as u64;

        thread::spawn(move|| {
            loop {
                let temp_data = Memory::get_memory_usage_safe();
                let _ = tx.send(temp_data);

                thread::sleep(Duration::from_millis(sleep_time));
            }
        });
    }

/**********************************************其他函数**********************************************/
    /*
    * @概述      将toperror转化成string的提示，用于UI绘制中如果数据返回TopError的情况
    * @参数1     TopError
    * @返回值    String
    */
    pub fn toperror_to_string(top_error: &TopError) -> String {
        match top_error {
            TopError::MissingDependentData => return String::from("Missing dependent data"),
            TopError::EmptyError => return String::from("data lost"),
            TopError::NotFindError => return String::from("not find"),
            TopError::OpenError => return String::from("can`t open"),
            TopError::ParseError => return String::from("can`t parse"),
            TopError::ReadError => return String::from("can`t read file"),
            TopError::ErrorInformation(str) => return str.clone(),
        }
    }

    /*
    * @概述      将FIleType转化成string的提示
    * @参数1     TopError
    * @返回值    String
    */
    pub fn filetype_to_string(file_type: &FileType) -> String {
        match file_type {
            FileType::Normal => return String::from("File"),
            FileType::Zip => return String::from("Zip File"),
            FileType::Folder => return String::from("Folder"),
            FileType::Audio => return String::from("Audio File"),
            FileType::Video => return String::from("Video File"),
            FileType::Image => return String::from("Image File"),
            FileType::Code => return String::from("Code source"),
            FileType::Markdown => return String::from("Markdown"),
            FileType::Document => return String::from("Document File"),
        }
    }
    
    /*
    * @概述      处理一部分指令
    * @参数1     String
    */
    pub fn command_deal(&mut self, command: String) {
        if command.eq("exit") {
            self.run = false;
        } else if command.eq("stop") {
            self.thread_run.store(false, Ordering::Release);
        }
    }
    
    
    /*
    * @概述      退出
    */
    pub fn exit(&mut self) {
        self.thread_run.store(false, Ordering::Relaxed);
        self.run = false;
    }
}
