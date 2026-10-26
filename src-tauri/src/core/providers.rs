use std::path::Path;

use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, Item, Table, Value};

use super::error::{KcError, Result};
use super::fs_ops;
use super::paths;
use super::write_guard;

/// capabilities 的已知取值；未知值读入时原样保留在 `other_capabilities`。
pub const KNOWN_CAPABILITIES: &[&str] = &[
    "thinking",
    "always_thinking",
    "image_in",
    "video_in",
    "tool_use",
    "dynamically_loaded_tools",
];

pub const KNOWN_EFFORTS: &[&str] = &["low", "medium", "high", "max"];

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelEntry {
    /// `[models."<id>"]` 的 key，通常形如 `provider/model`，可自定义
    pub id: String,
    pub provider: String,
    pub model: String,
    pub display_name: Option<String>,
    pub max_context_size: Option<i64>,
    pub max_output_size: Option<i64>,
    pub capabilities: Vec<String>,
    pub support_efforts: Vec<String>,
    pub default_effort: Option<String>,
    pub adaptive_thinking: Option<bool>,
    pub reasoning_key: Option<String>,
    /// 本工具不认识、但文件里存在的其它字段（原样保留，不在 UI 编辑）
    #[serde(skip)]
    pub unknown: Vec<String>,
    /// 内部标记：元数据是否来自 models.dev 目录（不落盘、不传给前端）
    #[serde(skip)]
    pub _used_catalog: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub name: String,
    pub base_url: Option<String>,
    pub api_key_masked: Option<String>,
    pub provider_type: Option<String>,
    pub managed: bool,
    pub models: Vec<ModelEntry>,
    /// 是否有可用密钥（供「获取模型列表」按钮判断可用性）
    pub has_api_key: bool,
    /// 真实密钥，仅供后端调用接口；不序列化给前端
    #[serde(skip_serializing)]
    pub api_key_value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvidersSnapshot {
    pub providers: Vec<Provider>,
    pub default_model: Option<String>,
    pub config_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpsert {
    pub name: String,
    /// 编辑已有 provider 时的原名；为空表示新增
    pub original_name: Option<String>,
    pub base_url: String,
    pub api_key: Option<String>,
    pub provider_type: Option<String>,
}

fn mask(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "*".repeat(chars.len().max(4));
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}****{tail}")
}

fn parse_document(text: &str) -> Result<DocumentMut> {
    text.parse::<DocumentMut>()
        .map_err(|e| KcError::Parse(format!("config.toml 解析失败: {e}")))
}

fn string_array(item: Option<&Item>) -> Vec<String> {
    item.and_then(|i| i.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn model_from_table(id: &str, t: &dyn toml_edit::TableLike) -> ModelEntry {
    let mut capabilities = string_array(t.get("capabilities"));
    // 保持文件里原有的顺序，不排序
    capabilities.dedup();
    let known_keys = [
        "provider",
        "model",
        "display_name",
        "max_context_size",
        "max_output_size",
        "capabilities",
        "support_efforts",
        "default_effort",
        "adaptive_thinking",
        "reasoning_key",
    ];
    let unknown = t
        .iter()
        .map(|(k, _)| k.to_string())
        .filter(|k| !known_keys.contains(&k.as_str()))
        .collect();

    ModelEntry {
        id: id.to_string(),
        provider: t
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        model: t
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        display_name: t
            .get("display_name")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        max_context_size: t.get("max_context_size").and_then(|v| v.as_integer()),
        max_output_size: t.get("max_output_size").and_then(|v| v.as_integer()),
        capabilities,
        support_efforts: string_array(t.get("support_efforts")),
        default_effort: t
            .get("default_effort")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        adaptive_thinking: t.get("adaptive_thinking").and_then(|v| v.as_bool()),
        reasoning_key: t
            .get("reasoning_key")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        unknown,
        _used_catalog: false,
    }
}

pub fn load_snapshot(path: &Path) -> Result<ProvidersSnapshot> {
    let text = fs_ops::read_to_string(path)?;
    if text.trim().is_empty() {
        return Ok(ProvidersSnapshot {
            providers: Vec::new(),
            default_model: None,
            config_path: path.display().to_string(),
        });
    }
    let doc = parse_document(&text)?;

    let mut models: Vec<ModelEntry> = Vec::new();
    if let Some(models_tbl) = doc.get("models").and_then(|i| i.as_table_like()) {
        for (id, item) in models_tbl.iter() {
            if let Some(t) = item.as_table_like() {
                models.push(model_from_table(id, t));
            }
        }
    }

    let mut providers: Vec<Provider> = Vec::new();
    if let Some(p_tbl) = doc.get("providers").and_then(|i| i.as_table_like()) {
        for (name, item) in p_tbl.iter() {
            let Some(t) = item.as_table_like() else {
                continue;
            };
            let api_key = t.get("api_key").and_then(|v| v.as_str());
            providers.push(Provider {
                name: name.to_string(),
                base_url: t.get("base_url").and_then(|v| v.as_str()).map(str::to_string),
                api_key_masked: api_key.map(mask),
                provider_type: t.get("type").and_then(|v| v.as_str()).map(str::to_string),
                managed: name.starts_with("managed:"),
                models: models
                    .iter()
                    .filter(|m| m.provider == name)
                    .cloned()
                    .collect(),
                has_api_key: api_key.map(|k| !k.trim().is_empty()).unwrap_or(false),
                api_key_value: api_key.unwrap_or_default().to_string(),
            });
        }
    }

    Ok(ProvidersSnapshot {
        providers,
        default_model: doc
            .get("default_model")
            .and_then(|i| i.as_str())
            .map(str::to_string),
        config_path: path.display().to_string(),
    })
}

pub fn snapshot() -> Result<ProvidersSnapshot> {
    load_snapshot(&paths::config_file()?)
}

fn is_managed(name: &str) -> bool {
    name.starts_with("managed:")
}

pub fn set_default_model(path: &Path, model: &str) -> Result<()> {
    let _guard = write_guard::lock();
    let (text, mtime) = write_guard::read_tracked(path)?;
    let mut doc = parse_document(&text)?;

    let exists = doc
        .get("models")
        .and_then(|i| i.as_table_like())
        .map(|t| t.contains_key(model))
        .unwrap_or(false);
    if !exists {
        return Err(KcError::NotFound(format!("模型条目不存在: {model}")));
    }
    doc["default_model"] = toml_edit::value(model);
    write_guard::write_if_unchanged(path, mtime, doc.to_string().as_bytes())
}

pub fn upsert_provider(path: &Path, input: &ProviderUpsert) -> Result<()> {
    if input.name.trim().is_empty() {
        return Err(KcError::Invalid("provider 名称不能为空".into()));
    }
    let target_name = input.name.trim();
    let original = input
        .original_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if is_managed(target_name) || original.map(is_managed).unwrap_or(false) {
        return Err(KcError::Invalid(
            "managed: 前缀的 provider 由 Kimi Code 托管，不可编辑".into(),
        ));
    }

    let _guard = write_guard::lock();
    let (text, mtime) = write_guard::read_tracked(path)?;
    let mut doc = parse_document(&text)?;

    if doc.get("providers").is_none() {
        doc["providers"] = Item::Table(Table::new());
    }
    let providers = doc["providers"]
        .as_table_like_mut()
        .ok_or_else(|| KcError::Parse("providers 不是表".into()))?;

    // 重命名：搬移整个表，并同步 models 里的 provider 字段
    let renamed_from = original.filter(|o| *o != target_name).map(str::to_string);
    if let Some(old) = &renamed_from {
        if providers.contains_key(target_name) {
            return Err(KcError::Invalid(format!(
                "已存在同名 provider: {target_name}"
            )));
        }
        let item = providers
            .remove(old)
            .ok_or_else(|| KcError::NotFound(format!("provider 不存在: {old}")))?;
        providers.insert(target_name, item);
    }

    if !providers.contains_key(target_name) {
        providers.insert(target_name, Item::Table(Table::new()));
    }
    let entry = providers
        .get_mut(target_name)
        .and_then(|i| i.as_table_like_mut())
        .ok_or_else(|| KcError::Parse(format!("provider 条目不是表: {target_name}")))?;

    entry.insert("base_url", toml_edit::value(input.base_url.trim()));
    if let Some(t) = input.provider_type.as_deref().filter(|s| !s.trim().is_empty()) {
        entry.insert("type", toml_edit::value(t.trim()));
    }
    if let Some(key) = input.api_key.as_deref().filter(|s| !s.trim().is_empty()) {
        entry.insert("api_key", toml_edit::value(key.trim()));
    }

    if let Some(old) = renamed_from {
        if let Some(models) = doc.get_mut("models").and_then(|i| i.as_table_like_mut()) {
            for (_, item) in models.iter_mut() {
                let Some(t) = item.as_table_like_mut() else {
                    continue;
                };
                if t.get("provider").and_then(|v| v.as_str()) == Some(old.as_str()) {
                    t.insert("provider", toml_edit::value(target_name));
                }
            }
        }
    }

    write_guard::write_if_unchanged(path, mtime, doc.to_string().as_bytes())
}

pub fn remove_provider(path: &Path, name: &str, cascade_models: bool) -> Result<()> {
    if is_managed(name) {
        return Err(KcError::Invalid("managed: 前缀的 provider 不可删除".into()));
    }
    let _guard = write_guard::lock();
    let (text, mtime) = write_guard::read_tracked(path)?;
    let mut doc = parse_document(&text)?;

    let providers = doc
        .get_mut("providers")
        .and_then(|i| i.as_table_like_mut())
        .ok_or_else(|| KcError::NotFound(format!("provider 不存在: {name}")))?;
    providers
        .remove(name)
        .ok_or_else(|| KcError::NotFound(format!("provider 不存在: {name}")))?;

    if cascade_models {
        let default_model = doc
            .get("default_model")
            .and_then(|i| i.as_str())
            .map(str::to_string);
        if let Some(models) = doc.get_mut("models").and_then(|i| i.as_table_like_mut()) {
            let to_remove: Vec<String> = models
                .iter()
                .filter(|(_, item)| {
                    item.as_table_like()
                        .and_then(|t| t.get("provider"))
                        .and_then(|v| v.as_str())
                        == Some(name)
                })
                .map(|(k, _)| k.to_string())
                .collect();
            for key in &to_remove {
                models.remove(key);
            }
            if let Some(dm) = default_model {
                if to_remove.iter().any(|m| m == &dm) {
                    doc.as_table_mut().remove("default_model");
                }
            }
        }
    }

    write_guard::write_if_unchanged(path, mtime, doc.to_string().as_bytes())
}

fn set_string_array(table: &mut dyn toml_edit::TableLike, key: &str, values: &[String]) {
    if values.is_empty() {
        table.remove(key);
        return;
    }
    let mut arr = Array::new();
    for v in values {
        arr.push(v.as_str());
    }
    table.insert(key, Item::Value(Value::Array(arr)));
}

fn set_optional_string(table: &mut dyn toml_edit::TableLike, key: &str, value: Option<&str>) {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        Some(v) => {
            table.insert(key, toml_edit::value(v));
        }
        None => {
            table.remove(key);
        }
    }
}

fn set_optional_int(table: &mut dyn toml_edit::TableLike, key: &str, value: Option<i64>) {
    match value.filter(|v| *v > 0) {
        Some(v) => {
            table.insert(key, toml_edit::value(v));
        }
        None => {
            table.remove(key);
        }
    }
}

/// 新增或编辑一个模型条目。`original_id` 非空且与 `entry.id` 不同时执行重命名。
pub fn upsert_model(path: &Path, entry: &ModelEntry, original_id: Option<&str>) -> Result<()> {
    let id = entry.id.trim();
    if id.is_empty() {
        return Err(KcError::Invalid("模型 ID 不能为空".into()));
    }
    if entry.provider.trim().is_empty() {
        return Err(KcError::Invalid("必须指定所属 provider".into()));
    }
    if entry.model.trim().is_empty() {
        return Err(KcError::Invalid("model 名不能为空".into()));
    }

    let _guard = write_guard::lock();
    let (text, mtime) = write_guard::read_tracked(path)?;
    let mut doc = parse_document(&text)?;

    let provider_exists = doc
        .get("providers")
        .and_then(|i| i.as_table_like())
        .map(|t| t.contains_key(entry.provider.trim()))
        .unwrap_or(false);
    if !provider_exists {
        return Err(KcError::NotFound(format!(
            "provider 不存在: {}",
            entry.provider
        )));
    }

    let default_model = doc
        .get("default_model")
        .and_then(|i| i.as_str())
        .map(str::to_string);

    if doc.get("models").is_none() {
        doc["models"] = Item::Table(Table::new());
    }
    let models = doc["models"]
        .as_table_like_mut()
        .ok_or_else(|| KcError::Parse("models 不是表".into()))?;

    // 重命名模型条目
    if let Some(old) = original_id
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != id)
    {
        if models.contains_key(id) {
            return Err(KcError::Invalid(format!("模型条目已存在: {id}")));
        }
        let item = models
            .remove(old)
            .ok_or_else(|| KcError::NotFound(format!("模型条目不存在: {old}")))?;
        models.insert(id, item);
    }

    if !models.contains_key(id) {
        models.insert(id, Item::Table(Table::new()));
    }
    let table = models
        .get_mut(id)
        .and_then(|i| i.as_table_like_mut())
        .ok_or_else(|| KcError::Parse(format!("模型条目不是表: {id}")))?;

    table.insert("provider", toml_edit::value(entry.provider.trim()));
    table.insert("model", toml_edit::value(entry.model.trim()));
    set_optional_string(table, "display_name", entry.display_name.as_deref());
    set_optional_int(table, "max_context_size", entry.max_context_size);
    set_optional_int(table, "max_output_size", entry.max_output_size);
    set_string_array(table, "capabilities", &entry.capabilities);
    set_string_array(table, "support_efforts", &entry.support_efforts);
    set_optional_string(table, "default_effort", entry.default_effort.as_deref());
    set_optional_string(table, "reasoning_key", entry.reasoning_key.as_deref());
    match entry.adaptive_thinking {
        Some(v) => {
            table.insert("adaptive_thinking", toml_edit::value(v));
        }
        None => {
            table.remove("adaptive_thinking");
        }
    }

    // 重命名时同步 default_model 指向
    if let (Some(dm), Some(old)) = (
        default_model,
        original_id.map(str::trim).filter(|s| *s != id),
    ) {
        if dm == old {
            doc["default_model"] = toml_edit::value(id);
        }
    }

    write_guard::write_if_unchanged(path, mtime, doc.to_string().as_bytes())
}

pub fn remove_model(path: &Path, id: &str) -> Result<()> {
    let _guard = write_guard::lock();
    let (text, mtime) = write_guard::read_tracked(path)?;
    let mut doc = parse_document(&text)?;
    let models = doc
        .get_mut("models")
        .and_then(|i| i.as_table_like_mut())
        .ok_or_else(|| KcError::NotFound(format!("模型条目不存在: {id}")))?;
    models
        .remove(id)
        .ok_or_else(|| KcError::NotFound(format!("模型条目不存在: {id}")))?;

    let is_default = doc.get("default_model").and_then(|i| i.as_str()) == Some(id);
    if is_default {
        doc.as_table_mut().remove("default_model");
    }
    write_guard::write_if_unchanged(path, mtime, doc.to_string().as_bytes())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelIssue {
    pub level: String,
    pub message: String,
}

/// 常见配置坑检查（只提示，不阻断保存）。
pub fn diagnose_model(entry: &ModelEntry) -> Vec<ModelIssue> {
    let mut out = Vec::new();
    let caps = &entry.capabilities;
    let id_lower = entry.id.to_lowercase();

    let looks_vision = {
        // 常见视觉模型的命名特征：vl / vision / image / multimodal / -4o / 结尾的 v
        let re_like = [
            "vision", "vl", "image", "multimodal", "gpt-4o", "gemini", "omni",
        ]
        .iter()
        .any(|k| id_lower.contains(k));
        // 形如 glm-4.5v / glm-4v / qwen-vl 的结尾 v
        let ends_with_v = id_lower
            .rsplit('/')
            .next()
            .map(|tail| tail.ends_with('v') && tail.len() > 1)
            .unwrap_or(false);
        re_like || ends_with_v
    };
    if looks_vision && !caps.iter().any(|c| c == "image_in") {
        out.push(ModelIssue {
            level: "warn".into(),
            message: "模型名看起来支持视觉，但未勾选 image_in，可能无法识别图片".into(),
        });
    }
    if caps.iter().any(|c| c == "always_thinking") && !caps.iter().any(|c| c == "thinking") {
        out.push(ModelIssue {
            level: "warn".into(),
            message: "勾了 always_thinking 但没勾 thinking，思考模式可能不生效".into(),
        });
    }
    if entry.default_effort.is_some() && entry.support_efforts.is_empty() {
        out.push(ModelIssue {
            level: "warn".into(),
            message: "填了 default_effort 但 support_efforts 为空，档位切换可能无效".into(),
        });
    }
    if let Some(de) = &entry.default_effort {
        if !entry.support_efforts.is_empty() && !entry.support_efforts.contains(de) {
            out.push(ModelIssue {
                level: "error".into(),
                message: format!("default_effort「{de}」不在 support_efforts 列表里"),
            });
        }
    }
    if entry.max_context_size.is_none() {
        out.push(ModelIssue {
            level: "info".into(),
            message: "未填 max_context_size，长上下文可能被默认值限制".into(),
        });
    }
    // 模型 ID 与 provider/model 结构不一致
    let expected_suffix = format!("/{}", entry.model.trim());
    if !entry.model.trim().is_empty() && !entry.id.ends_with(&expected_suffix) {
        out.push(ModelIssue {
            level: "info".into(),
            message: format!(
                "模型 ID 不以「{}」结尾，与 model 字段不一致（Kimi Code 通常按 provider/model 命名）",
                expected_suffix
            ),
        });
    }
    for c in caps {
        if !KNOWN_CAPABILITIES.contains(&c.as_str()) {
            out.push(ModelIssue {
                level: "info".into(),
                message: format!("capability「{c}」不在已知取值内，可能拼写有误"),
            });
        }
    }
    for e in &entry.support_efforts {
        if !KNOWN_EFFORTS.contains(&e.as_str()) {
            out.push(ModelIssue {
                level: "info".into(),
                message: format!("思考档位「{e}」不在已知取值内"),
            });
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaInfo {
    pub known_capabilities: Vec<String>,
    pub known_efforts: Vec<String>,
}

pub fn meta() -> MetaInfo {
    MetaInfo {
        known_capabilities: KNOWN_CAPABILITIES.iter().map(|s| s.to_string()).collect(),
        known_efforts: KNOWN_EFFORTS.iter().map(|s| s.to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::lock_data_dir;
    use std::fs;

    const SAMPLE: &str = r#"# 用户手写注释：请保留
default_model = "p1/m1"

[providers.p1]
# provider 注释
base_url = "https://a.example/v1"
type = "openai"
api_key = "sk-1234567890abcdef"

[providers."managed:kimi-code"]
type = "kimi"
base_url = "https://api.kimi.com/coding/v1"

[models."p1/m1"]
provider = "p1"
model = "m1"
display_name = "M1"
default_effort = "high"
max_context_size = 128000
capabilities = [ "thinking", "image_in" ]
support_efforts = [ "low", "high" ]

[models."p1/m2"]
provider = "p1"
model = "m2"
"#;

    fn fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("config.toml");
        fs::write(&p, SAMPLE).unwrap();
        (tmp, p)
    }

    #[test]
    fn snapshot_reads_full_model_fields() {
        let (_t, p) = fixture();
        let snap = load_snapshot(&p).unwrap();
        assert_eq!(snap.default_model.as_deref(), Some("p1/m1"));
        let p1 = snap.providers.iter().find(|x| x.name == "p1").unwrap();
        let m1 = p1.models.iter().find(|m| m.id == "p1/m1").unwrap();
        assert_eq!(m1.capabilities, vec!["thinking", "image_in"]);
        assert_eq!(m1.support_efforts, vec!["low", "high"]);
        assert_eq!(m1.default_effort.as_deref(), Some("high"));
        assert_eq!(m1.max_context_size, Some(128000));
        assert!(p1.api_key_masked.as_ref().unwrap().contains("****"));
    }

    #[test]
    fn upsert_model_creates_with_all_fields() {
        let _data = lock_data_dir();
        let (_t, p) = fixture();
        let entry = ModelEntry {
            id: "p1/m3".into(),
            provider: "p1".into(),
            model: "m3".into(),
            display_name: Some("M3 Vision".into()),
            max_context_size: Some(200000),
            max_output_size: Some(8192),
            capabilities: vec!["thinking".into(), "image_in".into(), "tool_use".into()],
            support_efforts: vec!["low".into(), "high".into(), "max".into()],
            default_effort: Some("high".into()),
            adaptive_thinking: Some(true),
            reasoning_key: None,
            unknown: vec![],
            _used_catalog: false,
        };
        upsert_model(&p, &entry, None).unwrap();

        let snap = load_snapshot(&p).unwrap();
        let m3 = snap
            .providers
            .iter()
            .find(|x| x.name == "p1")
            .unwrap()
            .models
            .iter()
            .find(|m| m.id == "p1/m3")
            .expect("m3 未写入");
        assert_eq!(m3.capabilities.len(), 3);
        assert!(m3.capabilities.contains(&"image_in".to_string()));
        assert_eq!(m3.support_efforts.len(), 3);
        assert_eq!(m3.adaptive_thinking, Some(true));
        assert_eq!(m3.max_output_size, Some(8192));

        let text = fs::read_to_string(&p).unwrap();
        assert!(text.contains("# 用户手写注释：请保留"), "注释丢失");
    }

    #[test]
    fn upsert_model_edits_existing() {
        let _data = lock_data_dir();
        let (_t, p) = fixture();
        let mut entry = load_snapshot(&p)
            .unwrap()
            .providers
            .iter()
            .find(|x| x.name == "p1")
            .unwrap()
            .models
            .iter()
            .find(|m| m.id == "p1/m1")
            .unwrap()
            .clone();
        entry.capabilities.push("video_in".into());
        entry.default_effort = Some("low".into());

        upsert_model(&p, &entry, Some("p1/m1")).unwrap();
        let snap = load_snapshot(&p).unwrap();
        let m1 = snap
            .providers
            .iter()
            .find(|x| x.name == "p1")
            .unwrap()
            .models
            .iter()
            .find(|m| m.id == "p1/m1")
            .unwrap();
        assert!(m1.capabilities.contains(&"video_in".to_string()));
        assert_eq!(m1.default_effort.as_deref(), Some("low"));
    }

    #[test]
    fn upsert_model_rename_keeps_default_model_in_sync() {
        let _data = lock_data_dir();
        let (_t, p) = fixture();
        let mut entry = load_snapshot(&p)
            .unwrap()
            .providers
            .iter()
            .find(|x| x.name == "p1")
            .unwrap()
            .models
            .iter()
            .find(|m| m.id == "p1/m1")
            .unwrap()
            .clone();
        entry.id = "p1/m1-renamed".into();

        upsert_model(&p, &entry, Some("p1/m1")).unwrap();
        let snap = load_snapshot(&p).unwrap();
        assert_eq!(snap.default_model.as_deref(), Some("p1/m1-renamed"));
        let ids: Vec<_> = snap
            .providers
            .iter()
            .find(|x| x.name == "p1")
            .unwrap()
            .models
            .iter()
            .map(|m| m.id.clone())
            .collect();
        assert!(ids.contains(&"p1/m1-renamed".to_string()));
        assert!(!ids.contains(&"p1/m1".to_string()));
    }

    #[test]
    fn upsert_model_rejects_unknown_provider() {
        let _data = lock_data_dir();
        let (_t, p) = fixture();
        let entry = ModelEntry {
            id: "nope/x".into(),
            provider: "nope".into(),
            model: "x".into(),
            ..Default::default()
        };
        assert!(upsert_model(&p, &entry, None).is_err());
    }

    #[test]
    fn provider_rename_updates_models() {
        let _data = lock_data_dir();
        let (_t, p) = fixture();
        upsert_provider(
            &p,
            &ProviderUpsert {
                name: "p1-renamed".into(),
                original_name: Some("p1".into()),
                base_url: "https://a.example/v1".into(),
                api_key: None,
                provider_type: Some("openai".into()),
            },
        )
        .unwrap();
        let snap = load_snapshot(&p).unwrap();
        let renamed = snap
            .providers
            .iter()
            .find(|x| x.name == "p1-renamed")
            .unwrap();
        assert_eq!(renamed.models.len(), 2);
        assert!(renamed.models.iter().all(|m| m.provider == "p1-renamed"));
        assert!(!snap.providers.iter().any(|x| x.name == "p1"));
    }

    #[test]
    fn remove_model_clears_default_if_needed() {
        let _data = lock_data_dir();
        let (_t, p) = fixture();
        remove_model(&p, "p1/m1").unwrap();
        let snap = load_snapshot(&p).unwrap();
        assert_eq!(snap.default_model, None, "被删除的默认模型应清空指向");
    }

    #[test]
    fn diagnose_flags_vision_without_image_in() {
        let entry = ModelEntry {
            id: "p/glm-4v".into(),
            provider: "p".into(),
            model: "glm-4v".into(),
            capabilities: vec!["thinking".into()],
            max_context_size: Some(1000),
            ..Default::default()
        };
        let issues = diagnose_model(&entry);
        assert!(issues.iter().any(|i| i.message.contains("image_in")));
    }

    #[test]
    fn diagnose_flags_effort_not_in_supported() {
        let entry = ModelEntry {
            id: "p/m".into(),
            provider: "p".into(),
            model: "m".into(),
            capabilities: vec![],
            support_efforts: vec!["low".into()],
            default_effort: Some("max".into()),
            max_context_size: Some(1),
            ..Default::default()
        };
        let issues = diagnose_model(&entry);
        assert!(issues.iter().any(|i| i.level == "error"));
    }

    #[test]
    fn diagnose_flags_context_missing() {
        let entry = ModelEntry {
            id: "p/m".into(),
            provider: "p".into(),
            model: "m".into(),
            ..Default::default()
        };
        let issues = diagnose_model(&entry);
        assert!(issues.iter().any(|i| i.message.contains("max_context_size")));
    }

    #[test]
    fn provider_upsert_rejects_managed() {
        let (_t, p) = fixture();
        let err = upsert_provider(
            &p,
            &ProviderUpsert {
                name: "managed:kimi-code".into(),
                original_name: None,
                base_url: "x".into(),
                api_key: None,
                provider_type: None,
            },
        );
        assert!(err.is_err());
    }

    #[test]
    fn provider_updates_base_url_preserving_key() {
        let _data = lock_data_dir();
        let (_t, p) = fixture();
        upsert_provider(
            &p,
            &ProviderUpsert {
                name: "p1".into(),
                original_name: Some("p1".into()),
                base_url: "https://new.example".into(),
                api_key: None,
                provider_type: None,
            },
        )
        .unwrap();
        let snap = load_snapshot(&p).unwrap();
        let p1 = snap.providers.iter().find(|x| x.name == "p1").unwrap();
        assert_eq!(p1.base_url.as_deref(), Some("https://new.example"));
        assert!(p1.api_key_masked.is_some(), "未传 key 时应保留原 key");
    }
}
