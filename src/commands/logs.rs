use std::process::Command;

pub fn execute(follow: bool) {
    let mut cmd = Command::new("journalctl");
    
    cmd.arg("-u").arg("sing-box")
        .arg("--output").arg("cat");
    
    if follow {
        println!("📺 实时跟踪 sing-box 日志（按 Ctrl+C 退出）...");
        cmd.arg("-f");
    } else {
        println!("📖 显示最近的 sing-box 日志...");
        cmd.arg("-e");
    }
    
    let status = cmd.status();
    
    match status {
        Ok(s) if s.success() => {
            if !follow {
                println!("✅ 日志读取完成");
            }
        }
        Ok(s) => eprintln!("❌ 日志读取失败，退出码: {}", s),
        Err(e) => eprintln!("❌ 无法执行 journalctl: {}", e),
    }
}
