//! 极简 HTTP GET：调用系统自带的 curl，避免引入 TLS 依赖。
//!
//! 为什么不用 reqwest/ureq：本项目要求尽量复用本机已有内容、少新增依赖，
//! 而 Windows 10+ 与 macOS/Linux 都自带 curl（带 TLS 支持）。
//!
//! 关键细节（都踩过坑）：
//! - **不能弹控制台**：GUI 程序调用控制台程序时 Windows 会新建控制台窗口，
//!   必须加 CREATE_NO_WINDOW，否则用户会看到一个黑框。
//! - **不能阻塞 UI**：命令处理函数要 async + spawn_blocking，否则窗口会「未响应」。
//! - **代理**：很多用户的供应商域名需要走代理。这里读 Windows 系统代理
//!   以及常见环境变量，自动把 --proxy 传给 curl；连不上时再直连重试一次。

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub stderr: String,
}

/// 代理解析结果，用于给 UI 展示「本次请求走了什么网络路径」
#[derive(Debug, Clone)]
pub struct ProxyInfo {
    pub url: String,
    pub source: String,
}

fn curl_binary() -> PathBuf {
    // 优先用绝对路径，避免 PATH 里被别的 curl 抢先（如 Git 自带的 MSYS 版）
    if cfg!(windows) {
        let sys = PathBuf::from(r"C:\Windows\System32\curl.exe");
        if sys.exists() {
            return sys;
        }
        return PathBuf::from("curl.exe");
    }
    PathBuf::from("curl")
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_window(_cmd: &mut Command) {}

/// 读注册表里的系统代理（Windows）。
#[cfg(windows)]
fn windows_system_proxy() -> Option<String> {
    let out = {
        let mut c = Command::new("reg.exe");
        c.args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            "ProxyServer",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
        no_window(&mut c);
        c.output().ok()?
    };
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    // ProxyServer 可能是 "host:port" 或 "http=host:port;https=host:port"
    let value = text
        .lines()
        .find(|l| l.contains("ProxyServer"))?
        .split_whitespace()
        .last()?
        .to_string();

    let pick = if value.contains('=') {
        value
            .split(';')
            .find(|p| p.starts_with("https="))
            .or_else(|| value.split(';').find(|p| p.starts_with("http=")))
            .map(|p| p.split_once('=').map(|(_, v)| v).unwrap_or(p))
            .unwrap_or("")
            .to_string()
    } else {
        value
    };
    if pick.trim().is_empty() {
        None
    } else if pick.starts_with("http") {
        Some(pick)
    } else {
        Some(format!("http://{pick}"))
    }
}

#[cfg(not(windows))]
fn windows_system_proxy() -> Option<String> {
    None
}

/// 找出应该用的代理：环境变量优先，其次系统代理。
pub fn detect_proxy() -> Option<ProxyInfo> {
    for var in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
        if let Ok(v) = std::env::var(var) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(ProxyInfo {
                    url: v.to_string(),
                    source: format!("环境变量 {var}"),
                });
            }
        }
    }
    windows_system_proxy().map(|url| ProxyInfo {
        url,
        source: "Windows 系统代理".into(),
    })
}

fn base_args(timeout: Duration) -> Vec<String> {
    vec![
        "-sS".into(),
        "--max-time".into(),
        timeout.as_secs().max(1).to_string(),
        // 连接阶段单独限时，避免「连不上但要等满整个超时」
        "--connect-timeout".into(),
        "10".into(),
    ]
}

fn run_curl(args: &[String]) -> Result<(u16, String, String), String> {
    let mut cmd = Command::new(curl_binary());
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    no_window(&mut cmd);

    let output = cmd
        .output()
        .map_err(|e| format!("无法执行 curl（{e}）；请确认系统已安装 curl"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // stdout 末尾是 -w 输出的内容。取最后一段非空的 token 当状态码：
    // - 普通模式：body + "\n" + "200"
    // - 下载模式（-o 文件）：只有 "200"（没有换行）
    let trimmed = stdout.trim_end();
    let last_line = trimmed.rsplit('\n').next().unwrap_or("").trim();
    let status = last_line.parse::<u16>().unwrap_or(0);

    let body = if trimmed.len() > last_line.len() {
        trimmed[..trimmed.len() - last_line.len()]
            .trim_end_matches('\n')
            .to_string()
    } else {
        String::new()
    };

    Ok((status, body, stderr))
}

/// 把常见的 curl 错误翻译成用户能懂的话。
fn friendly_error(stderr: &str, via_proxy: Option<&ProxyInfo>) -> String {
    let s = stderr.trim();
    let hint = if s.contains("Could not connect") || s.contains("Failed to connect") {
        if let Some(p) = via_proxy {
            format!(
                "无法连接。当前已尝试通过代理 {}（{}）访问；\
                 如果该地址不该走代理，请在代理软件里配置直连规则，或关闭代理后重试。",
                p.url, p.source
            )
        } else {
            "无法连接。该域名可能被墙或需要代理：请开启代理软件后重试（应用会自动读取系统代理）。"
                .to_string()
        }
    } else if s.contains("timed out") || s.contains("Operation timeout") {
        "请求超时。网络较慢或该地址不可达，可在代理开启后重试。".to_string()
    } else if s.contains("SSL") || s.contains("certificate") {
        format!("TLS 握手失败：{s}。若使用了代理软件，请检查其证书设置。")
    } else if s.contains("resolve") || s.contains("Could not resolve") {
        "域名解析失败：请检查网络或 DNS 设置。".to_string()
    } else {
        format!("请求失败：{s}")
    };
    hint
}

/// 发一个 GET。会依次尝试：直连 → 系统代理（或反之），全部失败才报错。
pub fn get(url: &str, headers: &[(&str, &str)], timeout: Duration) -> Result<(HttpResponse, Vec<String>), String> {
    let mut notes: Vec<String> = Vec::new();
    let proxy = detect_proxy();

    // 尝试顺序：有代理先走代理（多数需要代理的域名这样一次就成功），再试直连
    let mut attempts: Vec<Option<&ProxyInfo>> = Vec::new();
    if let Some(p) = proxy.as_ref() {
        attempts.push(Some(p));
        attempts.push(None);
    } else {
        attempts.push(None);
    }

    let mut last_err = String::new();
    for (i, attempt) in attempts.iter().enumerate() {
        let mut args = base_args(timeout);
        args.push("-w".into());
        args.push("\n%{http_code}".into());
        if let Some(p) = attempt {
            args.push("--proxy".into());
            args.push(p.url.clone());
        }
        for (k, v) in headers {
            args.push("-H".into());
            args.push(format!("{k}: {v}"));
        }
        args.push(url.to_string());

        match run_curl(&args) {
            Ok((0, _, stderr)) => {
                last_err = friendly_error(&stderr, *attempt);
                let label = attempt
                    .map(|p| format!("代理 {}", p.url))
                    .unwrap_or_else(|| "直连".into());
                notes.push(format!("{label} 失败：{stderr}"));
            }
            Ok((status, body, stderr)) => {
                let label = attempt
                    .map(|p| format!("经代理 {}", p.url))
                    .unwrap_or_else(|| "直连".into());
                notes.push(format!("{label} 成功（HTTP {status}）"));
                return Ok((
                    HttpResponse {
                        status,
                        body,
                        stderr,
                    },
                    notes,
                ));
            }
            Err(e) => {
                last_err = e;
            }
        }
        if i + 1 < attempts.len() {
            notes.push("改用另一种网络路径重试…".into());
        }
    }

    Err(last_err)
}

/// 下载到文件（用于 models.dev 目录这类大响应）
pub fn get_to_file(
    url: &str,
    headers: &[(&str, &str)],
    dest: &Path,
    timeout: Duration,
) -> Result<u16, String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let proxy = detect_proxy();
    let mut attempts: Vec<Option<&ProxyInfo>> = Vec::new();
    if let Some(p) = proxy.as_ref() {
        attempts.push(Some(p));
        attempts.push(None);
    } else {
        attempts.push(None);
    }

    let mut last_err = String::new();
    for (i, attempt) in attempts.iter().enumerate() {
        let mut args = base_args(timeout);
        args.push("-L".into());
        args.push("-o".into());
        args.push(dest.display().to_string());
        args.push("-w".into());
        args.push("%{http_code}".into());
        if let Some(p) = attempt {
            args.push("--proxy".into());
            args.push(p.url.clone());
        }
        for (k, v) in headers {
            args.push("-H".into());
            args.push(format!("{k}: {v}"));
        }
        args.push(url.to_string());

        match run_curl(&args) {
            Ok((0, _, stderr)) => {
                last_err = friendly_error(&stderr, *attempt);
            }
            Ok((status, _, _)) => return Ok(status),
            Err(e) => last_err = e,
        }
        if i + 1 < attempts.len() {
            continue;
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_host_returns_error_not_panic() {
        let r = get(
            "http://127.0.0.1:1/definitely-not-listening",
            &[],
            Duration::from_secs(3),
        );
        assert!(r.is_err(), "连不上时应返回错误");
        let msg = r.unwrap_err();
        assert!(!msg.is_empty(), "错误信息不应为空");
    }

    #[test]
    fn friendly_error_mentions_proxy_when_used() {
        let p = ProxyInfo {
            url: "http://127.0.0.1:7897".into(),
            source: "测试".into(),
        };
        let msg = friendly_error(
            "curl: (28) Failed to connect to x port 443 after 21064 ms: Could not connect to server",
            Some(&p),
        );
        assert!(msg.contains("127.0.0.1:7897"), "应提示用过的代理地址");
    }

    #[test]
    fn parses_status_from_body_and_from_stdout_only() {
        // 普通模式：body + 换行 + 状态码
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("out.txt");
        let _ = get_to_file(
            "http://127.0.0.1:1/x",
            &[],
            &f,
            std::time::Duration::from_secs(1),
        );
        // 只验证解析辅助逻辑：模拟两种 stdout 形态
        let parse = |stdout: &str| -> (u16, String) {
            let trimmed = stdout.trim_end();
            let last = trimmed.rsplit('\n').next().unwrap_or("").trim();
            let status = last.parse::<u16>().unwrap_or(0);
            let body = if trimmed.len() > last.len() {
                trimmed[..trimmed.len() - last.len()]
                    .trim_end_matches('\n')
                    .to_string()
            } else {
                String::new()
            };
            (status, body)
        };
        let (s1, b1) = parse("{\"a\":1}\n200");
        assert_eq!(s1, 200);
        assert_eq!(b1, "{\"a\":1}");
        let (s2, b2) = parse("200");
        assert_eq!(s2, 200);
        assert_eq!(b2, "", "下载模式下 stdout 只有状态码");
    }

    #[test]
    fn friendly_error_suggests_proxy_when_direct() {
        let msg = friendly_error(
            "curl: (28) Failed to connect to x port 443: Could not connect to server",
            None,
        );
        assert!(msg.contains("代理"), "直连失败应提示开启代理");
    }
}
