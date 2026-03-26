use base64::{engine::general_purpose, Engine as _};
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use url::Url;

const DEFAULT_CONFIG_PATH: &str = "/etc/sing-box/config.json";

pub fn execute(url: &str) -> Result<(), Box<dyn Error>> {
    execute_with_config_path(url, DEFAULT_CONFIG_PATH)
}

fn execute_with_config_path(url: &str, config_path: &str) -> Result<(), Box<dyn Error>> {
    if url.starts_with("ss://") {
        handle_shadowsocks(url, config_path)
    } else if url.starts_with("vmess://") {
        handle_vmess(url, config_path)
    } else if url.starts_with("trojan://") {
        handle_trojan(url, config_path)
    } else if url.starts_with("vless://") {
        handle_vless(url, config_path)
    } else if url.starts_with("hy2://") || url.starts_with("hysteria2://") {
        handle_hysteria2(url, config_path)
    } else if url.starts_with("tuic://") {
        handle_tuic(url, config_path)
    } else if url.starts_with("anytls://") {
        handle_anytls(url, config_path)
    } else {
        Err("不支持的协议，请提供 ss://, vmess://, trojan://, vless://, hy2://, hysteria2://, tuic:// 或 anytls:// 链接".into())
    }
}

fn decode_percent_value(value: &str) -> Result<String, Box<dyn Error>> {
    Ok(percent_decode_str(value).decode_utf8()?.into_owned())
}

fn decode_base64_bytes(input: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let normalized = input.trim();
    if normalized.is_empty() {
        return Err("链接内容为空".into());
    }

    let mut candidates = vec![normalized.to_string()];
    let url_safe_variant = normalized.replace('-', "+").replace('_', "/");
    if url_safe_variant != normalized {
        candidates.push(url_safe_variant);
    }

    for candidate in candidates {
        let mut padded = candidate;
        while padded.len() % 4 != 0 {
            padded.push('=');
        }

        if let Ok(bytes) = general_purpose::STANDARD.decode(&padded) {
            return Ok(bytes);
        }
    }

    Err("base64 解码失败".into())
}

fn decode_base64_text(input: &str) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(decode_base64_bytes(input)?)?)
}

fn split_host_port(server_addr: &str) -> Result<(String, u16), Box<dyn Error>> {
    if let Some(stripped) = server_addr.strip_prefix('[') {
        let closing = stripped.find(']').ok_or("IPv6 地址格式无效")?;
        let host = &stripped[..closing];
        let port = stripped[closing + 1..]
            .strip_prefix(':')
            .ok_or("无法解析服务器地址和端口")?
            .parse::<u16>()?;
        return Ok((host.to_string(), port));
    }

    let (host, port) = server_addr
        .rsplit_once(':')
        .ok_or("无法解析服务器地址和端口")?;

    if host.is_empty() {
        return Err("链接中缺少服务器地址".into());
    }

    Ok((host.to_string(), port.parse::<u16>()?))
}

fn parse_query_params(raw_query: Option<&str>) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut params = HashMap::new();

    if let Some(query) = raw_query {
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }

            let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
            let key = decode_percent_value(raw_key)?;
            let value = decode_percent_value(raw_value)?;
            params.insert(key, value);
        }
    }

    Ok(params)
}

fn parse_standard_url(raw_url: &str) -> Result<Url, Box<dyn Error>> {
    Ok(Url::parse(raw_url)?)
}

fn get_param<'a>(params: &'a HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| params.get(*key).map(String::as_str))
}

fn get_bool_param(params: &HashMap<String, String>, keys: &[&str]) -> Option<bool> {
    get_param(params, keys).and_then(parse_bool_param)
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn json_string_array(value: &str) -> Option<Value> {
    let items = split_csv(value);
    if items.is_empty() {
        None
    } else {
        Some(Value::Array(items.into_iter().map(Value::String).collect()))
    }
}

fn parse_plugin_value(plugin: &str) -> (String, String) {
    let mut escaped = false;
    for (index, ch) in plugin.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            ';' => return (plugin[..index].to_string(), plugin[index + 1..].to_string()),
            _ => {}
        }
    }
    (plugin.to_string(), String::new())
}

fn truthy_tls(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "tls" | "xtls" | "reality" | "1" | "true"
    )
}

fn normalize_transport_type(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "tcp" | "none" | "raw" => String::new(),
        "ws" | "websocket" => "ws".to_string(),
        "http" | "h2" => "http".to_string(),
        "grpc" => "grpc".to_string(),
        "httpupgrade" | "http-upgrade" => "httpupgrade".to_string(),
        "quic" => "quic".to_string(),
        other => other.to_string(),
    }
}

fn normalize_outbound_network(value: Option<&str>) -> &'static str {
    match value
        .map(str::trim)
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "udp" => "udp",
        _ => "tcp",
    }
}

fn build_tls_config(
    enabled: bool,
    server_name: Option<&str>,
    insecure: Option<bool>,
    alpn: Option<&str>,
    fingerprint: Option<&str>,
    reality_public_key: Option<&str>,
    reality_short_id: Option<&str>,
    cert_pin_sha256: Option<&str>,
) -> Option<Value> {
    let mut tls_config = json!({});
    let mut has_fields = false;
    let enable_utls = fingerprint
        .filter(|value| !value.is_empty())
        .is_some()
        || reality_public_key
            .filter(|value| !value.is_empty())
            .is_some()
        || reality_short_id
            .filter(|value| !value.is_empty())
            .is_some();

    if enabled {
        tls_config["enabled"] = Value::Bool(true);
        has_fields = true;
    }

    if let Some(server_name) = server_name.filter(|value| !value.is_empty()) {
        tls_config["server_name"] = Value::String(server_name.to_string());
        has_fields = true;
    }

    if let Some(insecure) = insecure {
        tls_config["insecure"] = Value::Bool(insecure);
        has_fields = true;
    }

    if let Some(alpn) = alpn.filter(|value| !value.is_empty()) {
        if let Some(values) = json_string_array(alpn) {
            tls_config["alpn"] = values;
            has_fields = true;
        }
    }

    if enable_utls {
        let mut utls_config = json!({ "enabled": true });
        if let Some(fingerprint) = fingerprint.filter(|value| !value.is_empty()) {
            utls_config["fingerprint"] = Value::String(fingerprint.to_string());
        }
        tls_config["utls"] = utls_config;
        has_fields = true;
    }

    if let Some(cert_pin_sha256) = cert_pin_sha256.filter(|value| !value.is_empty()) {
        if let Some(values) = json_string_array(cert_pin_sha256) {
            tls_config["certificate_public_key_sha256"] = values;
            has_fields = true;
        }
    }

    if reality_public_key.is_some() || reality_short_id.is_some() {
        let mut reality_config = json!({ "enabled": true });
        if let Some(public_key) = reality_public_key.filter(|value| !value.is_empty()) {
            reality_config["public_key"] = Value::String(public_key.to_string());
        }
        if let Some(short_id) = reality_short_id.filter(|value| !value.is_empty()) {
            reality_config["short_id"] = Value::String(short_id.to_string());
        }
        tls_config["reality"] = reality_config;
        has_fields = true;
    }

    has_fields.then_some(tls_config)
}

fn build_transport_config(
    transport_type: &str,
    host: Option<&str>,
    path: Option<&str>,
    service_name: Option<&str>,
    method: Option<&str>,
    idle_timeout: Option<&str>,
    ping_timeout: Option<&str>,
    permit_without_stream: Option<bool>,
    max_early_data: Option<u32>,
    early_data_header_name: Option<&str>,
) -> Option<Value> {
    if transport_type.is_empty() {
        return None;
    }

    let mut transport = json!({ "type": transport_type });

    match transport_type {
        "ws" => {
            if let Some(path) = path.filter(|value| !value.is_empty()) {
                transport["path"] = Value::String(path.to_string());
            }
            if let Some(host) = host.filter(|value| !value.is_empty()) {
                transport["headers"] = json!({ "Host": host });
            }
            if let Some(max_early_data) = max_early_data.filter(|value| *value > 0) {
                transport["max_early_data"] = Value::Number(max_early_data.into());
            }
            if let Some(header_name) = early_data_header_name.filter(|value| !value.is_empty()) {
                transport["early_data_header_name"] = Value::String(header_name.to_string());
            }
        }
        "http" => {
            if let Some(host) = host.filter(|value| !value.is_empty()) {
                if let Some(values) = json_string_array(host) {
                    transport["host"] = values;
                }
            }
            if let Some(path) = path.filter(|value| !value.is_empty()) {
                transport["path"] = Value::String(path.to_string());
            }
            if let Some(method) = method.filter(|value| !value.is_empty()) {
                transport["method"] = Value::String(method.to_string());
            }
            if let Some(idle_timeout) = idle_timeout.filter(|value| !value.is_empty()) {
                transport["idle_timeout"] = Value::String(idle_timeout.to_string());
            }
            if let Some(ping_timeout) = ping_timeout.filter(|value| !value.is_empty()) {
                transport["ping_timeout"] = Value::String(ping_timeout.to_string());
            }
        }
        "grpc" => {
            if let Some(service_name) = service_name.filter(|value| !value.is_empty()) {
                transport["service_name"] = Value::String(service_name.to_string());
            }
            if let Some(idle_timeout) = idle_timeout.filter(|value| !value.is_empty()) {
                transport["idle_timeout"] = Value::String(idle_timeout.to_string());
            }
            if let Some(ping_timeout) = ping_timeout.filter(|value| !value.is_empty()) {
                transport["ping_timeout"] = Value::String(ping_timeout.to_string());
            }
            if let Some(permit_without_stream) = permit_without_stream {
                transport["permit_without_stream"] = Value::Bool(permit_without_stream);
            }
        }
        "httpupgrade" => {
            if let Some(host) = host.filter(|value| !value.is_empty()) {
                transport["host"] = Value::String(host.to_string());
            }
            if let Some(path) = path.filter(|value| !value.is_empty()) {
                transport["path"] = Value::String(path.to_string());
            }
        }
        "quic" => {}
        _ => {}
    }

    Some(transport)
}

fn update_proxy_outbound(
    config_path: &str,
    mutator: impl FnOnce(&mut Value) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string(config_path)?;
    let mut config: Value = serde_json::from_str(&content)?;

    let outbound = config
        .get_mut("outbounds")
        .and_then(|value| value.as_array_mut())
        .and_then(|outbounds| {
            outbounds
                .iter_mut()
                .find(|outbound| outbound.get("tag") == Some(&Value::String("proxy".to_string())))
        })
        .ok_or("配置文件中缺少 tag 为 proxy 的 outbound")?;

    let tag = outbound.get("tag").cloned().unwrap_or(Value::Null);
    *outbound = json!({ "tag": tag });
    mutator(outbound)?;

    fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

fn require_username(parsed: &Url, field_name: &str) -> Result<String, Box<dyn Error>> {
    let username = decode_percent_value(parsed.username())?;
    if username.is_empty() {
        return Err(format!("链接中缺少 {} 字段", field_name).into());
    }
    Ok(username)
}

fn require_password(parsed: &Url, field_name: &str) -> Result<String, Box<dyn Error>> {
    let password = parsed
        .password()
        .ok_or_else(|| format!("链接中缺少 {} 字段", field_name))?;
    let password = decode_percent_value(password)?;
    if password.is_empty() {
        return Err(format!("链接中缺少 {} 字段", field_name).into());
    }
    Ok(password)
}

fn require_auth_field(parsed: &Url, field_name: &str) -> Result<String, Box<dyn Error>> {
    let username = parsed.username();
    let auth = if let Some(password) = parsed.password() {
        format!("{}:{}", username, password)
    } else {
        username.to_string()
    };

    let auth = decode_percent_value(&auth)?;
    if auth.is_empty() {
        return Err(format!("链接中缺少 {} 字段", field_name).into());
    }
    Ok(auth)
}

fn require_host_and_port(parsed: &Url) -> Result<(String, u16), Box<dyn Error>> {
    let host = parsed.host_str().ok_or("链接中缺少服务器地址")?;
    let port = parsed.port().ok_or("链接中缺少端口")?;
    Ok((host.to_string(), port))
}

fn parse_shadowsocks_parts(url: &str) -> Result<(String, String, String, u16), Box<dyn Error>> {
    let main_part = url
        .trim_start_matches("ss://")
        .split('#')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");

    if main_part.is_empty() {
        return Err("无法识别的 SS 链接格式".into());
    }

    let decoded = if let Some((user_info, server_addr)) = main_part.rsplit_once('@') {
        let decoded_user_info = match decode_base64_text(user_info) {
            Ok(value) => value,
            Err(_) => decode_percent_value(user_info)?,
        };
        format!("{}@{}", decoded_user_info, server_addr)
    } else {
        decode_base64_text(main_part)?
    };

    let (user_info, server_addr) = decoded.rsplit_once('@').ok_or("链接中缺少 @ 符号")?;
    let user_info = decode_percent_value(user_info)?;
    let (method, password) = user_info.split_once(':').ok_or("解析用户信息失败")?;
    let (server, port) = split_host_port(server_addr)?;

    Ok((method.to_string(), password.to_string(), server, port))
}

fn parse_hysteria2_parts(
    url: &str,
) -> Result<
    (
        String,
        String,
        Option<u16>,
        Option<Vec<String>>,
        HashMap<String, String>,
    ),
    Box<dyn Error>,
> {
    let raw = if url.starts_with("hysteria2://") {
        url.trim_start_matches("hysteria2://")
    } else {
        url.trim_start_matches("hy2://")
    };

    let raw = raw.split('#').next().unwrap_or("");
    let (authority, query) = raw.split_once('?').unwrap_or((raw, ""));
    let (auth, server_part) = authority.rsplit_once('@').ok_or("链接中缺少 @ 符号")?;
    let auth = decode_percent_value(auth)?;
    if auth.is_empty() {
        return Err("链接中缺少认证信息".into());
    }

    let (server, port_spec) = if let Some(stripped) = server_part.strip_prefix('[') {
        let closing = stripped.find(']').ok_or("IPv6 地址格式无效")?;
        let host = stripped[..closing].to_string();
        let suffix = &stripped[closing + 1..];
        if let Some(port_spec) = suffix.strip_prefix(':') {
            (host, Some(port_spec.to_string()))
        } else if suffix.is_empty() {
            (host, None)
        } else {
            return Err("无法解析服务器地址和端口".into());
        }
    } else if let Some((host, port_spec)) = server_part.rsplit_once(':') {
        if host.is_empty() {
            return Err("链接中缺少服务器地址".into());
        }
        (host.to_string(), Some(port_spec.to_string()))
    } else {
        (server_part.to_string(), None)
    };

    let (server_port, server_ports) = match port_spec.as_deref() {
        Some(spec) if spec.contains(',') || spec.contains('-') || spec.contains(':') => {
            let mut primary_port = None;
            let mut port_ranges = Vec::new();

            for item in split_csv(spec) {
                if item.contains('-') || item.contains(':') {
                    let normalized = item.replace('-', ":");
                    port_ranges.push(normalized);
                } else if primary_port.is_none() {
                    primary_port = Some(item.parse::<u16>()?);
                } else {
                    let port = item.parse::<u16>()?;
                    port_ranges.push(format!("{port}:{port}"));
                }
            }

            (
                primary_port.or(Some(443)),
                (!port_ranges.is_empty()).then_some(port_ranges),
            )
        }
        Some(spec) if !spec.is_empty() => (Some(spec.parse::<u16>()?), None),
        _ => (Some(443), None),
    };

    let params = parse_query_params(Some(query))?;
    Ok((auth, server, server_port, server_ports, params))
}

fn handle_shadowsocks(url: &str, config_path: &str) -> Result<(), Box<dyn Error>> {
    let (method, password, server, port) = parse_shadowsocks_parts(url)?;
    let main_part = url
        .trim_start_matches("ss://")
        .split('#')
        .next()
        .unwrap_or("");
    let query = main_part.split_once('?').map(|(_, query)| query);
    let params = parse_query_params(query)?;
    let plugin = get_param(&params, &["plugin"]);
    let network = get_param(&params, &["network"]);
    let udp_over_tcp = get_bool_param(&params, &["uot", "udp_over_tcp", "udp-over-tcp"]);

    let (plugin_name, plugin_opts) = plugin
        .map(parse_plugin_value)
        .unwrap_or_else(|| (String::new(), String::new()));

    update_shadowsocks_config(
        config_path,
        &method,
        &password,
        &server,
        port,
        (!plugin_name.is_empty()).then_some(plugin_name.as_str()),
        (!plugin_opts.is_empty()).then_some(plugin_opts.as_str()),
        network,
        udp_over_tcp,
    )
}

fn handle_vmess(url: &str, config_path: &str) -> Result<(), Box<dyn Error>> {
    let encoded_part = url
        .trim_start_matches("vmess://")
        .split('#')
        .next()
        .unwrap_or("");

    let decoded_str = decode_base64_text(encoded_part)?;
    let vmess_obj: HashMap<String, serde_json::Value> = serde_json::from_str(&decoded_str)?;
    let uuid = vmess_obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("缺少 id (UUID) 字段")?;

    let server = vmess_obj
        .get("add")
        .and_then(|v| v.as_str())
        .ok_or("缺少 add (服务器地址) 字段")?;

    let port = vmess_obj
        .get("port")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<u16>().ok()))
        .or_else(|| {
            vmess_obj
                .get("port")
                .and_then(|v| v.as_u64().map(|n| n as u16))
        })
        .ok_or("缺少或无效的 port 字段")?;

    let security = vmess_obj
        .get("scy")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");

    let alter_id = vmess_obj
        .get("aid")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<u32>().ok()))
        .or_else(|| {
            vmess_obj
                .get("aid")
                .and_then(|v| v.as_u64().map(|n| n as u32))
        })
        .unwrap_or(0);

    let transport_type = vmess_obj
        .get("net")
        .and_then(|v| v.as_str())
        .unwrap_or("tcp");

    let tls = vmess_obj.get("tls").and_then(|v| v.as_str()).unwrap_or("");

    let host = vmess_obj.get("host").and_then(|v| v.as_str()).unwrap_or("");

    let path = vmess_obj.get("path").and_then(|v| v.as_str()).unwrap_or("");

    let sni = vmess_obj.get("sni").and_then(|v| v.as_str()).unwrap_or("");

    let alpn = vmess_obj.get("alpn").and_then(|v| v.as_str()).unwrap_or("");

    let fingerprint = vmess_obj.get("fp").and_then(|v| v.as_str()).unwrap_or("");

    let insecure = vmess_obj
        .get("allowInsecure")
        .and_then(|v| v.as_bool())
        .or_else(|| {
            vmess_obj
                .get("allowInsecure")
                .and_then(|v| v.as_str())
                .and_then(parse_bool_param)
        });
    let packet_encoding = vmess_obj
        .get("packetEncoding")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let service_name = vmess_obj
        .get("serviceName")
        .or_else(|| vmess_obj.get("service_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let method = vmess_obj
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let idle_timeout = vmess_obj
        .get("idleTimeout")
        .or_else(|| vmess_obj.get("idle_timeout"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let ping_timeout = vmess_obj
        .get("pingTimeout")
        .or_else(|| vmess_obj.get("ping_timeout"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let early_data = vmess_obj
        .get("ed")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<u32>().ok()))
        .or_else(|| {
            vmess_obj
                .get("ed")
                .and_then(|v| v.as_u64().map(|n| n as u32))
        });
    let early_data_header_name = vmess_obj.get("eh").and_then(|v| v.as_str()).unwrap_or("");

    update_vmess_config(
        config_path,
        uuid,
        server,
        port,
        security,
        alter_id,
        normalize_outbound_network(None),
        &normalize_transport_type(transport_type),
        tls,
        host,
        path,
        sni,
        alpn,
        fingerprint,
        insecure,
        packet_encoding,
        service_name,
        method,
        idle_timeout,
        ping_timeout,
        None,
        early_data,
        early_data_header_name,
    )
}

fn update_shadowsocks_config(
    config_path: &str,
    method: &str,
    pass: &str,
    host: &str,
    port: u16,
    plugin: Option<&str>,
    plugin_opts: Option<&str>,
    network: Option<&str>,
    udp_over_tcp: Option<bool>,
) -> Result<(), Box<dyn Error>> {
    update_proxy_outbound(config_path, |outbound| {
        outbound["type"] = Value::String("shadowsocks".to_string());
        outbound["method"] = Value::String(method.to_string());
        outbound["password"] = Value::String(pass.to_string());
        outbound["server"] = Value::String(host.to_string());
        outbound["server_port"] = Value::Number(port.into());
        if let Some(plugin) = plugin.filter(|value| !value.is_empty()) {
            outbound["plugin"] = Value::String(plugin.to_string());
        }
        if let Some(plugin_opts) = plugin_opts.filter(|value| !value.is_empty()) {
            outbound["plugin_opts"] = Value::String(plugin_opts.to_string());
        }
        if let Some(network) = network.filter(|value| !value.is_empty()) {
            outbound["network"] = Value::String(network.to_string());
        }
        if let Some(udp_over_tcp) = udp_over_tcp {
            outbound["udp_over_tcp"] = Value::Bool(udp_over_tcp);
        }
        Ok(())
    })?;

    println!("✅ Shadowsocks 配置已更新 => {}:{}", host, port);
    Ok(())
}

fn update_vmess_config(
    config_path: &str,
    uuid: &str,
    server: &str,
    port: u16,
    security: &str,
    alter_id: u32,
    network: &str,
    transport_type: &str,
    tls: &str,
    host: &str,
    path: &str,
    sni: &str,
    alpn: &str,
    fingerprint: &str,
    insecure: Option<bool>,
    packet_encoding: &str,
    service_name: &str,
    method: &str,
    idle_timeout: &str,
    ping_timeout: &str,
    permit_without_stream: Option<bool>,
    early_data: Option<u32>,
    early_data_header_name: &str,
) -> Result<(), Box<dyn Error>> {
    let enable_tls = truthy_tls(tls);
    let tls_config = build_tls_config(
        enable_tls,
        Some(if !sni.is_empty() { sni } else { host }),
        insecure,
        Some(alpn),
        Some(fingerprint),
        None,
        None,
        None,
    );
    let transport = build_transport_config(
        transport_type,
        Some(host),
        Some(path),
        Some(service_name),
        Some(method),
        Some(idle_timeout),
        Some(ping_timeout),
        permit_without_stream,
        early_data,
        Some(early_data_header_name),
    );

    update_proxy_outbound(config_path, |outbound| {
        outbound["type"] = Value::String("vmess".to_string());
        outbound["uuid"] = Value::String(uuid.to_string());
        outbound["server"] = Value::String(server.to_string());
        outbound["server_port"] = Value::Number(port.into());
        outbound["security"] = Value::String(security.to_string());
        outbound["alter_id"] = Value::Number(alter_id.into());
        outbound["network"] = Value::String(network.to_string());
        if let Some(transport) = transport.clone() {
            outbound["transport"] = transport;
        }
        if let Some(tls_config) = tls_config.clone() {
            outbound["tls"] = tls_config;
        }
        outbound["global_padding"] = Value::Bool(false);
        outbound["authenticated_length"] = Value::Bool(true);
        if !packet_encoding.is_empty() {
            outbound["packet_encoding"] = Value::String(packet_encoding.to_string());
        } else {
            outbound["packet_encoding"] = Value::String("".to_string());
        }
        outbound["multiplex"] = json!({});
        Ok(())
    })?;

    println!("✅ VMess 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn handle_trojan(url: &str, config_path: &str) -> Result<(), Box<dyn Error>> {
    let parsed = parse_standard_url(url)?;
    let password = require_auth_field(&parsed, "密码")?;
    let (server, port) = require_host_and_port(&parsed)?;
    let params = parse_query_params(parsed.query())?;
    let transport_type = normalize_transport_type(get_param(&params, &["type"]).unwrap_or(""));
    let network = normalize_outbound_network(get_param(&params, &["network"]));
    let service_name = get_param(&params, &["serviceName", "service_name", "path"]).unwrap_or("");
    let host = get_param(&params, &["host", "authority"]).unwrap_or("");
    let path = get_param(&params, &["path"]).unwrap_or("");
    let sni = get_param(&params, &["sni", "peer"]).unwrap_or("");
    let alpn = get_param(&params, &["alpn"]).unwrap_or("");
    let fingerprint = get_param(&params, &["fp", "fingerprint"]).unwrap_or("");
    let insecure = get_bool_param(&params, &["allowInsecure", "allow_insecure", "insecure"]);
    let method = get_param(&params, &["method"]).unwrap_or("");
    let idle_timeout = get_param(&params, &["idle_timeout", "idleTimeout"]).unwrap_or("");
    let ping_timeout = get_param(&params, &["ping_timeout", "pingTimeout"]).unwrap_or("");
    let permit_without_stream =
        get_bool_param(&params, &["permit_without_stream", "permitWithoutStream"]);
    let early_data =
        get_param(&params, &["ed", "maxEarlyData"]).and_then(|v| v.parse::<u32>().ok());
    let early_data_header_name = get_param(
        &params,
        &["eh", "early_data_header_name", "earlyDataHeaderName"],
    )
    .unwrap_or("");

    update_trojan_config(
        config_path,
        &password,
        &server,
        port,
        network,
        &transport_type,
        host,
        path,
        service_name,
        sni,
        alpn,
        fingerprint,
        insecure,
        method,
        idle_timeout,
        ping_timeout,
        permit_without_stream,
        early_data,
        early_data_header_name,
    )
}

fn handle_vless(url: &str, config_path: &str) -> Result<(), Box<dyn Error>> {
    let parsed = parse_standard_url(url)?;
    let uuid = require_username(&parsed, "UUID")?;
    let (server, port) = require_host_and_port(&parsed)?;
    let params = parse_query_params(parsed.query())?;
    let flow = get_param(&params, &["flow"]).unwrap_or("");
    let transport_type = normalize_transport_type(get_param(&params, &["type"]).unwrap_or(""));
    let outbound_network = normalize_outbound_network(get_param(&params, &["network"]));
    let tls = get_param(&params, &["security"]).unwrap_or("");
    let sni = get_param(&params, &["sni", "peer"]).unwrap_or("");
    let host = get_param(&params, &["host", "authority"]).unwrap_or("");
    let path = get_param(&params, &["path"]).unwrap_or("");
    let alpn = get_param(&params, &["alpn"]).unwrap_or("");
    let fingerprint = get_param(&params, &["fp", "fingerprint"]).unwrap_or("");
    let insecure = get_bool_param(&params, &["allowInsecure", "allow_insecure", "insecure"]);
    let reality_public_key = get_param(&params, &["pbk", "publicKey", "public_key"]).unwrap_or("");
    let reality_short_id = get_param(&params, &["sid", "shortId", "short_id"]).unwrap_or("");
    let packet_encoding = get_param(&params, &["packetEncoding", "packet_encoding"]).unwrap_or("");
    let service_name = get_param(&params, &["serviceName", "service_name", "path"]).unwrap_or("");
    let method = get_param(&params, &["method"]).unwrap_or("");
    let idle_timeout = get_param(&params, &["idle_timeout", "idleTimeout"]).unwrap_or("");
    let ping_timeout = get_param(&params, &["ping_timeout", "pingTimeout"]).unwrap_or("");
    let permit_without_stream =
        get_bool_param(&params, &["permit_without_stream", "permitWithoutStream"]);
    let early_data =
        get_param(&params, &["ed", "maxEarlyData"]).and_then(|v| v.parse::<u32>().ok());
    let early_data_header_name = get_param(
        &params,
        &["eh", "early_data_header_name", "earlyDataHeaderName"],
    )
    .unwrap_or("");

    update_vless_config(
        config_path,
        &uuid,
        &server,
        port,
        flow,
        outbound_network,
        &transport_type,
        tls,
        sni,
        host,
        path,
        alpn,
        fingerprint,
        insecure,
        reality_public_key,
        reality_short_id,
        packet_encoding,
        service_name,
        method,
        idle_timeout,
        ping_timeout,
        permit_without_stream,
        early_data,
        early_data_header_name,
    )
}

fn handle_hysteria2(url: &str, config_path: &str) -> Result<(), Box<dyn Error>> {
    let (password, server, server_port, server_ports, params) = parse_hysteria2_parts(url)?;
    let peer = get_param(&params, &["peer"]).unwrap_or("");
    let insecure_opt = get_bool_param(&params, &["insecure"]);
    let obfs_opt = get_param(&params, &["obfs"]);
    let obfs_password = get_param(&params, &["obfs-password", "obfs_password"]).unwrap_or("");
    let sni = get_param(&params, &["sni"]).unwrap_or("");
    let alpn = get_param(&params, &["alpn"]).unwrap_or("");
    let cert_pin = get_param(&params, &["pinSHA256", "pin_sha256"]).unwrap_or("");
    let network = normalize_outbound_network(get_param(&params, &["network"]));

    update_hysteria2_config(
        config_path,
        &password,
        &server,
        server_port,
        server_ports.as_deref(),
        network,
        peer,
        insecure_opt,
        obfs_opt,
        obfs_password,
        sni,
        alpn,
        cert_pin,
    )
}

fn handle_tuic(url: &str, config_path: &str) -> Result<(), Box<dyn Error>> {
    let parsed = parse_standard_url(url)?;
    let uuid = require_username(&parsed, "UUID")?;
    let password = require_password(&parsed, "密码")?;
    let (server, port) = require_host_and_port(&parsed)?;
    let params = parse_query_params(parsed.query())?;
    let sni = get_param(&params, &["sni"]).unwrap_or("");
    let alpn = get_param(&params, &["alpn"]).unwrap_or("");
    let congestion_control = get_param(&params, &["congestion_control"]).unwrap_or("");
    let udp_relay_mode = get_param(&params, &["udp_relay_mode"]).unwrap_or("");
    let mut udp_over_stream_opt = get_bool_param(&params, &["udp_over_stream"]);
    let zero_rtt_handshake_opt = get_bool_param(&params, &["zero_rtt_handshake"]);
    let heartbeat = get_param(&params, &["heartbeat"]).unwrap_or("");
    let network = normalize_outbound_network(get_param(&params, &["network"]));
    let insecure_opt = get_bool_param(&params, &["allow_insecure", "allowInsecure", "insecure"]);
    let fingerprint = get_param(&params, &["fp", "fingerprint"]).unwrap_or("");

    if udp_over_stream_opt == Some(false) {
        udp_over_stream_opt = None;
    }
    if udp_over_stream_opt == Some(true) && !udp_relay_mode.is_empty() {
        return Err("udp_relay_mode 与 udp_over_stream 冲突，请只保留其一".into());
    }

    update_tuic_config(
        config_path,
        &uuid,
        &password,
        &server,
        port,
        sni,
        alpn,
        congestion_control,
        udp_relay_mode,
        udp_over_stream_opt,
        zero_rtt_handshake_opt,
        heartbeat,
        network,
        insecure_opt,
        fingerprint,
    )
}

fn handle_anytls(url: &str, config_path: &str) -> Result<(), Box<dyn Error>> {
    let parsed = parse_standard_url(url)?;
    let password = require_auth_field(&parsed, "密码")?;
    let (server, port) = require_host_and_port(&parsed)?;
    let params = parse_query_params(parsed.query())?;
    let sni = get_param(&params, &["sni", "peer"]).unwrap_or("");
    let alpn = get_param(&params, &["alpn"]).unwrap_or("");
    let fingerprint = get_param(&params, &["fp", "fingerprint"]).unwrap_or("");
    let insecure = get_bool_param(&params, &["allowInsecure", "allow_insecure", "insecure"]);
    let idle_session_check_interval = get_param(
        &params,
        &["idle_session_check_interval", "idleSessionCheckInterval"],
    )
    .unwrap_or("");
    let idle_session_timeout =
        get_param(&params, &["idle_session_timeout", "idleSessionTimeout"]).unwrap_or("");
    let min_idle_session = get_param(&params, &["min_idle_session", "minIdleSession"])
        .and_then(|v| v.parse::<u32>().ok());

    update_anytls_config(
        config_path,
        &password,
        &server,
        port,
        sni,
        alpn,
        insecure,
        fingerprint,
        idle_session_check_interval,
        idle_session_timeout,
        min_idle_session,
    )
}

fn update_trojan_config(
    config_path: &str,
    password: &str,
    server: &str,
    port: u16,
    network: &str,
    transport_type: &str,
    host: &str,
    path: &str,
    service_name: &str,
    sni: &str,
    alpn: &str,
    fingerprint: &str,
    insecure: Option<bool>,
    method: &str,
    idle_timeout: &str,
    ping_timeout: &str,
    permit_without_stream: Option<bool>,
    early_data: Option<u32>,
    early_data_header_name: &str,
) -> Result<(), Box<dyn Error>> {
    let tls_config = build_tls_config(
        true,
        Some(if !sni.is_empty() { sni } else { server }),
        insecure,
        Some(alpn),
        Some(fingerprint),
        None,
        None,
        None,
    )
    .ok_or("无法构造 TLS 配置")?;
    let transport = build_transport_config(
        transport_type,
        Some(host),
        Some(path),
        Some(service_name),
        Some(method),
        Some(idle_timeout),
        Some(ping_timeout),
        permit_without_stream,
        early_data,
        Some(early_data_header_name),
    );

    update_proxy_outbound(config_path, |outbound| {
        outbound["type"] = Value::String("trojan".to_string());
        outbound["password"] = Value::String(password.to_string());
        outbound["server"] = Value::String(server.to_string());
        outbound["server_port"] = Value::Number(port.into());
        outbound["network"] = Value::String(network.to_string());
        outbound["tls"] = tls_config.clone();
        if let Some(transport) = transport.clone() {
            outbound["transport"] = transport;
        }
        outbound["multiplex"] = json!({});
        Ok(())
    })?;

    println!("✅ Trojan 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn update_vless_config(
    config_path: &str,
    uuid: &str,
    server: &str,
    port: u16,
    flow: &str,
    network: &str,
    transport_type: &str,
    tls: &str,
    sni: &str,
    host: &str,
    path: &str,
    alpn: &str,
    fingerprint: &str,
    insecure: Option<bool>,
    reality_public_key: &str,
    reality_short_id: &str,
    packet_encoding: &str,
    service_name: &str,
    method: &str,
    idle_timeout: &str,
    ping_timeout: &str,
    permit_without_stream: Option<bool>,
    early_data: Option<u32>,
    early_data_header_name: &str,
) -> Result<(), Box<dyn Error>> {
    let enable_tls = truthy_tls(tls);
    let tls_config = build_tls_config(
        enable_tls,
        Some(if !sni.is_empty() { sni } else { host }),
        insecure,
        Some(alpn),
        Some(fingerprint),
        (!reality_public_key.is_empty()).then_some(reality_public_key),
        (!reality_short_id.is_empty()).then_some(reality_short_id),
        None,
    );
    let transport = build_transport_config(
        transport_type,
        Some(host),
        Some(path),
        Some(service_name),
        Some(method),
        Some(idle_timeout),
        Some(ping_timeout),
        permit_without_stream,
        early_data,
        Some(early_data_header_name),
    );

    update_proxy_outbound(config_path, |outbound| {
        outbound["type"] = Value::String("vless".to_string());
        outbound["uuid"] = Value::String(uuid.to_string());
        outbound["server"] = Value::String(server.to_string());
        outbound["server_port"] = Value::Number(port.into());
        if !flow.is_empty() {
            outbound["flow"] = Value::String(flow.to_string());
        }
        outbound["network"] = Value::String(network.to_string());
        if let Some(transport) = transport.clone() {
            outbound["transport"] = transport;
        }
        if let Some(tls_config) = tls_config.clone() {
            outbound["tls"] = tls_config;
        }
        if !packet_encoding.is_empty() {
            outbound["packet_encoding"] = Value::String(packet_encoding.to_string());
        } else {
            outbound["packet_encoding"] = Value::String("".to_string());
        }
        outbound["multiplex"] = json!({});
        Ok(())
    })?;

    println!("✅ VLESS 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn update_hysteria2_config(
    config_path: &str,
    password: &str,
    server: &str,
    server_port: Option<u16>,
    server_ports: Option<&[String]>,
    network: &str,
    peer: &str,
    insecure_opt: Option<bool>,
    obfs_opt: Option<&str>,
    obfs_password: &str,
    sni: &str,
    alpn: &str,
    cert_pin: &str,
) -> Result<(), Box<dyn Error>> {
    let tls_config = build_tls_config(
        true,
        Some(if !sni.is_empty() {
            sni
        } else if !peer.is_empty() {
            peer
        } else {
            server
        }),
        insecure_opt,
        Some(alpn),
        None,
        None,
        None,
        Some(cert_pin),
    )
    .ok_or("无法构造 TLS 配置")?;

    update_proxy_outbound(config_path, |outbound| {
        outbound["type"] = Value::String("hysteria2".to_string());
        outbound["password"] = Value::String(password.to_string());
        outbound["server"] = Value::String(server.to_string());
        if let Some(server_port) = server_port {
            outbound["server_port"] = Value::Number(server_port.into());
        }
        if let Some(server_ports) = server_ports {
            outbound["server_ports"] =
                Value::Array(server_ports.iter().cloned().map(Value::String).collect());
        }
        outbound["network"] = Value::String(network.to_string());
        if let Some(obfs_val) = obfs_opt.filter(|value| !value.is_empty()) {
            let mut obfs_config = json!({ "type": obfs_val });
            if !obfs_password.is_empty() {
                obfs_config["password"] = Value::String(obfs_password.to_string());
            }
            outbound["obfs"] = obfs_config;
        }
        outbound["tls"] = tls_config.clone();
        Ok(())
    })?;

    let port_label = server_port
        .map(|value| value.to_string())
        .or_else(|| server_ports.map(|values| values.join(",")))
        .unwrap_or_else(|| "443".to_string());
    println!("✅ Hysteria2 配置已更新 => {}:{}", server, port_label);
    Ok(())
}

fn update_tuic_config(
    config_path: &str,
    uuid: &str,
    password: &str,
    server: &str,
    port: u16,
    sni: &str,
    alpn: &str,
    congestion_control: &str,
    udp_relay_mode: &str,
    udp_over_stream_opt: Option<bool>,
    zero_rtt_handshake_opt: Option<bool>,
    heartbeat: &str,
    network: &str,
    insecure_opt: Option<bool>,
    fingerprint: &str,
) -> Result<(), Box<dyn Error>> {
    let tls_config = build_tls_config(
        true,
        Some(if !sni.is_empty() { sni } else { server }),
        insecure_opt,
        Some(alpn),
        Some(fingerprint),
        None,
        None,
        None,
    )
    .ok_or("无法构造 TLS 配置")?;

    update_proxy_outbound(config_path, |outbound| {
        outbound["type"] = Value::String("tuic".to_string());
        outbound["uuid"] = Value::String(uuid.to_string());
        outbound["password"] = Value::String(password.to_string());
        outbound["server"] = Value::String(server.to_string());
        outbound["server_port"] = Value::Number(port.into());
        if !congestion_control.is_empty() {
            outbound["congestion_control"] = Value::String(congestion_control.to_string());
        }
        if !udp_relay_mode.is_empty() {
            outbound["udp_relay_mode"] = Value::String(udp_relay_mode.to_string());
        }
        if let Some(udp_over_stream) = udp_over_stream_opt {
            outbound["udp_over_stream"] = Value::Bool(udp_over_stream);
        }
        if let Some(zero_rtt_handshake) = zero_rtt_handshake_opt {
            outbound["zero_rtt_handshake"] = Value::Bool(zero_rtt_handshake);
        }
        if !heartbeat.is_empty() {
            outbound["heartbeat"] = Value::String(heartbeat.to_string());
        }
        outbound["network"] = Value::String(network.to_string());
        outbound["tls"] = tls_config.clone();
        Ok(())
    })?;

    println!("✅ TUIC 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn update_anytls_config(
    config_path: &str,
    password: &str,
    server: &str,
    port: u16,
    sni: &str,
    alpn: &str,
    insecure: Option<bool>,
    fingerprint: &str,
    idle_session_check_interval: &str,
    idle_session_timeout: &str,
    min_idle_session: Option<u32>,
) -> Result<(), Box<dyn Error>> {
    let tls_config = build_tls_config(
        true,
        Some(if !sni.is_empty() { sni } else { server }),
        insecure,
        Some(alpn),
        Some(fingerprint),
        None,
        None,
        None,
    )
    .ok_or("无法构造 TLS 配置")?;

    update_proxy_outbound(config_path, |outbound| {
        outbound["type"] = Value::String("anytls".to_string());
        outbound["password"] = Value::String(password.to_string());
        outbound["server"] = Value::String(server.to_string());
        outbound["server_port"] = Value::Number(port.into());
        outbound["tls"] = tls_config.clone();
        if !idle_session_check_interval.is_empty() {
            outbound["idle_session_check_interval"] =
                Value::String(idle_session_check_interval.to_string());
        }
        if !idle_session_timeout.is_empty() {
            outbound["idle_session_timeout"] = Value::String(idle_session_timeout.to_string());
        }
        if let Some(min_idle_session) = min_idle_session {
            outbound["min_idle_session"] = Value::Number(min_idle_session.into());
        }
        Ok(())
    })?;

    println!("✅ AnyTLS 配置已更新 => {}:{}", server, port);
    Ok(())
}

fn parse_bool_param(value: &str) -> Option<bool> {
    if value.is_empty() {
        return Some(true);
    }
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value as JsonValue;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("kiki-{name}-{}-{nanos}.json", std::process::id()));
        path
    }

    fn run_and_check(label: &str, url: &str) -> JsonValue {
        let path = temp_config_path(label);
        fs::write(&path, include_str!("../../config.json")).unwrap();

        execute_with_config_path(url, path.to_str().unwrap()).unwrap();

        let output = Command::new("sing-box")
            .args(["check", "-c", path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let content = fs::read_to_string(&path).unwrap();
        let value = serde_json::from_str(&content).unwrap();
        let _ = fs::remove_file(path);
        value
    }

    fn proxy_outbound(config: &JsonValue) -> &JsonValue {
        config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|outbound| outbound["tag"] == "proxy")
            .unwrap()
    }

    #[test]
    fn shadowsocks_parser_ignores_fragment_and_plugin() {
        let user_info = general_purpose::STANDARD.encode("aes-256-gcm:secret");
        let url = format!(
            "ss://{}@example.com:8388?plugin=v2ray-plugin#US-01",
            user_info
        );

        let parsed = parse_shadowsocks_parts(&url).unwrap();
        assert_eq!(
            parsed,
            (
                "aes-256-gcm".to_string(),
                "secret".to_string(),
                "example.com".to_string(),
                8388,
            )
        );
    }

    #[test]
    fn shadowsocks_parser_supports_full_url_safe_base64_and_ipv6() {
        let encoded =
            general_purpose::URL_SAFE_NO_PAD.encode("aes-128-gcm:hello@[2001:db8::1]:443");
        let url = format!("ss://{}#node-name", encoded);

        let parsed = parse_shadowsocks_parts(&url).unwrap();
        assert_eq!(
            parsed,
            (
                "aes-128-gcm".to_string(),
                "hello".to_string(),
                "2001:db8::1".to_string(),
                443,
            )
        );
    }

    #[test]
    fn query_parser_decodes_percent_and_preserves_plus() {
        let params =
            parse_query_params(Some("path=%2Fws&obfs-password=a+b%2Fc&udp_over_stream")).unwrap();

        assert_eq!(params.get("path"), Some(&"/ws".to_string()));
        assert_eq!(params.get("obfs-password"), Some(&"a+b/c".to_string()));
        assert_eq!(params.get("udp_over_stream"), Some(&"".to_string()));
    }

    #[test]
    fn max_scheme_shadowsocks_outputs_valid_config() {
        let user_info = general_purpose::URL_SAFE_NO_PAD.encode("aes-256-gcm:secret");
        let url = format!(
            "ss://{}@ss.example.com:8388?plugin=v2ray-plugin%3Bmode%3Dwebsocket%3Bhost%3Dcdn.example.com%3Bpath%3D%2Fss&uot=1#Node",
            user_info
        );

        let config = run_and_check("ss-max", &url);
        let outbound = proxy_outbound(&config);
        assert_eq!(outbound["type"], "shadowsocks");
        assert_eq!(outbound["plugin"], "v2ray-plugin");
        assert_eq!(
            outbound["plugin_opts"],
            "mode=websocket;host=cdn.example.com;path=/ss"
        );
        assert_eq!(outbound["udp_over_tcp"], true);
    }

    #[test]
    fn max_scheme_vmess_outputs_valid_config() {
        let payload = json!({
            "v": "2",
            "ps": "vmess-node",
            "add": "vmess.example.com",
            "port": "443",
            "id": "11111111-1111-1111-1111-111111111111",
            "aid": "0",
            "scy": "auto",
            "net": "ws",
            "host": "ws.example.com",
            "path": "/vmess",
            "tls": "tls",
            "sni": "sni.example.com",
            "alpn": "h2,http/1.1",
            "fp": "chrome",
            "allowInsecure": "1",
            "packetEncoding": "xudp",
            "ed": "2048",
            "eh": "Sec-WebSocket-Protocol"
        });
        let encoded = general_purpose::STANDARD.encode(payload.to_string());
        let url = format!("vmess://{}#Node", encoded);

        let config = run_and_check("vmess-max", &url);
        let outbound = proxy_outbound(&config);
        assert_eq!(outbound["type"], "vmess");
        assert_eq!(outbound["network"], "tcp");
        assert_eq!(outbound["transport"]["type"], "ws");
        assert_eq!(outbound["transport"]["path"], "/vmess");
        assert_eq!(outbound["transport"]["max_early_data"], 2048);
        assert_eq!(outbound["tls"]["enabled"], true);
        assert_eq!(outbound["tls"]["utls"]["fingerprint"], "chrome");
        assert_eq!(outbound["packet_encoding"], "xudp");
    }

    #[test]
    fn max_scheme_trojan_outputs_valid_config() {
        let url = "trojan://secret@trojan.example.com:443?type=httpupgrade&host=upgrade.example.com&path=%2Ftrojan&sni=trojan-sni.example.com&alpn=h2%2Chttp%2F1.1&fp=chrome&allowInsecure=1#Node";

        let config = run_and_check("trojan-max", url);
        let outbound = proxy_outbound(&config);
        assert_eq!(outbound["type"], "trojan");
        assert_eq!(outbound["transport"]["type"], "httpupgrade");
        assert_eq!(outbound["transport"]["host"], "upgrade.example.com");
        assert_eq!(outbound["transport"]["path"], "/trojan");
        assert_eq!(outbound["tls"]["enabled"], true);
        assert_eq!(outbound["tls"]["server_name"], "trojan-sni.example.com");
    }

    #[test]
    fn max_scheme_vless_outputs_valid_config() {
        let url = "vless://22222222-2222-2222-2222-222222222222@vless.example.com:443?security=reality&sni=reality.example.com&pbk=4TnH0pyX1Jf8V8wA8rV5b6lOz7lLJtQ4QWlL4o4jP8Q&sid=0123456789abcdef&fp=chrome&alpn=h2%2Chttp%2F1.1&packetEncoding=xudp#Node";

        let config = run_and_check("vless-max", url);
        let outbound = proxy_outbound(&config);
        assert_eq!(outbound["type"], "vless");
        assert_eq!(outbound["tls"]["enabled"], true);
        assert_eq!(outbound["tls"]["reality"]["enabled"], true);
        assert_eq!(outbound["tls"]["reality"]["short_id"], "0123456789abcdef");
        assert_eq!(outbound["packet_encoding"], "xudp");
    }

    #[test]
    fn vless_reality_without_fingerprint_still_enables_utls() {
        let url = "vless://44444444-4444-4444-4444-444444444444@203.0.113.10:443/?type=tcp&encryption=none&sni=reality.example.com&security=reality&pbk=4TnH0pyX1Jf8V8wA8rV5b6lOz7lLJtQ4QWlL4o4jP8Q&sid=0123456789abcdef#Example";

        let config = run_and_check("vless-reality-no-fp", url);
        let outbound = proxy_outbound(&config);
        assert_eq!(outbound["type"], "vless");
        assert_eq!(outbound["tls"]["enabled"], true);
        assert_eq!(outbound["tls"]["server_name"], "reality.example.com");
        assert_eq!(outbound["tls"]["utls"]["enabled"], true);
        assert_eq!(outbound["tls"]["reality"]["enabled"], true);
        assert_eq!(
            outbound["tls"]["reality"]["public_key"],
            "4TnH0pyX1Jf8V8wA8rV5b6lOz7lLJtQ4QWlL4o4jP8Q"
        );
        assert_eq!(outbound["tls"]["reality"]["short_id"], "0123456789abcdef");
    }

    #[test]
    fn max_scheme_hysteria2_outputs_valid_config() {
        let url = "hy2://user%3Apass@hy2.example.com:8443,9000-9002?obfs=salamander&obfs-password=hy2-secret&sni=hy2.example.com&alpn=h3&insecure=1&network=udp#Node";

        let config = run_and_check("hy2-max", url);
        let outbound = proxy_outbound(&config);
        assert_eq!(outbound["type"], "hysteria2");
        assert_eq!(outbound["password"], "user:pass");
        assert_eq!(outbound["server_port"], 8443);
        assert_eq!(outbound["server_ports"][0], "9000:9002");
        assert_eq!(outbound["network"], "udp");
        assert_eq!(outbound["obfs"]["type"], "salamander");
        assert_eq!(outbound["tls"]["enabled"], true);
    }

    #[test]
    fn max_scheme_tuic_outputs_valid_config() {
        let url = "tuic://33333333-3333-3333-3333-333333333333:password@tuic.example.com:443?sni=tuic.example.com&alpn=h3&congestion_control=bbr&udp_over_stream=1&zero_rtt_handshake=1&heartbeat=10s&allow_insecure=1&network=udp&fp=chrome#Node";

        let config = run_and_check("tuic-max", url);
        let outbound = proxy_outbound(&config);
        assert_eq!(outbound["type"], "tuic");
        assert_eq!(outbound["udp_over_stream"], true);
        assert_eq!(outbound["zero_rtt_handshake"], true);
        assert_eq!(outbound["network"], "udp");
        assert_eq!(outbound["tls"]["utls"]["fingerprint"], "chrome");
    }

    #[test]
    fn max_scheme_anytls_outputs_valid_config() {
        let url = "anytls://secret@anytls.example.com:443?sni=anytls.example.com&alpn=h2%2Chttp%2F1.1&allowInsecure=1&fp=chrome&idle_session_check_interval=30s&idle_session_timeout=10m&min_idle_session=3#Node";

        let config = run_and_check("anytls-max", url);
        let outbound = proxy_outbound(&config);
        assert_eq!(outbound["type"], "anytls");
        assert_eq!(outbound["tls"]["enabled"], true);
        assert_eq!(outbound["idle_session_check_interval"], "30s");
        assert_eq!(outbound["min_idle_session"], 3);
    }
}
