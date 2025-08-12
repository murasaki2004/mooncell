use std::process::Command;
use std::path::{Path, PathBuf};
use encoding_rs::GBK;

use super::TopError;

pub enum FileType {
    Normal,
    Markdown,
    Code,
    Document,
    Audio,
    Video,
    Image,
    Zip,
    Folder,
}

pub enum FileOperation {
    Null,
    Copy,
    Move,
    Delete,
}

pub struct FileUnit {
    pub name: String,
    pub file_type: FileType,
    pub occupy: f64,
    pub path: PathBuf,
}

pub struct FileManage {
    now_path: PathBuf,    // 当前处理的路径
    file_list: Result<Vec<FileUnit>, TopError>,    // 文件列表
    select_list: Vec<FileUnit>,    // 选中的文件列表
    wait_operation_list: Vec<FileUnit>,    // 等待操作的文件列表
    file_operation: FileOperation,     // 准备进行的文件操作
}

impl FileUnit {
    pub fn new() -> Self {
        Self { name: String::new(), file_type: FileType::Normal, occupy: 0.0, path: PathBuf::new()}
    }

    pub fn is_equal(file_a: FileUnit, file_b: FileUnit) -> bool {
        if file_a.path.exists() && file_b.path.exists() {
            if file_a.path != file_b.path {
                return false
            }
        } else {
            return false
        }
        true
    }
}
impl Clone for FileUnit {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            path: self.path.clone(),
            occupy: self.occupy.clone(),
            file_type: match self.file_type {
                FileType::Zip => FileType::Zip,
                FileType::Code => FileType::Code,
                FileType::Audio => FileType::Audio,
                FileType::Video => FileType::Video,
                FileType::Image => FileType::Image,
                FileType::Folder => FileType::Folder,
                FileType::Normal => FileType::Normal,
                FileType::Markdown => FileType::Markdown,
                FileType::Document => FileType::Document,
            }
        }
    }
}

impl FileManage {
    pub fn new() -> Self {
        Self {
            now_path: match Self::get_pwd() {
                Ok(str) => PathBuf::from(str),
                Err(_) => {
                    #[cfg(target_os = "linux")]{
                        PathBuf::from("/home")
                    }
                    #[cfg(target_os = "windows")]{
                        PathBuf::from("C:\\")
                    }
                }
            },
            file_list: Ok(Vec::new()),
            select_list: Vec::new(),
            wait_operation_list: Vec::new(),
            file_operation: FileOperation::Null,
        }
    }

    pub fn select_push(&mut self, file: FileUnit) {
        for select_file in &self.select_list {
            if FileUnit::is_equal(select_file.clone(), file.clone()) {
                return;
            }
        }
        self.select_list.push(file);
    }

    pub fn clear_select(&mut self) {
        self.select_list.clear();
    }

    /*
     * @概述        准备操作select_list
     * @参数1      FileOperation
     */
    pub fn select_ready_operation(&mut self, operate: FileOperation) {
        self.read_select_operate();
        self.file_operation = operate;
    }

    /*
     * @概述      调用ls -l命令读取path参数路径下的内容，处理成FileUnit写入self.file_list
     * @返回值    Option<TopError>，仅在错误时返回
     */
    pub fn refresh_file_list(&mut self) -> Option<TopError> {
        let path_str = match self.get_path_str() {
            Some(str) => str.to_string(),
            None => return Some(TopError::ParseError),
        };
        if Self::is_path(&path_str) == false {
            return Some(TopError::OpenError);
        }

        let output_str: String;
        #[cfg(target_os = "linux")] {
            let output = match Command::new("ls").arg("-l").arg(path_str).output() {
                Ok(out) => out,
                Err(e) => return Some(TopError::ErrorInformation(e.to_string())),
            };
            output_str = match String::from_utf8(output.stdout) {
                Ok(str) => str,
                Err(e) => return Some(TopError::ErrorInformation(e.to_string())),
            };
        }
        #[cfg(target_os = "windows")] {
            let output = match Command::new("powershell").arg("ls").arg(path_str).output() {
                Ok(out) => out,
                Err(e) => return Some(TopError::ErrorInformation(e.to_string())),
            };

            let encoding = GBK;
            let (cow, _, _) = encoding.decode(&output.stdout);
            output_str = cow.into_owned();
        }

        let mut file_list: Vec<FileUnit> = Vec::new();
        for line in output_str.lines() {
            let data:Vec<&str> = line.split_whitespace().collect();
            // 跳过格式不完整的行
            #[cfg(target_os = "linux")] {
                if data.len() < 8 {
                    continue;
                }
            }
            #[cfg(target_os = "windows")] {
                if data.len() < 3 {
                    continue;
                }
                match data.get(0) {
                    Some(str) => {
                        if *str == "----" {
                            continue;
                        }
                    }
                    _ => continue,
                }
            }
            
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

            if let Some(str_size) = data.get(4) {
                match str_size.trim().parse::<u64>() {
                    Ok(num) => {
                        now_file_unit.occupy = num as f64 / 1024.0;
                    },
                    Err(_) => {},
                }
            }

            let skip: i8;
            #[cfg(target_os = "linux")] {
                skip = 8;
            }
            #[cfg(target_os = "windows")] {
                match now_file_unit.file_type {
                    FileType::Folder => skip = 3,
                    _ => skip = 4,
                }
            }
            let name = data.iter().skip(skip as usize).cloned().collect::<Vec<_>>().join(" ");
            now_file_unit.name = name;
            now_file_unit.path = self.now_path.clone();
            now_file_unit.path.push(now_file_unit.name.clone());
                if let FileType::Normal = now_file_unit.file_type {
                    if let Some(suffix) = Self::get_file_name_suffix(now_file_unit.name.clone()) {
                        match suffix.as_str() {
                            "txt" | "doc" | "docx" => now_file_unit.file_type = FileType::Document,
                            "mp4" => now_file_unit.file_type = FileType::Video,
                            "mp3" | "wav" => now_file_unit.file_type = FileType::Audio,
                            "zip" | "7z" | "rar" => now_file_unit.file_type = FileType::Zip,
                            "md" => now_file_unit.file_type = FileType::Markdown,
                            "png" | "jpg" | "jpeg" => now_file_unit.file_type = FileType::Image,
                            "rs" | "c" | "py" | "cpp" | "h" => now_file_unit.file_type = FileType::Code,
                            _ => {}
                        }
                    }
                }
            file_list.push(now_file_unit);
        }
        // 文件夹为空的情况
        if file_list.is_empty() {
            return Some(TopError::EmptyError)
        }
        self.file_list = Ok(file_list);
        return None;
    }

    pub fn get_file_list(&self) -> Option<Vec<FileUnit>> {
        return match &self.file_list {
            Ok(vec) => Some(vec.clone()),
            Err(_) => None,
        }
    }

    pub  fn select_operate(&mut self) -> Option<TopError> {
        if self.wait_operation_list.is_empty() {
            return  Some(TopError::EmptyError)
        }
        let mut path_str = String::new();
        for file in &self.wait_operation_list {
            path_str.push_str(&Self::pathbuf_to_string(file.path.clone()));
        }
        if let Some(target_path_str) = self.get_path_str() {
            match self.file_operation {
                FileOperation::Copy => {
                    match  Command::new("cp").arg("-r").arg(path_str).arg(target_path_str).output() {
                        Ok(out) => out,
                        Err(e) => return Some(TopError::ErrorInformation(e.to_string())),
                    };
                },
                FileOperation::Move => {
                    match  Command::new("mv").arg("-r").arg(path_str).arg(target_path_str).output() {
                        Ok(out) => out,
                        Err(e) => return Some(TopError::ErrorInformation(e.to_string())),
                    };
                },
                FileOperation::Delete => {
                    todo!()
                },
                _ => {},
            }
        }
        None
    }

    /*
     * @概述       清空select_list，读进wait_operation_list
     */
    fn read_select_operate(&mut self) {
        self.wait_operation_list = self.select_list.clone();
        self.clear_select();
    }

    /*
     * @概述      处理内部PathBuf回到上一层文件夹
     */
    pub fn back_upper_layer(&mut self) -> bool {
        self.now_path.pop()
    }

    /*
     * @概述      处理PathBuf使进入到参数的文件夹
     * @参数1     String，路径字符串
     * @参数2     String，需要进入的路径的字符串
     * @返回值    bool
     */
    pub fn enter_new_folder(&mut self, folder: &FileUnit) -> bool {
        match folder.file_type {
            FileType::Folder => {
                self.now_path.push(folder.name.clone());
                if self.now_path.exists() {
                    return true;
                } else {
                    self.now_path.pop();
                    return false;
                }
            }
            _ => {
                return false;
            }
        }
    }

    /*
     * @概述       处理内部file_list创建String容器，类型为文件夹的会特殊标记
     * @返回值    Vec<String>
     */
    pub fn create_name_list(&self) -> Vec<String> {
        let mut return_vec: Vec<String> = Vec::new();
        match &self.file_list {
            Ok(vec) => {
                for deal_unit in vec.iter() {
                    match deal_unit.file_type {
                        FileType::Folder => {
                            return_vec.push(format!("[{}]", deal_unit.name));
                        },
                        _ => {
                            return_vec.push(deal_unit.name.clone());
                        },
                    }
                }
                return return_vec
            }
            Err(error) => {
                let return_vec = vec![error.to_string()];
                return return_vec
            }
        }
    }

    pub fn get_select(&self) -> Vec<FileUnit> {
        if self.wait_operation_list.is_empty() {
            self.select_list.clone()
        } else {
            self.wait_operation_list.clone()
        }
    }

    pub fn get_path_str(&self) -> Option<&str> {
        self.now_path.to_str().map(|e|e)
    }

    /*
     * @概述       判断字符串是否为路径
     * @参数1     &str
     * @返回值    bool
     */
    fn is_path(s: &str) -> bool {
        let tmp = Path::new(s);
        tmp.exists()
    }

    /*
     * @概述         忽略错误返回的将pathbuf转换成string
     * @参数1      PathBuf
     * @返回值    string
     */
    fn pathbuf_to_string(path: PathBuf) -> String {
        if let Some(str) = path.to_str() {
            return str.to_string();
        }
        String::new()
    }

    /*
     * @概述         调用`pwd命令读取当前文件路径
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

    fn get_file_name_suffix(filename: String) -> Option<String> {
        Path::new(&filename)
            .extension()  // 获取后缀部分（不包括点）
            .and_then(|ext| ext.to_str())  // 转换为字符串
            .filter(|s| !s.is_empty())  // 过滤掉空字符串
            .map(|s| s.to_string())  // 转换为 String
    }
}

