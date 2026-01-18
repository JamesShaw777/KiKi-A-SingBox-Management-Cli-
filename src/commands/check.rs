use std::process::Command;
use std::path::Path;

pub fn execute() {
    println!("🔍 开始系统环境检查...");

    // 1. 检查是否安装了 sing-box
    // 我们尝试运行 sing-box version 来确认它是否在系统路径中
    let singbox_check = Command::new("sing-box")
        .arg("version")
        .output();

    if singbox_check.is_err() {
        println!("❌ 未检测到 sing-box 程序。");
        println!("=> 请确保已安装 sing-box 并将其添加到系统 PATH 中。");
        return;
    }
    println!("✅ 已检测到 sing-box 程序");

    // 2. 检查 config.json 文件是否存在
    let config_path = "/etc/sing-box/config.json";
    if !Path::new(config_path).exists() {
        println!("❌ 配置文件不存在: {}", config_path);
        println!("=> 请先执行 'kiki set <URL>' 来生成或配置该文件。");
        return;
    }
    println!("✅ 配置文件已找到: {}", config_path);

    // 3. 都没有问题，执行 sing-box 自带的配置语法校验
    println!("⚙️ 正在执行配置语法深度检查...");
    let output = Command::new("sing-box")
        .args(["check", "-c", config_path])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                println!("⭐⭐ 所有检查已通过！kiki 随时可以启动。");
            } else {
                eprintln!("❌ 配置文件语法有误！");
                eprintln!("=> 错误详情：\n{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => eprintln!("=> 无法执行校验命令: {}", e),
    }
}