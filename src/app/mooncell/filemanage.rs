use sysinfo::{Disk, DiskExt, System, SystemExt};
use std::process::Command;
use std::path::Path;

use crate::app::TopError;

pub enum FileType {
    Normal,
    Folder,
}

pub struct DiskInfo {
    pub name: String,
    pub all_space: f64,    // 总空间
    pub available_space: f64,    // 可用空间
}

pub struct FileUnit {
    pub name: String,
    pub file_type: FileType,
}

pub struct FileManage {
    pub file_tree: Vec<FileUnit>,
    pub select: FileUnit,
    pub now_path: String,
    pub sys_disks: Vec<DiskInfo>,
}

impl FileUnit {
    pub fn new() -> Self {
        Self { name: String::new(), file_type: FileType::Normal}
    }

    pub fn clone(&self) -> Self {
        Self { 
            name: self.name.clone(),
            file_type: match self.file_type {
                FileType::Folder => FileType::Folder,
                FileType::Normal => FileType::Normal,
            }
        }
    }

    pub fn clear(&mut self) {
        self.name.clear();
        self.file_type = FileType::Normal;
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

    pub fn clone(&mut self) -> Self {
        Self {
            name: self.name.clone(),
            all_space: self.all_space.clone(),
            available_space: self.available_space.clone(),
        }
    }

    pub fn copy(&mut self) -> Self {
        Self {
            name: self.name.clone(),
            all_space: self.all_space.clone(),
            available_space: self.available_space.clone(),
        }
    }
}

impl FileManage {
    pub fn new() -> Self {
        Self{
            file_tree: Vec::new(), 
            select: FileUnit::new(), 
            now_path: String::new(),
            sys_disks: Vec::new(),
        }
    }

    /* 
     * @概述      通过sysinfo查询系统下的所有硬盘
     */
    pub fn refresh_disks(&mut self) {
        let sys = System::new_all();
        for disk in sys.disks() {
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

            self.sys_disks.push(data);
        }
    }

    /* 
     * @概述      通过now_path查询当前路径下的所有文件，并集合到file_tree中
     *            如果当前路劲有问题就调用pwd获取路径，否则直接调用get_file_tree
     */
    pub fn refresh_file_tree(&mut self) {
        if !Self::is_path(&self.now_path.clone()) {
            self.now_path = match Self::get_pwd() {
                Ok(str) => str,
                Err(_) => String::from("/"),
            }
        }
        self.file_tree =  match Self::get_file_three(self.now_path.clone()) {
            Ok(file_tree) => file_tree,
            Err(_) => Vec::new(), 
        }
    }

    /* 
     * @概述      选中某一文件
     * @参数1     FileUinit
     */
    pub fn select_some_file(&mut self, file: &FileUnit) {
        self.select = file.clone();
    }

    /*
     * @概述      处理FileUnit容器创建标记出文件类型的String容器
     * @参数1     FileUinit容器
     * @返回值    Result<Vec<String>, TopError>
     */
    pub fn create_name_list(&self) -> Result<Vec<String>, TopError> {
        let mut return_vec: Vec<String> = Vec::new();
        if self.file_tree.is_empty() {
            return Err(TopError::EmptyError);
        } else {
            for deal_unit in self.file_tree.iter() {
                match deal_unit.file_type {
                    FileType::Folder => {
                        return_vec.push("[".to_string() + &deal_unit.name.clone() + &"]".to_string());
                    },
                    FileType::Normal => {
                        return_vec.push(deal_unit.name.clone());
                    },
                }
            }
            return Ok(return_vec);
        }
    }

    /*
     * @概述      处理路径String使回到上一层文件夹
     * @参数1     String，路径字符串
     * @返回值    Result<String, TopError>
     */
    pub fn back_upper_layer(&mut self) -> Result<String, TopError> {
        if Self::is_path(&self.now_path) {
            if let Some(pos) = self.now_path.rfind('/') {
                let new_path = self.now_path[0..pos].to_string().clone();
                if Self::is_path(&new_path) {
                    self.now_path = new_path.clone();
                    self.refresh_file_tree();
                    return Ok(self.now_path.clone());
                } else {
                    return Err(TopError::EmptyError);
                }
            } else {
                return Err(TopError::NotFindError);
            }
        } else {
            return Err(TopError::ErrorInformation("this not path".to_string()));
        }
    }

    /*
     * @概述      处理路径String使进入到参数path的文件夹
     * @参数1     String，路径字符串
     * @参数2     String，需要进入的路径的字符串
     * @返回值    Result<String, TopError>
     */
    pub fn enter_new_folder(&mut self, folder: &FileUnit) -> Result<String, TopError> {
        match folder.file_type {
            FileType::Normal => {
                return Err(TopError::ErrorInformation("not folder".to_string()));
            },

            FileType::Folder => {
                if !folder.name.is_empty() && Self::is_path(&self.now_path) {
                    let new_path = self.now_path.clone() + &String::from("/") + &folder.name.clone();
                    self.now_path = new_path.clone();
                    self.file_tree.clear();
                    self.refresh_file_tree();
                    return Ok(new_path);
                } else {
                    return Err(TopError::ErrorInformation("this not path".to_string()));
                }
            },
        }
    }

    /*
     * @概述      判断字符串是否为路径
     * @参数1     &str
     * @返回值    bool
     */
    pub fn is_path(s: &str) -> bool {
        let tmp = Path::new(s);
        tmp.exists()
    }

    /*
     * @概述      调用pwd命令读取当前文件路径
     * @返回值    Result<String, TopError>
     */
    fn get_pwd() -> Result<String, TopError> {
        let output = Command::new("pwd")
            .output()
            .map_err(|e| TopError::ErrorInformation(e.to_string()))?;

        if output.status.success() {
            let str = String::from_utf8(output.stdout).map_err(|_| TopError::ParseError)?;
            let return_str = str.clone()[0..str.len()-1].to_string();
            return Ok(return_str);
        } else {
            return Err(TopError::ErrorInformation("command execute error".to_string()));
        }
    }

    /*
     * @概述      调用ls -l命令读取path参数路径下的内容
     * @参数1     String，路径字符串
     * @返回值    Result<String, TopError>
     */
    fn get_file_three(path: String) -> Result<Vec<FileUnit>, TopError> {
        let output = Command::new("ls")
            .arg("-l")
            .arg(path.clone())
            .output()
            .map_err(|e| TopError::ErrorInformation(e.to_string()))?;

        let output_str = String::from_utf8(output.stdout)
            .map_err(|_| TopError::ParseError)?;

        // 如果输出为空，返回 EmptyError
        if output_str.is_empty() {
            return Err(TopError::EmptyError);
        } else {
            let mut file_tree: Vec<FileUnit> = Vec::new();
            for line in output_str.lines() {
                let mut now_file_unit = FileUnit::new();
                if let Some(str) = line.chars().next() {
                    if str == '-' {
                        now_file_unit.file_type = FileType::Normal;
                    } else if str == 'd' {
                        now_file_unit.file_type = FileType::Folder;
                    } else {
                        continue;
                    }
                }

                let data:Vec<&str> = line.split_whitespace().collect();
                if let Some(str) = data.get(8) {
                    now_file_unit.name = str.to_string();
                    file_tree.push(now_file_unit);
                }
            }
            return Ok(file_tree);
        }
    }
}