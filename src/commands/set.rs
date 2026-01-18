use std::fs;
use serde_json::Value;
use base64::{engine::general_purpose, Engine as _};
use std::error::Error;

pub fn execute(url: &str) -> Result<(), Box<dyn Error>> {
    // 逻辑：解析链接
    let url_body = url.trim_start_matches("ss://").split('#').next().unwrap();
    let decoded = general_purpose::STANDARD.decode(url_body)?;
    let decoded_str = String::from_utf8(decoded)?;
    
    let parts: Vec<&str> = decoded_str.splitn(2, '@').collect();
    let auth: Vec<&str> = parts[0].splitn(2, ':').collect();
    let addr: Vec<&str> = parts[1].splitn(2, ':').collect();

    let method = auth[0];
    let password = auth[1];
    let server = addr[0];
    let port = addr[1].parse::<u16>()?;

    // 逻辑：修改文件
    let path = "/etc/sing-box/config.json";
    let content = fs::read_to_string(path)?;
    let mut config: Value = serde_json::from_str(&content)?;

    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds {
            if outbound.get("type") == Some(&Value::String("shadowsocks".to_string())) {
                outbound["method"] = Value::String(method.to_string());
                outbound["password"] = Value::String(password.to_string());
                outbound["server"] = Value::String(server.to_string());
                outbound["server_port"] = Value::Number(port.into());
                break;
            }
        }
    }

    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    println!("✅ 节点已成功切换至: {}", server);
    Ok(())
}