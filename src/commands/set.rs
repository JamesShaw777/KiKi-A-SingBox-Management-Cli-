use std::fs;
use serde_json::Value;
use base64::{engine::general_purpose, Engine as _};
use std::error::Error;

pub fn execute(url: &str) -> Result<(), Box<dyn Error>> {
    // 1. 处理基础字符串
    let mut main_part = url.trim_start_matches("ss://")
        .split('#') // 彻底切除 #tag
        .next()
        .unwrap_or("")
        .to_string();

    // 2. 解决 Invalid padding: 补齐 4 的倍数
    while main_part.len() % 4 != 0 {
        main_part.push('=');
    }

    // 3. 尝试解码
    // 情况 A: 全编码格式 ss://[base64(method:pass@host:port)]
    // 情况 B: 插件/部分编码格式 ss://[base64(method:pass)]@host:port
    
    let decoded_str = match general_purpose::STANDARD.decode(&main_part) {
        Ok(bytes) => String::from_utf8(bytes)?,
        Err(_) => {
            // 如果全解码失败，尝试处理部分编码格式
            // 找到最后一个 @ 符号
            let at_split: Vec<&str> = main_part.splitn(2, '@').collect();
            if at_split.len() == 2 {
                let mut user_info_b64 = at_split[0].to_string();
                while user_info_b64.len() % 4 != 0 { user_info_b64.push('='); }
                let user_info_bytes = general_purpose::STANDARD.decode(user_info_b64)?;
                format!("{}@{}", String::from_utf8(user_info_bytes)?, at_split[1])
            } else {
                return Err("无法识别的 SS 链接格式".into());
            }
        }
    };

    // 4. 从解码后的字符串 (method:pass@host:port) 提取字段
    // 逻辑：先找最后一个 @ 分隔用户信息和地址，再分别找冒号
    let at_index = decoded_str.rfind('@').ok_or("链接中缺少 @ 符号")?;
    let (user_info, server_addr) = decoded_str.split_at(at_index);
    let server_addr = &server_addr[1..]; // 去掉开头的 @

    let auth_parts: Vec<&str> = user_info.splitn(2, ':').collect();
    let addr_parts: Vec<&str> = server_addr.rsplitn(2, ':').collect(); // 从右边找，兼容 IPv6

    if auth_parts.len() < 2 || addr_parts.len() < 2 {
        return Err("解析用户信息或地址失败".into());
    }

    let method = auth_parts[0];
    let password = auth_parts[1];
    let server = addr_parts[1];
    let port = addr_parts[0].parse::<u16>()?;

    // 5. 填充配置文件 (逻辑保持不变)
    update_config(method, password, server, port)
}

fn update_config(method: &str, pass: &str, host: &str, port: u16) -> Result<(), Box<dyn Error>> {
    let path = "/etc/sing-box/config.json";
    let content = fs::read_to_string(path)?;
    let mut config: Value = serde_json::from_str(&content)?;

    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds {
            if outbound.get("type") == Some(&Value::String("shadowsocks".to_string())) {
                outbound["method"] = Value::String(method.to_string());
                outbound["password"] = Value::String(pass.to_string());
                outbound["server"] = Value::String(host.to_string());
                outbound["server_port"] = Value::Number(port.into());
                break;
            }
        }
    }

    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    println!("✅ 配置已更新 => {}:{}", host, port);
    Ok(())
}