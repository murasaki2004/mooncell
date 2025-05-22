use std::io;
mod app;
use app::App;

/*对于tui程序，有三个主要步骤
 * 初始化终端
 * 循环运行应用，直到退出
 * 恢复终端至原始状态 
 */
fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::new().run(&mut terminal);
    ratatui::restore();
    app_result
}

