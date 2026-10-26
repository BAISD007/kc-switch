//! 模型发现编排：拉取 `/models` → 用 models.dev 补元数据 → 转成可导入的模型条目。

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::discovery::{
    catalog_capabilities, catalog_efforts, catalog_reasoning_key, match_catalog_provider,
    parse_models_response, CatalogProvider, RemoteModel,
};
use super::error::{KcError, Result};
use super::http;
use super::paths;
use super::providers::{ModelEntry, Provider};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredModel {
    /// 来自供应商接口的模型 id
    pub remote_id: String,
    pub display_name: String,
    /// 已带好元数据的建议条目；用户可在 UI 里调整后导入
    pub entry: ModelEntry,
    /// 该模型是否已存在于配置中（按 model 字段比对）
    pub exists: bool,
    /// 元数据来源：catalog 表示已用 models.dev 补全
    pub metadata_ok: bool,
    /// 该模型上下文等参数来自哪里，用于 UI 说明
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverResult {
    pub models: Vec<DiscoveredModel>,
    /// 诊断信息：接口是否成功、目录是否匹配上
    pub notes: Vec<String>,
}

fn cache_dir() -> Result<std::path::PathBuf> {
    Ok(paths::data_dir()?.join("cache"))
}

fn catalog_path() -> Result<std::path::PathBuf> {
    Ok(cache_dir()?.join("models.dev.json"))
}

/// 目录缓存有效期：7 天
const CATALOG_TTL_SECS: u64 = 7 * 24 * 3600;

fn catalog_is_fresh(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|age| age.as_secs() < CATALOG_TTL_SECS)
        .unwrap_or(false)
}

/// 载入 models.dev 目录（带磁盘缓存）。
pub fn load_catalog(force_refresh: bool) -> Result<HashMap<String, CatalogProvider>> {
    let path = catalog_path()?;
    let mut notes = Vec::new();

    if force_refresh || !catalog_is_fresh(&path) {
        match http::get_to_file(
            "https://models.dev/api.json",
            &[("User-Agent", "kc-switch/0.1")],
            &path,
            Duration::from_secs(45),
        ) {
            Ok(200) => {}
            Ok(code) => notes.push(format!("模型目录下载返回 HTTP {code}，将使用本地缓存（如有）")),
            Err(e) => notes.push(format!("模型目录下载失败（{e}），将使用本地缓存（如有）")),
        }
    }

    if !path.exists() {
        return Err(KcError::Invalid(
            "无法获取 models.dev 模型目录，且本地没有缓存。请检查网络后重试。".into(),
        ));
    }

    let text = std::fs::read_to_string(&path)?;
    let catalog: HashMap<String, CatalogProvider> = serde_json::from_str(&text)
        .map_err(|e| KcError::Parse(format!("模型目录解析失败: {e}")))?;
    if catalog.is_empty() {
        return Err(KcError::Invalid("模型目录为空".into()));
    }
    Ok(catalog)
}

/// 把远程模型 + 目录元数据合成一个可直接保存的 ModelEntry。
fn build_entry(
    provider: &Provider,
    remote: &RemoteModel,
    catalog_model: Option<&super::discovery::CatalogModel>,
) -> ModelEntry {
    let mut capabilities: Vec<String> = Vec::new();
    let mut efforts: Vec<String> = Vec::new();
    let mut default_effort: Option<String> = None;
    let mut context: Option<i64> = remote.context_window;
    let mut max_output: Option<i64> = remote.max_output_tokens;
    let mut display: Option<String> = remote.display_name.clone();
    let mut reasoning_key: Option<String> = None;
    let mut used_catalog = false;

    // 1) 先取接口直接给出的能力
    for m in &remote.input_modalities {
        match m.as_str() {
            "image" => capabilities.push("image_in".into()),
            "video" => capabilities.push("video_in".into()),
            _ => {}
        }
    }
    if remote.reasoning == Some(true) {
        capabilities.push("thinking".into());
    }
    if !remote.reasoning_levels.is_empty() {
        efforts = remote.reasoning_levels.clone();
        default_effort = remote.default_reasoning_level.clone();
    }

    // 2) 再用 models.dev 目录补齐接口没给的
    if let Some(cm) = catalog_model {
        let cat_caps = catalog_capabilities(cm);
        for c in cat_caps {
            if !capabilities.contains(&c) {
                capabilities.push(c);
            }
        }
        if efforts.is_empty() {
            let (levels, def) = catalog_efforts(cm);
            if !levels.is_empty() {
                efforts = levels;
                default_effort = default_effort.or(def);
            }
        }
        if context.is_none() {
            context = cm.limit.as_ref().and_then(|l| l.context);
        }
        if max_output.is_none() {
            max_output = cm.limit.as_ref().and_then(|l| l.output);
        }
        if display.as_deref().unwrap_or("").is_empty() {
            display = cm.name.clone();
        }
        reasoning_key = catalog_reasoning_key(cm);
        used_catalog = true;
    }

    // 有思考能力但没档位时，给一组保守默认，避免用户忘了配
    if capabilities.iter().any(|c| c == "thinking") && efforts.is_empty() {
        efforts = vec!["low".into(), "high".into()];
    }
    if default_effort.is_none() && !efforts.is_empty() {
        default_effort = efforts
            .iter()
            .find(|e| e.as_str() == "high")
            .cloned()
            .or_else(|| efforts.last().cloned());
    }

    capabilities.sort();
    capabilities.dedup();

    let model_name = remote.id.clone();
    let provider_name = provider.name.clone();
    let id = format!("{provider_name}/{model_name}");

    ModelEntry {
        id,
        provider: provider_name,
        model: model_name,
        display_name: display.filter(|d| !d.trim().is_empty()),
        max_context_size: context,
        max_output_size: max_output,
        capabilities,
        support_efforts: efforts,
        default_effort,
        adaptive_thinking: None,
        reasoning_key,
        unknown: vec![],
        _used_catalog: used_catalog,
    }
}

/// 拉取某个 provider 的模型列表，并用目录补全元数据。
pub fn discover(
    provider: &Provider,
    existing: &[ModelEntry],
    force_catalog_refresh: bool,
) -> Result<DiscoverResult> {
    let base_url = provider
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| KcError::Invalid("该 provider 没有 base_url，无法拉取模型".into()))?;

    if !provider.has_api_key {
        return Err(KcError::Invalid(
            "该 provider 没有可用的 API Key，无法拉取模型".into(),
        ));
    }

    let endpoint = format!("{}/models", base_url.trim_end_matches('/'));
    let mut notes = vec![format!("请求 {endpoint}")];

    let (resp, net_notes) = http::get(
        &endpoint,
        &[
            ("Authorization", &format!("Bearer {}", provider.api_key_value)),
            ("Content-Type", "application/json"),
            ("User-Agent", "kc-switch/0.1"),
        ],
        Duration::from_secs(25),
    )
    .map_err(|e| KcError::Io(e))?;
    notes.extend(net_notes);

    if resp.status == 401 || resp.status == 403 {
        return Err(KcError::Invalid(format!(
            "认证失败（HTTP {}），请检查该 provider 的 API Key",
            resp.status
        )));
    }
    if !(200..300).contains(&resp.status) {
        let snippet: String = resp.body.chars().take(200).collect();
        return Err(KcError::Io(format!(
            "接口返回 HTTP {}：{}",
            resp.status, snippet
        )));
    }

    let remotes = parse_models_response(&resp.body)
        .map_err(|e| KcError::Parse(format!("解析模型列表失败: {e}")))?;
    notes.push(format!("接口返回 {} 个模型", remotes.len()));

    // 目录：匹配 provider
    let catalog_model_map = match load_catalog(force_catalog_refresh) {
        Ok(catalog) => match match_catalog_provider(&catalog, base_url) {
            Some((cat_id, cp)) => {
                notes.push(format!(
                    "已匹配 models.dev 目录「{}」，可补全上下文与能力参数",
                    cp.name.clone().unwrap_or_else(|| cat_id.to_string())
                ));
                Some(cp.models.clone())
            }
            None => {
                notes.push(
                    "models.dev 目录里没有匹配该 base_url 的供应商，将只用接口返回的信息"
                        .into(),
                );
                None
            }
        },
        Err(e) => {
            notes.push(format!("{e}"));
            None
        }
    };

    let models = remotes
        .iter()
        .map(|r| {
            // 目录里的 key 通常等于模型 id，也可能带前缀
            let cm = catalog_model_map.as_ref().and_then(|m| {
                m.get(&r.id).or_else(|| {
                    let short = r.id.rsplit('/').next().unwrap_or(&r.id);
                    m.get(short)
                })
            });
            let entry = build_entry(provider, r, cm);
            let exists = existing
                .iter()
                .any(|e| e.provider == provider.name && e.model == r.id);
            DiscoveredModel {
                remote_id: r.id.clone(),
                display_name: entry
                    .display_name
                    .clone()
                    .unwrap_or_else(|| r.id.clone()),
                metadata_ok: entry._used_catalog
                    || entry.max_context_size.is_some()
                    || !entry.capabilities.is_empty(),
                source: if entry._used_catalog && entry.max_context_size.is_some() {
                    "catalog".into()
                } else if entry.max_context_size.is_some() || !entry.capabilities.is_empty() {
                    "api".into()
                } else {
                    "none".into()
                },
                exists,
                entry,
            }
        })
        .collect();

    Ok(DiscoverResult { models, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> Provider {
        let mut p = Provider {
            name: "test".into(),
            base_url: Some("https://api.example.com/v1".into()),
            api_key_masked: Some("sk-****abcd".into()),
            provider_type: Some("openai".into()),
            managed: false,
            models: vec![],
            has_api_key: true,
            api_key_value: String::new(),
        };
        p.api_key_value = "sk-realkey".into();
        p
    }

    #[test]
    fn builds_entry_from_api_fields() {
        let p = provider();
        let remote = RemoteModel {
            id: "deepseek-flash".into(),
            display_name: Some("DeepSeek Flash".into()),
            context_window: Some(1048576),
            max_output_tokens: Some(393216),
            input_modalities: vec!["text".into(), "image".into()],
            output_modalities: vec!["text".into()],
            reasoning_levels: vec!["low".into(), "high".into(), "max".into()],
            default_reasoning_level: Some("high".into()),
            reasoning: Some(true),
            source: "api".into(),
        };
        let e = build_entry(&p, &remote, None);
        assert_eq!(e.id, "test/deepseek-flash");
        assert_eq!(e.model, "deepseek-flash");
        assert_eq!(e.max_context_size, Some(1048576));
        assert!(e.capabilities.contains(&"image_in".to_string()));
        assert!(e.capabilities.contains(&"thinking".to_string()));
        assert_eq!(e.support_efforts, vec!["low", "high", "max"]);
        assert_eq!(e.default_effort.as_deref(), Some("high"));
    }

    #[test]
    fn catalog_fills_missing_context() {
        let p = provider();
        let remote = RemoteModel {
            id: "glm-5.3".into(),
            ..Default::default()
        };
        let cm = super::super::discovery::CatalogModel {
            name: Some("GLM-5.3".into()),
            reasoning: true,
            tool_call: true,
            attachment: false,
            reasoning_options: vec![serde_json::json!({
                "type":"effort","values":["low","high","max"]
            })],
            modalities: Some(super::super::discovery::Modalities {
                input: vec!["text".into(), "image".into()],
                output: vec!["text".into()],
            }),
            limit: Some(super::super::discovery::Limit {
                context: Some(204800),
                output: Some(131072),
            }),
            interleaved: None,
        };
        let e = build_entry(&p, &remote, Some(&cm));
        assert_eq!(e.max_context_size, Some(204800));
        assert_eq!(e.max_output_size, Some(131072));
        assert!(e.capabilities.contains(&"image_in".to_string()));
        assert!(e.capabilities.contains(&"thinking".to_string()));
        assert_eq!(e.display_name.as_deref(), Some("GLM-5.3"));
        assert_eq!(e.support_efforts, vec!["low", "high", "max"]);
    }

    #[test]
    fn thinking_without_levels_gets_safe_defaults() {
        let p = provider();
        let remote = RemoteModel {
            id: "reasoner".into(),
            reasoning: Some(true),
            ..Default::default()
        };
        let e = build_entry(&p, &remote, None);
        assert!(e.capabilities.contains(&"thinking".to_string()));
        assert!(!e.support_efforts.is_empty(), "应给出默认档位");
        assert_eq!(e.default_effort.as_deref(), Some("high"));
    }
}
