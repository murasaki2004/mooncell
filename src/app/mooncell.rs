use std::sync::{mpsc, Arc};
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::result::Result::Ok;

use super::TopError;

mod info;
use info::{CpuInfo, MemoryInfo, Info};

mod filemanage;
use filemanage::{FileType, FileUnit, FileManage, FileOperation};



pub struct Mooncell {
    pub logo: String,
    pub version: String,
    pub run: bool,
    pub thread_run: Arc<AtomicBool>,
    pub info: Info,
    pub file_manage: FileManage,
}


impl Mooncell {
    pub fn new() -> Self {
        Self {
            run: true,
            info: Info::new(),
            file_manage: FileManage::new(),
            version: String::from("test-v_0.1.0 "),
            thread_run: Arc::new(AtomicBool::new(true)),
            logo: String::from("    __  ___                  ______     ____\n   /  |/  /___  ____  ____  / ____/__  / / /\n  / /|_/ / __ \\/ __ \\/ __ \\/ /   / _ \\/ / / \n / /  / / /_/ / /_/ / / / / /___/  __/ / /  \n/_/  /_/\\____/\\____/_/ /_/\\____/\\___/_/_/   "),
        }
    }

/**********************************************文件管理**********************************************/
    pub fn enter_file(&mut self, file: &FileUnit) {
        match file.file_type {
            FileType::Folder => {
                let _ = self.file_manage.enter_new_folder(file);
            },
            _ => { },
        }
    }

    pub fn fm_copy_ready(&mut self) {
        self.file_manage.select_ready_operation(FileOperation::Copy);
    }

    pub fn fm_move_ready(&mut self) {
        self.file_manage.select_ready_operation(FileOperation::Move);
    }

    pub fn fm_perform_operations(&mut self) {
        self.file_manage.select_operate();
    }

    pub fn clear_select(&mut self) {
        self.file_manage.clear_select();
    }

    pub fn create_select_str(&self) -> String {
        let mut str = String::new();
        for file in self.file_manage.get_select() {
            str.push_str(&format!("\n{}", file.name).to_string());
        }
        str
    }

/**********************************************其他函数**********************************************/
    /*
    * @概述      将toperror转化成string
    * @参数1     TopError
    * @返回值    String
    */
    pub fn toperror_to_string(top_error: &TopError) -> String {
        match top_error {
            TopError::EmptyError => return String::from("data lost"),
            TopError::NotFindError => return String::from("not find"),
            TopError::OpenError => return String::from("can`t open"),
            TopError::ParseError => return String::from("can`t parse"),
            TopError::ErrorInformation(str) => return str.clone(),
            TopError::ReadError => return String::from("can`t read file"),
            TopError::MissingDependentData => return String::from("Missing dependent data"),
        }
    }

    /*
    * @概述      将FIleType转化成string的提示
    * @参数1     TopError
    * @返回值    String
    */
    pub fn filetype_to_string(file_type: &FileType) -> String {
        match file_type {
            FileType::Zip => return String::from("Zip File"),
            FileType::Normal => return String::from("File"),
            FileType::Folder => return String::from("Folder"),
            FileType::Video => return String::from("Video File"),
            FileType::Audio => return String::from("Audio File"),
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

    /*
     * @概述      将一个f32容器按一定格式处理成String
     * @参数1    Vec<f32>
     */
    pub fn deal_cpu_usage(core_usage_data: Vec<f32>) -> String {
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

    /*
     * @概述        将f32仅保留后两位小数转换成String
     * @返回值    String
     */
    pub fn float_to_string(value: f32) -> String {
        let str = value.to_string();
        match str.find('.') {
            None => return str.clone(),
            Some(pos) => str[..pos+2].to_string(),
        }
    }
}
