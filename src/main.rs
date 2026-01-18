mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kiki")]
#[command(version = "1.0", about = "一个简单的 singbox 管理工具")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 设置/配置相关
    Set { url: String },
    /// 检查配置有效性
    Check,
    /// 启动服务
    Start,
    /// 停止服务
    Stop,
    /// 重启服务
    Restart,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Set { url } => {
            if let Err(e) = commands::set::execute(url) {
                eprintln!("❌ Error: {}", e);
            }
        }
        Commands::Check => commands::check::execute(),
        Commands::Start => commands::service::start(),
        Commands::Stop => commands::service::stop(),
        Commands::Restart => commands::service::restart(),
    }
}