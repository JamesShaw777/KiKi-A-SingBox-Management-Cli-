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

#[allow(dead_code)]
pub fn enable() {
    println!("🔄 正在设置开机启动 sing-box...");
    execute_systemctl("enable");
}

#[allow(dead_code)]
pub fn disable() {
    println!("🔄 正在取消开机启动 sing-box...");
    execute_systemctl("disable");
}

#[allow(dead_code)]
pub fn kill() {
    println!("💀 正在强制终止 sing-box 进程...");
    let status = Command::new("pkill").arg("-f").arg("sing-box").status();

    match status {
        Ok(s) if s.success() => println!("=> 强制终止成功"),
        Ok(s) => eprintln!("=> 强制终止失败，退出码: {}", s),
        Err(e) => eprintln!("=> 无法执行 pkill: {}", e),
    }
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
