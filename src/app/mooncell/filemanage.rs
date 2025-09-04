use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use std::ffi::OsString;

use super::TopError;

/*
 * @概述        FileManage是整个功能的集合体
 *              FileUnit用来表示单个文件/文件夹
 *              FileType和FileOperation都仅起到辅助作用

 *              整体逻辑：
 *              self(指代filemanager)的file_list用来存储now_path下的内容，需要调用refresh_file_list刷新
 *              如果需要对文件操作需要先通过select_push将待操作的fileunit加入self的select_list
 *              因为复制/剪切需要切换目录，所以又将select_list读进wait_operation_list等待粘贴
 *              最终由select_operate来按照self.file_operation执行操作
 */

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

    /*
     * @概述        向select_file容器添加新文件
     * @参数1       FileUnit
     */
    pub fn select_push(&mut self, file: FileUnit) {
        for select_file in &self.select_list {
            if FileUnit::is_equal(select_file.clone(), file.clone()) {
                return;
            }
        }
        self.select_list.push(file);
    }

    /*
     * @概述        清理选中文件
     */
    pub fn clear_select(&mut self) {
        self.select_list.clear();
    }

    /*
     * @概述        准备操作select_list
     * @参数1       FileOperation
     */
    pub fn select_ready_operation(&mut self, operate: FileOperation) {
        self.read_select_operate();
        self.file_operation = operate;
    }

    /*
     * @概述        调用fs::read_dir读取self.now_path路径下的内容，处理成FileUnit写入self.file_list
     * @返回值      Option<TopError>，仅在错误时返回
     */
    pub fn refresh_file_list(&mut self) -> Option<TopError> {
        if let Some(path_str) = self.get_path_str() {
            match fs::read_dir(&path_str) {
                Ok(entries) => {
                    let mut file_list: Vec<FileUnit> = Vec::new();
                    for entry in entries {
                        let mut fileunit = FileUnit::new();
                        match entry {
                            Ok(entry) => {
                                fileunit.name = Self::osstring_to_string(entry.file_name());

                                // 匹配文件类型
                                let path = entry.path();
                                fileunit.path = entry.path();
                                if path.is_dir() {
                                    fileunit.file_type = FileType::Folder;
                                } else {
                                    if let Some(suffix) = Self::get_file_name_suffix(fileunit.name.clone()) {
                                        match suffix.as_str() {
                                            "txt" | "doc" | "docx" => fileunit.file_type = FileType::Document,
                                            "mp4" => fileunit.file_type = FileType::Video,
                                            "mp3" | "wav" => fileunit.file_type = FileType::Audio,
                                            "zip" | "7z" | "rar" => fileunit.file_type = FileType::Zip,
                                            "md" => fileunit.file_type = FileType::Markdown,
                                            "png" | "jpg" | "jpeg" => fileunit.file_type = FileType::Image,
                                            "rs" | "c" | "py" | "cpp" | "h" => fileunit.file_type = FileType::Code,
                                            _ => {}
                                        }
                                    }
                                }
                                file_list.push(fileunit);
                            }
                            Err(_) => break
                        }
                    }
                    if file_list.is_empty() {
                        return Some(TopError::EmptyError)
                    }
                    self.file_list = Ok(file_list);
                    return None;
                }
                Err(_) => return Some(TopError::ReadError)
            }
        } else {
            return Some(TopError::OpenError);
        }
    }

    /*
     * @概述        获取当前的file_list
     * @返回值      Option<Vec<FileUnit>>
     */
    pub fn get_file_list(&self) -> Option<Vec<FileUnit>> {
        return match &self.file_list {
            Ok(vec) => Some(vec.clone()),
            Err(_) => None,
        }
    }

    /*
     * @概述        按照file_operation对wait_operation_list执行操作
     * @返回值      Option<TopError>
     */
    pub fn select_operate(&mut self) -> Option<TopError> {
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
     * @概述        清空select_list，读进wait_operation_list
     */
    fn read_select_operate(&mut self) {
        self.wait_operation_list = self.select_list.clone();
        self.clear_select();
    }

    /*
     * @概述        处理内部PathBuf回到上一层文件夹
     */
    pub fn back_upper_layer(&mut self) -> bool {
        self.now_path.pop()
    }

    /*
     * @概述        处理PathBuf使进入到参数的文件夹
     * @参数1       String，路径字符串
     * @参数2       String，需要进入的路径的字符串
     * @返回值      bool
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
     * @概述        返回选中的文件列表
     * @返回值      Vec<FileUnit>
     */
    pub fn get_select(&self) -> Vec<FileUnit> {
        if self.wait_operation_list.is_empty() {
            self.select_list.clone()
        } else {
            self.wait_operation_list.clone()
        }
    }

    /*
     * @概述        返回当前路径的&str
     * @返回值      Option<&str>
     */
    pub fn get_path_str(&self) -> Option<&str> {
        self.now_path.to_str().map(|e|e)
    }

    /*
     * @概述        忽略错误返回的将pathbuf转换成string
     * @参数1       PathBuf
     * @返回值      string
     */
    fn pathbuf_to_string(path: PathBuf) -> String {
        if let Some(str) = path.to_str() {
            return str.to_string();
        }
        String::new()
    }

    /*
     * @概述        调用pwd命令读取当前文件路径
     * @返回值      Result<String, TopError>
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
     * @概述        获取文件名的后缀
     * @返回值      Option<String>
     */
    fn get_file_name_suffix(filename: String) -> Option<String> {
        Path::new(&filename)
            .extension()  // 获取后缀部分（不包括点）
            .and_then(|ext| ext.to_str())  // 转换为字符串
            .filter(|s| !s.is_empty())  // 过滤掉空字符串
            .map(|s| s.to_string())  // 转换为 String
    }

    /*
     * @概述        将OsString强制转换成String
     * @参数1       str: OsString
     * @返回值      String
     */
    fn osstring_to_string(str: OsString) -> String {
        match str.to_str() {
            Some(new_str) => return new_str.to_string(),
            None => return String::from("parse fail"),
        }
    }
}

