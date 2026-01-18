use std::process::Command;

pub fn start() {
    println!("🚀 正在启动 sing-box...");
    execute_systemctl("start");
}

pub fn stop() {
    println!("🛑 正在停止 sing-box...");
    execute_systemctl("stop");
}

pub fn restart() {
    println!("🔄 正在重启 sing-box...");
    execute_systemctl("restart");
}

// 提取一个私有辅助函数，减少重复代码
fn execute_systemctl(action: &str) {
    let status = Command::new("systemctl") // 确保有双引号
        .args([action, "sing-box"])
        .status();
    
    match status {
        Ok(s) if s.success() => println!("=> {} 成功", action),
        Ok(s) => eprintln!("=> {} 失败，退出码: {}", action, s),
        Err(e) => eprintln!("=> 无法执行 systemctl: {}", e),
    }
}