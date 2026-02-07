use std::fs;
use serde_json::{Value, json};
use base64::{engine::general_purpose, Engine as _};
use std::error::Error;
use std::collections::HashMap;

pub fn execute(url: &str) -> Result<(), Box<dyn Error>> {
    if url.starts_with("ss://") {
        handle_shadowsocks(url)
    } else if url.starts_with("vmess://") {
        handle_vmess(url)
    } else if url.starts_with("trojan://") {
        handle_trojan(url)
    } else if url.starts_with("vless://") {
        handle_vless(url)
    } else if url.starts_with("hy2://") || url.starts_with("hysteria2://") {
        handle_hysteria2(url)
    } else if url.starts_with("anytls://") {
        handle_anytls(url)
    } else {
        Err("不支持的协议，请提供 ss://, vmess://, trojan://, vless://, hy2://, hysteria2:// 或 anytls:// 链接".into())
    }
}

fn handle_shadowsocks(url: &str) -> Result<(), Box<dyn Error>> {
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

    // 5. 填充配置文件
    update_shadowsocks_config(method, password, server, port)
}

fn handle_vmess(url: &str) -> Result<(), Box<dyn Error>> {
    // 1. 提取 base64 部分（去掉 vmess:// 前缀和 #tag）
    let mut encoded_part = url.trim_start_matches("vmess://")
        .split('#')
        .next()
        .unwrap_or("")
        .to_string();

    // 2. 补齐 base64 padding
    while encoded_part.len() % 4 != 0 {
        encoded_part.push('=');
    }

    // 3. 解码 base64 得到 JSON
    let decoded_bytes = general_purpose::STANDARD.decode(&encoded_part)?;
    let decoded_str = String::from_utf8(decoded_bytes)?;
    let vmess_obj: HashMap<String, serde_json::Value> = serde_json::from_str(&decoded_str)?;

    // 4. 提取必需字段
    let uuid = vmess_obj.get("id")
        .and_then(|v| v.as_str())
        .ok_or("缺少 id (UUID) 字段")?;
    
    let server = vmess_obj.get("add")
        .and_then(|v| v.as_str())
        .ok_or("缺少 add (服务器地址) 字段")?;
    
    let port = vmess_obj.get("port")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<u16>().ok()))
        .or_else(|| vmess_obj.get("port").and_then(|v| v.as_u64().map(|n| n as u16)))
        .ok_or("缺少或无效的 port 字段")?;

    // 5. 提取可选字段，使用默认值
    let security = vmess_obj.get("scy")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    
    let alter_id = vmess_obj.get("aid")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<u32>().ok()))
        .or_else(|| vmess_obj.get("aid").and_then(|v| v.as_u64().map(|n| n as u32)))
        .unwrap_or(0);

    let network = vmess_obj.get("net")
        .and_then(|v| v.as_str())
        .unwrap_or("tcp");

    let tls = vmess_obj.get("tls")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let host = vmess_obj.get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let path = vmess_obj.get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let sni = vmess_obj.get("sni")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let alpn = vmess_obj.get("alpn")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let fingerprint = vmess_obj.get("fp")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // 6. 更新配置文件
    update_vmess_config(uuid, server, port, security, alter_id, network, 
                       tls, host, path, sni, alpn, fingerprint)
}

fn update_shadowsocks_config(method: &str, pass: &str, host: &str, port: u16) -> Result<(), Box<dyn Error>> {
    let path = "/etc/sing-box/config.json";
    let content = fs::read_to_string(path)?;
    let mut config: Value = serde_json::from_str(&content)?;

    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds {
            if outbound.get("tag") == Some(&Value::String("proxy".to_string())) {
                // 保持 tag，清除旧字段后添加新配置
                let tag = outbound.get("tag").cloned().unwrap_or(Value::Null);
                *outbound = json!({ "tag": tag });
                
                outbound["type"] = Value::String("shadowsocks".to_string());
                outbound["method"] = Value::String(method.to_string());
                outbound["password"] = Value::String(pass.to_string());
                outbound["server"] = Value::String(host.to_string());
                outbound["server_port"] = Value::Number(port.into());
                break;
            }
        }
    }

    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    println!("✅ Shadowsocks 配置已更新 => {}:{}", host, port);
    Ok(())
}

fn update_vmess_config(
    uuid: &str, 
    server: &str, 
    port: u16, 
    security: &str, 
    alter_id: u32, 
    network: &str,
    tls: &str,
    host: &str,
    path: &str,
    sni: &str,
    alpn: &str,
    fingerprint: &str,
) -> Result<(), Box<dyn Error>> {
    let path_file = "/etc/sing-box/config.json";
    let content = fs::read_to_string(path_file)?;
    let mut config: Value = serde_json::from_str(&content)?;

    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds {
            if outbound.get("tag") == Some(&Value::String("proxy".to_string())) {
                // 保持 tag，清除旧字段后添加新配置
                let tag = outbound.get("tag").cloned().unwrap_or(Value::Null);
                *outbound = json!({ "tag": tag });
                
                outbound["type"] = Value::String("vmess".to_string());
                outbound["uuid"] = Value::String(uuid.to_string());
                outbound["server"] = Value::String(server.to_string());
                outbound["server_port"] = Value::Number(port.into());
                outbound["security"] = Value::String(security.to_string());
                outbound["alter_id"] = Value::Number(alter_id.into());
                outbound["network"] = Value::String(network.to_string());

                // 构造 transport 对象
                let mut transport = json!({});
                if network == "ws" {
                    transport["type"] = Value::String("websocket".to_string());
                    if !path.is_empty() {
                        transport["path"] = Value::String(path.to_string());
                    }
                    if !host.is_empty() {
                        transport["headers"] = json!({ "Host": host });
                    }
                } else if network == "h2" {
                    transport["type"] = Value::String("http".to_string());
                    if !host.is_empty() {
                        transport["host"] = Value::String(host.to_string());
                    }
                    if !path.is_empty() {
                        transport["path"] = Value::String(path.to_string());
                    }
                } else {
                    transport["type"] = Value::String(network.to_string());
                }
                outbound["transport"] = transport;

                // 构造 TLS 对象
                let mut tls_config = json!({});
                if !tls.is_empty() && tls != "false" && tls != "none" {
                    tls_config["enabled"] = Value::Bool(true);
                    if !sni.is_empty() {
                        tls_config["server_name"] = Value::String(sni.to_string());
                    } else if !host.is_empty() {
                        tls_config["server_name"] = Value::String(host.to_string());
                    }
                    if !fingerprint.is_empty() {
                        tls_config["utls"] = json!({
                            "enabled": true,
                            "fingerprint": fingerprint
                        });
                    }
                    if !alpn.is_empty() {
                        let alpn_list: Vec<&str> = alpn.split(',').collect();
                        tls_config["alpn"] = Value::Array(
                            alpn_list.iter().map(|a| Value::String(a.trim().to_string())).collect()
                        );
                    }
                }
                outbound["tls"] = tls_config;

                // 其他必需字段
                outbound["global_padding"] = Value::Bool(false);
                outbound["authenticated_length"] = Value::Bool(true);
                outbound["packet_encoding"] = Value::String("".to_string());
                outbound["multiplex"] = json!({});

                break;
            }
        }
    }

    fs::write(path_file, serde_json::to_string_pretty(&config)?)?;
    println!("✅ VMess 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn handle_trojan(url: &str) -> Result<(), Box<dyn Error>> {
    // trojan://password@server:port?...#tag
    let url_str = url.trim_start_matches("trojan://");
    
    // 先去掉 #tag 片段
    let url_str = url_str.split('#').next().unwrap_or("");
    
    // 分离密码和地址
    let at_index = url_str.rfind('@').ok_or("链接中缺少 @ 符号")?;
    let (password, server_part) = url_str.split_at(at_index);
    let server_part = &server_part[1..];
    
    // 去掉查询参数
    let server_part = server_part.split('?').next().unwrap_or("");
    
    // 分离服务器和端口（从右边找，兼容 IPv6）
    let addr_parts: Vec<&str> = server_part.rsplitn(2, ':').collect();
    if addr_parts.len() < 2 {
        return Err("无法解析服务器地址和端口".into());
    }
    
    let server = addr_parts[1];
    let port = addr_parts[0].parse::<u16>()?;
    
    update_trojan_config(password, server, port)
}

fn handle_vless(url: &str) -> Result<(), Box<dyn Error>> {
    // vless://uuid@server:port?...#tag
    let url_str = url.trim_start_matches("vless://");
    
    // 先去掉 #tag 片段
    let url_str = url_str.split('#').next().unwrap_or("");
    
    // 分离 UUID 和地址
    let at_index = url_str.rfind('@').ok_or("链接中缺少 @ 符号")?;
    let (uuid, server_part) = url_str.split_at(at_index);
    let server_part = &server_part[1..];
    
    // 分离服务器、端口和查询参数
    let (server_addr, query) = server_part.split_once('?').unwrap_or((server_part, ""));
    
    let addr_parts: Vec<&str> = server_addr.rsplitn(2, ':').collect();
    if addr_parts.len() < 2 {
        return Err("无法解析服务器地址和端口".into());
    }
    
    let server = addr_parts[1];
    let port = addr_parts[0].parse::<u16>()?;
    
    // 解析查询参数
    let mut flow = "";
    let mut network = "tcp";
    let mut tls = "";
    let mut sni = "";
    let mut host = "";
    let mut path = "";
    let mut alpn = "";
    
    for param in query.split('&') {
        if let Some((key, val)) = param.split_once('=') {
            match key {
                "flow" => flow = val,
                "type" => network = val,
                "security" => tls = val,
                "sni" => sni = val,
                "host" => host = val,
                "path" => path = val,
                "alpn" => alpn = val,
                _ => {}
            }
        }
    }
    
    update_vless_config(uuid, server, port, flow, network, tls, sni, host, path, alpn)
}

fn handle_hysteria2(url: &str) -> Result<(), Box<dyn Error>> {
    // hysteria2://uuid@server:port?peer=...&insecure=...&obfs=...&obfs-password=...#tag
    let url_str = if url.starts_with("hysteria2://") {
        url.trim_start_matches("hysteria2://")
    } else {
        url.trim_start_matches("hy2://")
    };
    
    // 先去掉 #tag 片段
    let url_str = url_str.split('#').next().unwrap_or("");
    
    // 分离 UUID 和地址
    let at_index = url_str.rfind('@').ok_or("链接中缺少 @ 符号")?;
    let (uuid, server_part) = url_str.split_at(at_index);
    let server_part = &server_part[1..];
    
    // 分离服务器、端口和查询参数
    let (server_addr, query) = server_part.split_once('?').unwrap_or((server_part, ""));
    
    let addr_parts: Vec<&str> = server_addr.rsplitn(2, ':').collect();
    if addr_parts.len() < 2 {
        return Err("无法解析服务器地址和端口".into());
    }
    
    let server = addr_parts[1];
    let port = addr_parts[0].parse::<u16>()?;
    
    // 解析查询参数
    let mut peer = "";
    let mut insecure_opt: Option<bool> = None; // None=未指定, Some(true)=1, Some(false)=0
    let mut obfs_opt: Option<&str> = None; // None=未指定, Some("")=给空值
    let mut obfs_password_str = String::new();
    let mut sni = "";
    let mut alpn = "";
    
    for param in query.split('&') {
        if let Some((key, val)) = param.split_once('=') {
            match key {
                "peer" => peer = val,
                "insecure" => insecure_opt = Some(val == "1"),
                "obfs" => obfs_opt = Some(val),
                "obfs-password" => {
                    // 简单的 URL 解码（%3D 等）
                    obfs_password_str = val.replace("%3D", "=").replace("%2B", "+").replace("%2F", "/");
                }
                "sni" => sni = val,
                "alpn" => alpn = val,
                _ => {}
            }
        }
    }
    
    update_hysteria2_config(uuid, server, port, peer, insecure_opt, obfs_opt, &obfs_password_str, sni, alpn)
}

fn handle_anytls(url: &str) -> Result<(), Box<dyn Error>> {
    // anytls://password@server:port?...#tag
    let url_str = url.trim_start_matches("anytls://");

    // 先去掉 #tag 片段
    let url_str = url_str.split('#').next().unwrap_or("");

    // 分离服务器、端口和查询参数（查询参数当前忽略）
    let (server_part, _query) = url_str.split_once('?').unwrap_or((url_str, ""));

    // 分离密码和地址
    let at_index = server_part.rfind('@').ok_or("链接中缺少 @ 符号")?;
    let (password, server_addr) = server_part.split_at(at_index);
    let server_addr = &server_addr[1..];

    // 分离服务器和端口（从右边找，兼容 IPv6）
    let addr_parts: Vec<&str> = server_addr.rsplitn(2, ':').collect();
    if addr_parts.len() < 2 {
        return Err("无法解析服务器地址和端口".into());
    }

    let server = addr_parts[1];
    let port = addr_parts[0].parse::<u16>()?;

    update_anytls_config(password, server, port)
}

fn update_trojan_config(password: &str, server: &str, port: u16) -> Result<(), Box<dyn Error>> {
    let path = "/etc/sing-box/config.json";
    let content = fs::read_to_string(path)?;
    let mut config: Value = serde_json::from_str(&content)?;

    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds {
            if outbound.get("tag") == Some(&Value::String("proxy".to_string())) {
                // 保持 tag，清除旧字段后添加新配置
                let tag = outbound.get("tag").cloned().unwrap_or(Value::Null);
                *outbound = json!({ "tag": tag });
                
                outbound["type"] = Value::String("trojan".to_string());
                outbound["password"] = Value::String(password.to_string());
                outbound["server"] = Value::String(server.to_string());
                outbound["server_port"] = Value::Number(port.into());
                outbound["network"] = Value::String("tcp".to_string());
                outbound["tls"] = json!({});
                outbound["multiplex"] = json!({});
                outbound["transport"] = json!({});
                break;
            }
        }
    }

    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    println!("✅ Trojan 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn update_vless_config(
    uuid: &str,
    server: &str,
    port: u16,
    flow: &str,
    network: &str,
    tls: &str,
    sni: &str,
    host: &str,
    path: &str,
    alpn: &str,
) -> Result<(), Box<dyn Error>> {
    let path_file = "/etc/sing-box/config.json";
    let content = fs::read_to_string(path_file)?;
    let mut config: Value = serde_json::from_str(&content)?;

    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds {
            if outbound.get("tag") == Some(&Value::String("proxy".to_string())) {
                // 保持 tag，清除旧字段后添加新配置
                let tag = outbound.get("tag").cloned().unwrap_or(Value::Null);
                *outbound = json!({ "tag": tag });
                
                outbound["type"] = Value::String("vless".to_string());
                outbound["uuid"] = Value::String(uuid.to_string());
                outbound["server"] = Value::String(server.to_string());
                outbound["server_port"] = Value::Number(port.into());
                
                if !flow.is_empty() {
                    outbound["flow"] = Value::String(flow.to_string());
                }
                
                outbound["network"] = Value::String(network.to_string());
                
                // 构造 transport 对象
                let mut transport = json!({});
                if network == "ws" {
                    transport["type"] = Value::String("websocket".to_string());
                    if !path.is_empty() {
                        transport["path"] = Value::String(path.to_string());
                    }
                    if !host.is_empty() {
                        transport["headers"] = json!({ "Host": host });
                    }
                } else if network == "h2" {
                    transport["type"] = Value::String("http".to_string());
                    if !host.is_empty() {
                        transport["host"] = Value::String(host.to_string());
                    }
                    if !path.is_empty() {
                        transport["path"] = Value::String(path.to_string());
                    }
                } else if network == "grpc" {
                    transport["type"] = Value::String("grpc".to_string());
                    if !path.is_empty() {
                        transport["service_name"] = Value::String(path.to_string());
                    }
                } else {
                    transport["type"] = Value::String(network.to_string());
                }
                outbound["transport"] = transport;
                
                // 构造 TLS 对象
                let mut tls_config = json!({});
                if !tls.is_empty() && tls != "none" {
                    tls_config["enabled"] = Value::Bool(true);
                    if !sni.is_empty() {
                        tls_config["server_name"] = Value::String(sni.to_string());
                    } else if !host.is_empty() {
                        tls_config["server_name"] = Value::String(host.to_string());
                    }
                    if !alpn.is_empty() {
                        let alpn_list: Vec<&str> = alpn.split(',').collect();
                        tls_config["alpn"] = Value::Array(
                            alpn_list.iter().map(|a| Value::String(a.trim().to_string())).collect()
                        );
                    }
                }
                outbound["tls"] = tls_config;
                
                outbound["packet_encoding"] = Value::String("".to_string());
                outbound["multiplex"] = json!({});
                break;
            }
        }
    }

    fs::write(path_file, serde_json::to_string_pretty(&config)?)?;
    println!("✅ VLESS 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn update_hysteria2_config(
    uuid: &str,
    server: &str,
    port: u16,
    peer: &str,
    insecure_opt: Option<bool>,
    obfs_opt: Option<&str>,
    obfs_password: &str,
    sni: &str,
    alpn: &str,
) -> Result<(), Box<dyn Error>> {
    let path = "/etc/sing-box/config.json";
    let content = fs::read_to_string(path)?;
    let mut config: Value = serde_json::from_str(&content)?;

    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds {
            if outbound.get("tag") == Some(&Value::String("proxy".to_string())) {
                // 保持 tag，清除旧字段后添加新配置
                let tag = outbound.get("tag").cloned().unwrap_or(Value::Null);
                *outbound = json!({ "tag": tag });
                
                outbound["type"] = Value::String("hysteria2".to_string());
                outbound["password"] = Value::String(uuid.to_string()); // Hysteria2 使用 password 字段存储 UUID
                outbound["server"] = Value::String(server.to_string());
                outbound["server_port"] = Value::Number(port.into());
                
                // 构造 obfs 对象 - 只在明确提供时
                if let Some(obfs_val) = obfs_opt {
                    if !obfs_val.is_empty() {
                        let mut obfs_config = json!({"type": obfs_val});
                        if !obfs_password.is_empty() {
                            obfs_config["password"] = Value::String(obfs_password.to_string());
                        }
                        outbound["obfs"] = obfs_config;
                    }
                }
                
                // 构造 TLS 对象
                let mut tls_config = json!({});
                let mut has_tls_config = false;
                
                // 只在明确设置为 true 时添加 insecure
                if let Some(insecure) = insecure_opt {
                    if insecure {
                        tls_config["insecure"] = Value::Bool(true);
                        has_tls_config = true;
                    }
                }
                
                // sni 和 server_name
                let server_name = if !sni.is_empty() {
                    sni
                } else if !peer.is_empty() {
                    peer
                } else {
                    ""
                };
                
                if !server_name.is_empty() {
                    tls_config["server_name"] = Value::String(server_name.to_string());
                    has_tls_config = true;
                }
                
                // alpn
                if !alpn.is_empty() {
                    let alpn_list: Vec<&str> = alpn.split(',').collect();
                    tls_config["alpn"] = Value::Array(
                        alpn_list.iter().map(|a| Value::String(a.trim().to_string())).collect()
                    );
                    has_tls_config = true;
                }
                
                // 只在有有效配置时添加 tls
                if has_tls_config {
                    outbound["tls"] = tls_config;
                }
                
                break;
            }
        }
    }

    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    println!("✅ Hysteria2 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn update_anytls_config(password: &str, server: &str, port: u16) -> Result<(), Box<dyn Error>> {
    let path = "/etc/sing-box/config.json";
    let content = fs::read_to_string(path)?;
    let mut config: Value = serde_json::from_str(&content)?;

    if let Some(outbounds) = config.get_mut("outbounds").and_then(|o| o.as_array_mut()) {
        for outbound in outbounds {
            if outbound.get("tag") == Some(&Value::String("proxy".to_string())) {
                // 保持 tag，清除旧字段后添加新配置
                let tag = outbound.get("tag").cloned().unwrap_or(Value::Null);
                *outbound = json!({ "tag": tag });

                outbound["type"] = Value::String("anytls".to_string());
                outbound["password"] = Value::String(password.to_string());
                outbound["server"] = Value::String(server.to_string());
                outbound["server_port"] = Value::Number(port.into());
                outbound["tls"] = json!({});
                break;
            }
        }
    }

    fs::write(path, serde_json::to_string_pretty(&config)?)?;
    println!("✅ AnyTLS 配置已更新 => {}:{}", server, port);
    Ok(())
}
