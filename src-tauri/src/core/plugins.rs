use serde::Serialize;
use serde_json::Value;

use std::path::Path;

use super::error::{KcError, Result};
use super::fs_ops;
use super::write_guard;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plugin {
    pub id: String,
    pub root: String,
    pub source: String,
    pub enabled: bool,
    pub original_source: Option<String>,
    pub installed_at: Option<String>,
    pub updated_at: Option<String>,
    pub ref_label: Option<String>,
}

fn read_root(path: &Path) -> Result<(Value, Option<std::time::SystemTime>)> {
    let (text, mtime) = write_guard::read_tracked(path)?;
    if text.trim().is_empty() {
        return Ok((serde_json::json!({ "version": 1, "plugins": [] }), mtime));
    }
    let v: Value = serde_json::from_str(&text)?;
    if !v.is_object() {
        return Err(KcError::Parse("installed.json 顶层必须是对象".into()));
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

fn plugin_array(root: &Value) -> Vec<Value> {
    root.get("plugins")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

pub fn list(path: &Path) -> Result<Vec<Plugin>> {
    let (root, _) = read_root(path)?;
    let mut out: Vec<Plugin> = plugin_array(&root)
        .into_iter()
        .filter_map(|p| {
            let id = p.get("id")?.as_str()?.to_string();
            let ref_label = p
                .get("github")
                .and_then(|g| g.get("ref"))
                .and_then(|r| r.get("value"))
                .and_then(|v| v.as_str())
                .map(str::to_string);
            Some(Plugin {
                id,
                root: p
                    .get("root")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                source: p
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                enabled: p.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                original_source: p
                    .get("originalSource")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                installed_at: p
                    .get("installedAt")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                updated_at: p
                    .get("updatedAt")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                ref_label,
            })
        })
        .collect();
    out.sort_by(|a, b| a.id.to_lowercase().cmp(&b.id.to_lowercase()));
    Ok(out)
}

pub fn set_enabled(path: &Path, id: &str, enabled: bool) -> Result<()> {
    let _guard = write_guard::lock();
    let (mut root, mtime) = read_root(path)?;
    let plugins = root
        .get_mut("plugins")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| KcError::NotFound("installed.json 没有 plugins 数组".into()))?;
    let entry = plugins
        .iter_mut()
        .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(id))
        .ok_or_else(|| KcError::NotFound(format!("插件不存在: {id}")))?;
    entry["enabled"] = Value::Bool(enabled);
    entry["updatedAt"] = Value::String(now_iso());
    write_root(path, &root, mtime)
}

pub fn remove(path: &Path, id: &str, delete_files: bool) -> Result<()> {
    let _guard = write_guard::lock();
    let (mut root, mtime) = read_root(path)?;
    let plugins = root
        .get_mut("plugins")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| KcError::NotFound("installed.json 没有 plugins 数组".into()))?;
    let idx = plugins
        .iter()
        .position(|p| p.get("id").and_then(|v| v.as_str()) == Some(id))
        .ok_or_else(|| KcError::NotFound(format!("插件不存在: {id}")))?;
    let removed = plugins.remove(idx);
    if delete_files {
        if let Some(root_dir) = removed.get("root").and_then(|v| v.as_str()) {
            let dir = Path::new(root_dir);
            if dir.is_dir() {
                fs_ops::move_to_trash(dir)?;
            }
        }
    }
    write_root(path, &root, mtime)
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // 简化：秒级时间戳的 ISO 近似（无需引入 chrono）
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{days}dT{h:02}:{m:02}:{s:02}Z")
}

pub fn list_default() -> Result<Vec<Plugin>> {
    list(&super::paths::plugins_file()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("installed.json");
        fs::write(
            &p,
            r#"{
  "version": 1,
  "plugins": [
    { "id": "alpha", "root": "R:/alpha", "source": "github", "enabled": true,
      "github": { "owner": "o", "repo": "r", "ref": { "kind": "tag", "value": "v1" } } },
    { "id": "beta", "root": "R:/beta", "source": "zip-url", "enabled": false }
  ]
}"#,
        )
        .unwrap();
        (tmp, p)
    }

    #[test]
    fn lists_plugins_with_ref() {
        let _data = crate::core::test_support::lock_data_dir();
        let (_t, p) = fixture();
        let list = super::list(&p).unwrap();
        assert_eq!(list.len(), 2);
        let a = list.iter().find(|x| x.id == "alpha").unwrap();
        assert_eq!(a.ref_label.as_deref(), Some("v1"));
        assert!(a.enabled);
        let b = list.iter().find(|x| x.id == "beta").unwrap();
        assert!(!b.enabled);
    }

    #[test]
    fn toggles_enabled() {
        let _data = crate::core::test_support::lock_data_dir();
        let (_t, p) = fixture();
        set_enabled(&p, "alpha", false).unwrap();
        let list = super::list(&p).unwrap();
        assert!(!list.iter().find(|x| x.id == "alpha").unwrap().enabled);
    }

    #[test]
    fn unknown_id_errors() {
        let _data = crate::core::test_support::lock_data_dir();
        let (_t, p) = fixture();
        assert!(set_enabled(&p, "nope", true).is_err());
        assert!(remove(&p, "nope", false).is_err());
    }

    #[test]
    fn remove_keeps_files_when_not_requested() {
        let _data = crate::core::test_support::lock_data_dir();
        let (_t, p) = fixture();
        remove(&p, "alpha", false).unwrap();
        let list = super::list(&p).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list.iter().all(|x| x.id != "alpha"));
    }
}
