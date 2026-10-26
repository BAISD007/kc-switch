use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{KcError, Result};
use super::write_guard;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub bearer_token_env_var: Option<String>,
    pub env_keys: Vec<String>,
    pub deferred: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpUpsert {
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub bearer_token_env_var: Option<String>,
    pub env: Vec<(String, String)>,
    pub deferred: bool,
}

fn read_root(path: &Path) -> Result<(Value, Option<std::time::SystemTime>)> {
    let (text, mtime) = write_guard::read_tracked(path)?;
    if text.trim().is_empty() {
        return Ok((serde_json::json!({ "mcpServers": {} }), mtime));
    }
    let v: Value = serde_json::from_str(&text)?;
    if !v.is_object() {
        return Err(KcError::Parse("mcp.json 顶层必须是对象".into()));
    }
    Ok((v, mtime))
}

fn write_root(
    path: &Path,
    root: &Value,
    mtime: Option<std::time::SystemTime>,
) -> Result<()> {
    let pretty = serde_json::to_string_pretty(root)?;
    write_guard::write_if_unchanged(path, mtime, format!("{pretty}\n").as_bytes())
}

pub fn list(path: &Path) -> Result<Vec<McpServer>> {
    let (root, _) = read_root(path)?;
    let Some(servers) = root.get("mcpServers").and_then(|v| v.as_object()) else {
        return Ok(Vec::new());
    };
    let mut out: Vec<McpServer> = servers
        .iter()
        .map(|(name, cfg)| {
            let url = cfg.get("url").and_then(|v| v.as_str()).map(str::to_string);
            let transport = if url.is_some() { "http" } else { "stdio" };
            McpServer {
                name: name.clone(),
                transport: transport.into(),
                command: cfg
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                args: cfg
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                url,
                bearer_token_env_var: cfg
                    .get("bearerTokenEnvVar")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                env_keys: cfg
                    .get("env")
                    .and_then(|v| v.as_object())
                    .map(|o| o.keys().cloned().collect())
                    .unwrap_or_default(),
                deferred: cfg
                    .get("deferred")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn upsert(path: &Path, input: &McpUpsert) -> Result<()> {
    let _guard = write_guard::lock();
    let name = input.name.trim();
    if name.is_empty() {
        return Err(KcError::Invalid("MCP 名称不能为空".into()));
    }
    let (mut root, mtime) = read_root(path)?;
    if root.get("mcpServers").is_none() {
        root["mcpServers"] = serde_json::json!({});
    }

    let mut entry = serde_json::Map::new();
    match input.transport.as_str() {
        "http" => {
            let url = input
                .url
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| KcError::Invalid("HTTP 类型必须填写 url".into()))?;
            entry.insert("url".into(), Value::String(url.to_string()));
            if let Some(var) = input
                .bearer_token_env_var
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                entry.insert("bearerTokenEnvVar".into(), Value::String(var.to_string()));
            }
            if !input.env.is_empty() {
                let map: serde_json::Map<String, Value> = input
                    .env
                    .iter()
                    .filter(|(k, _)| !k.trim().is_empty())
                    .map(|(k, v)| (k.trim().to_string(), Value::String(v.clone())))
                    .collect();
                if !map.is_empty() {
                    entry.insert("headers".into(), Value::Object(map));
                }
            }
        }
        _ => {
            let cmd = input
                .command
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| KcError::Invalid("stdio 类型必须填写 command".into()))?;
            entry.insert("command".into(), Value::String(cmd.to_string()));
            let args: Vec<Value> = input
                .args
                .iter()
                .filter(|a| !a.is_empty())
                .map(|a| Value::String(a.clone()))
                .collect();
            if !args.is_empty() {
                entry.insert("args".into(), Value::Array(args));
            }
            if !input.env.is_empty() {
                let map: serde_json::Map<String, Value> = input
                    .env
                    .iter()
                    .filter(|(k, _)| !k.trim().is_empty())
                    .map(|(k, v)| (k.trim().to_string(), Value::String(v.clone())))
                    .collect();
                if !map.is_empty() {
                    entry.insert("env".into(), Value::Object(map));
                }
            }
        }
    }
    if input.deferred {
        entry.insert("deferred".into(), Value::Bool(true));
    }

    root["mcpServers"][name] = Value::Object(entry);
    write_root(path, &root, mtime)
}

pub fn remove(path: &Path, name: &str) -> Result<()> {
    let _guard = write_guard::lock();
    let (mut root, mtime) = read_root(path)?;
    let servers = root
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| KcError::NotFound(format!("MCP 不存在: {name}")))?;
    servers
        .remove(name)
        .ok_or_else(|| KcError::NotFound(format!("MCP 不存在: {name}")))?;
    write_root(path, &root, mtime)
}

pub fn set_deferred(path: &Path, name: &str, deferred: bool) -> Result<()> {
    let _guard = write_guard::lock();
    let (mut root, mtime) = read_root(path)?;
    let servers = root
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| KcError::NotFound(format!("MCP 不存在: {name}")))?;
    let entry = servers
        .get_mut(name)
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| KcError::NotFound(format!("MCP 不存在: {name}")))?;
    if deferred {
        entry.insert("deferred".into(), Value::Bool(true));
    } else {
        entry.remove("deferred");
    }
    write_root(path, &root, mtime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("mcp.json");
        fs::write(
            &p,
            r#"{
  "mcpServers": {
    "a": { "command": "npx", "args": ["-y", "x"] },
    "b": { "url": "https://x", "bearerTokenEnvVar": "TOK", "deferred": true }
  }
}"#,
        )
        .unwrap();
        (tmp, p)
    }

    #[test]
    fn lists_stdio_and_http() {
        let _data = crate::core::test_support::lock_data_dir();
        let (_t, p) = fixture();
        let list = super::list(&p).unwrap();
        assert_eq!(list.len(), 2);
        let b = list.iter().find(|s| s.name == "b").unwrap();
        assert_eq!(b.transport, "http");
        assert!(b.deferred);
        let a = list.iter().find(|s| s.name == "a").unwrap();
        assert_eq!(a.transport, "stdio");
        assert_eq!(a.args, vec!["-y", "x"]);
    }

    #[test]
    fn upsert_adds_stdio() {
        let _data = crate::core::test_support::lock_data_dir();
        let (_t, p) = fixture();
        upsert(
            &p,
            &McpUpsert {
                name: "c".into(),
                transport: "stdio".into(),
                command: Some("uvx".into()),
                args: vec!["blender-mcp".into()],
                url: None,
                bearer_token_env_var: None,
                env: vec![],
                deferred: true,
            },
        )
        .unwrap();
        let list = super::list(&p).unwrap();
        let c = list.iter().find(|s| s.name == "c").unwrap();
        assert_eq!(c.command.as_deref(), Some("uvx"));
        assert!(c.deferred);
    }

    #[test]
    fn upsert_requires_url_for_http() {
        let _data = crate::core::test_support::lock_data_dir();
        let (_t, p) = fixture();
        let r = upsert(
            &p,
            &McpUpsert {
                name: "z".into(),
                transport: "http".into(),
                command: None,
                args: vec![],
                url: Some("  ".into()),
                bearer_token_env_var: None,
                env: vec![],
                deferred: false,
            },
        );
        assert!(r.is_err());
    }

    #[test]
    fn toggle_deferred_and_remove() {
        let _data = crate::core::test_support::lock_data_dir();
        let (_t, p) = fixture();
        set_deferred(&p, "a", true).unwrap();
        let list = super::list(&p).unwrap();
        assert!(list.iter().find(|s| s.name == "a").unwrap().deferred);

        set_deferred(&p, "a", false).unwrap();
        let list = super::list(&p).unwrap();
        assert!(!list.iter().find(|s| s.name == "a").unwrap().deferred);

        remove(&p, "b").unwrap();
        assert_eq!(super::list(&p).unwrap().len(), 1);
    }
}
