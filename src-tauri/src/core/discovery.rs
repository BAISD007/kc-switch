//! 模型发现：从供应商的 `/models` 接口拉取模型列表，
//! 并用 models.dev 公共目录补齐接口没给的元数据（上下文长度、模态、思考档位等）。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 供应商 `/models` 返回的原始模型条目（各家字段差异极大，全部按可选处理）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RemoteModel {
    pub id: String,
    pub display_name: Option<String>,
    /// 上下文窗口（token）
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    /// 输入模态，如 ["text", "image"]
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    /// 思考档位
    pub reasoning_levels: Vec<String>,
    pub default_reasoning_level: Option<String>,
    pub reasoning: Option<bool>,
    /// 元数据来源：api / catalog / none
    pub source: String,
}

fn str_field(v: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_str()).map(str::to_string))
}

fn int_field(v: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_i64()))
}

fn str_array(v: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_array()).map(|a| {
            a.iter()
                .filter_map(|i| i.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        }))
        .unwrap_or_default()
}

/// 解析供应商返回的 models 列表。兼容三种常见结构：
/// - OpenAI: `{"data":[{id, object, ...}]}`
/// - 自定义: `{"models":[...]}`
/// - 裸数组: `[...]`
pub fn parse_models_response(body: &str) -> Result<Vec<RemoteModel>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("响应不是有效 JSON: {e}"))?;

    let items: Vec<&Value> = if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        arr.iter().collect()
    } else if let Some(arr) = v.get("models").and_then(|d| d.as_array()) {
        arr.iter().collect()
    } else if let Some(arr) = v.as_array() {
        arr.iter().collect()
    } else if let Some(obj) = v.as_object() {
        // 少数实现直接返回 {modelId: {...}}
        if obj.values().all(|x| x.is_object()) && obj.keys().any(|k| obj[k].is_object()) {
            return Ok(obj
                .iter()
                .map(|(k, val)| build_remote(k.clone(), val))
                .collect());
        }
        return Err("无法识别的响应结构".into());
    } else {
        return Err("无法识别的响应结构".into());
    };

    Ok(items
        .into_iter()
        .filter_map(|it| {
            let id = str_field(it, &["id", "name", "model"])?;
            Some(build_remote(id, it))
        })
        .collect())
}

fn build_remote(id: String, v: &Value) -> RemoteModel {
    // 思考档位有两种常见写法：effort.supported_levels 或 supported_reasoning_levels
    let (levels, default_level) = {
        let mut levels = str_array(v, &["supported_reasoning_levels"]);
        let mut default = None;
        if let Some(eff) = v.get("effort") {
            if levels.is_empty() {
                levels = str_array(eff, &["supported_levels"]);
            }
            default = str_field(eff, &["default_level"]);
        }
        if levels.is_empty() {
            if let Some(opts) = v.get("reasoning_options").and_then(|o| o.as_array()) {
                for opt in opts {
                    if opt.get("type").and_then(|t| t.as_str()) == Some("effort") {
                        levels = str_array(opt, &["values"]);
                        default = str_field(opt, &["default"]);
                    }
                }
            }
        }
        (levels, default)
    };

    let mut input_modalities = str_array(v, &["input_modalities"]);
    if input_modalities.is_empty() {
        if let Some(m) = v.get("modalities") {
            input_modalities = str_array(m, &["input"]);
        }
    }
    let mut output_modalities = str_array(v, &["output_modalities"]);
    if output_modalities.is_empty() {
        if let Some(m) = v.get("modalities") {
            output_modalities = str_array(m, &["output"]);
        }
    }

    RemoteModel {
        display_name: str_field(v, &["display_name", "name"]),
        context_window: int_field(v, &["context_window", "context_length"])
            .or_else(|| v.get("limit").and_then(|l| l.get("context")).and_then(|x| x.as_i64())),
        max_output_tokens: int_field(v, &["max_output_tokens"])
            .or_else(|| v.get("limit").and_then(|l| l.get("output")).and_then(|x| x.as_i64())),
        input_modalities,
        output_modalities,
        reasoning_levels: levels,
        default_reasoning_level: default_level,
        reasoning: v.get("reasoning").and_then(|r| r.as_bool()),
        source: "api".into(),
        id,
    }
}

/// models.dev 目录中的一个模型条目（只取需要的字段）
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogModel {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub tool_call: bool,
    #[serde(default)]
    pub attachment: bool,
    #[serde(default)]
    pub reasoning_options: Vec<Value>,
    #[serde(default)]
    pub modalities: Option<Modalities>,
    #[serde(default)]
    pub limit: Option<Limit>,
    /// 可能是对象 `{field: "..."}`，也可能是布尔（各家写法不一）
    #[serde(default)]
    pub interleaved: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Modalities {
    #[serde(default)]
    pub input: Vec<String>,
    #[serde(default)]
    pub output: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Limit {
    #[serde(default)]
    pub context: Option<i64>,
    #[serde(default)]
    pub output: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CatalogProvider {
    #[serde(default)]
    pub api: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub models: std::collections::HashMap<String, CatalogModel>,
}

/// 按 base_url 在该目录里找最匹配的 provider。
/// 精确匹配优先，其次互相包含；都不命中返回 None。
pub fn match_catalog_provider<'a>(
    catalog: &'a std::collections::HashMap<String, CatalogProvider>,
    base_url: &str,
) -> Option<(&'a str, &'a CatalogProvider)> {
    let norm = |u: &str| {
        u.trim()
            .trim_end_matches('/')
            .to_lowercase()
            .replace("https://", "")
            .replace("http://", "")
            .replace("/api/paas/v4", "")
            .replace("/openai/v1", "")
    };
    let target = norm(base_url);
    if target.is_empty() {
        return None;
    }

    let mut best: Option<(usize, &str, &CatalogProvider)> = None;
    for (id, p) in catalog {
        let Some(api) = p.api.as_deref() else { continue };
        let cand = norm(api);
        if cand.is_empty() {
            continue;
        }
        let score = if cand == target {
            100
        } else if cand.contains(&target) || target.contains(&cand) {
            50
        } else {
            continue;
        };
        if best.map(|(s, _, _)| score > s).unwrap_or(true) {
            best = Some((score, id, p));
        }
    }
    best.map(|(_, id, p)| (id, p))
}

/// 把目录里某个模型的能力换算成 Kimi Code 的 capabilities 列表
pub fn catalog_capabilities(m: &CatalogModel) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let has_image = m
        .modalities
        .as_ref()
        .map(|x| x.input.iter().any(|i| i == "image"))
        .unwrap_or(false)
        || m.attachment;
    let has_video = m
        .modalities
        .as_ref()
        .map(|x| x.input.iter().any(|i| i == "video"))
        .unwrap_or(false);

    if m.reasoning {
        out.push("thinking".into());
    }
    if has_image {
        out.push("image_in".into());
    }
    if has_video {
        out.push("video_in".into());
    }
    if m.tool_call {
        out.push("tool_use".into());
    }
    // 目录里没有 always_thinking 概念，不做臆测
    out
}

pub fn catalog_efforts(m: &CatalogModel) -> (Vec<String>, Option<String>) {
    for opt in &m.reasoning_options {
        if opt.get("type").and_then(|t| t.as_str()) == Some("effort") {
            let values = opt
                .get("values")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|i| i.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let default = opt.get("default").and_then(|d| d.as_str()).map(str::to_string);
            return (values, default);
        }
    }
    (Vec::new(), None)
}

pub fn catalog_reasoning_key(m: &CatalogModel) -> Option<String> {
    m.interleaved
        .as_ref()
        .and_then(|i| i.get("field"))
        .and_then(|f| f.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_style() {
        let body = r#"{"object":"list","data":[
            {"id":"deepseek-flash","object":"model","context_window":1048576,
             "max_output_tokens":393216,"input_modalities":["text","image"],
             "effort":{"supported_levels":["low","high","max"],"default_level":"high"}}
        ]}"#;
        let models = parse_models_response(body).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "deepseek-flash");
        assert_eq!(models[0].context_window, Some(1048576));
        assert_eq!(models[0].max_output_tokens, Some(393216));
        assert_eq!(models[0].input_modalities, vec!["text", "image"]);
        assert_eq!(models[0].reasoning_levels, vec!["low", "high", "max"]);
        assert_eq!(models[0].default_reasoning_level.as_deref(), Some("high"));
    }

    #[test]
    fn parses_minimal_openai_style() {
        let body = r#"{"data":[{"id":"glm-4.5","object":"model","owned_by":"z-ai"}]}"#;
        let models = parse_models_response(body).unwrap();
        assert_eq!(models[0].id, "glm-4.5");
        assert_eq!(models[0].context_window, None);
        assert!(models[0].input_modalities.is_empty());
    }

    #[test]
    fn parses_bare_array_and_models_key() {
        let a = parse_models_response(r#"[{"id":"x"}]"#).unwrap();
        assert_eq!(a[0].id, "x");
        let b = parse_models_response(r#"{"models":[{"id":"y"}]}"#).unwrap();
        assert_eq!(b[0].id, "y");
    }

    #[test]
    fn parses_supported_reasoning_levels_variant() {
        let body = r#"{"data":[{"id":"qwen","supported_reasoning_levels":["low","medium","xhigh"],"reasoning":true}]}"#;
        let m = &parse_models_response(body).unwrap()[0];
        assert_eq!(m.reasoning_levels, vec!["low", "medium", "xhigh"]);
        assert_eq!(m.reasoning, Some(true));
    }

    #[test]
    fn rejects_non_json() {
        assert!(parse_models_response("<html>500</html>").is_err());
    }

    #[test]
    fn matches_catalog_by_base_url() {
        let mut catalog = std::collections::HashMap::new();
        catalog.insert(
            "deepseek".to_string(),
            CatalogProvider {
                api: Some("https://api.deepseek.com".into()),
                name: Some("DeepSeek".into()),
                models: Default::default(),
            },
        );
        catalog.insert(
            "zhipuai".to_string(),
            CatalogProvider {
                api: Some("https://open.bigmodel.cn/api/paas/v4".into()),
                name: Some("Zhipu AI".into()),
                models: Default::default(),
            },
        );
        let (id, _) = match_catalog_provider(&catalog, "https://api.deepseek.com/").unwrap();
        assert_eq!(id, "deepseek");
        let (id, _) =
            match_catalog_provider(&catalog, "https://open.bigmodel.cn/api/paas/v4").unwrap();
        assert_eq!(id, "zhipuai");
        assert!(match_catalog_provider(&catalog, "https://unknown.example").is_none());
    }

    #[test]
    fn catalog_maps_capabilities() {
        let m = CatalogModel {
            name: Some("Test".into()),
            reasoning: true,
            tool_call: true,
            attachment: false,
            reasoning_options: vec![],
            modalities: Some(Modalities {
                input: vec!["text".into(), "image".into()],
                output: vec!["text".into()],
            }),
            limit: Some(Limit {
                context: Some(1000000),
                output: Some(131072),
            }),
            interleaved: None,
        };
        let caps = catalog_capabilities(&m);
        assert!(caps.contains(&"thinking".to_string()));
        assert!(caps.contains(&"image_in".to_string()));
        assert!(caps.contains(&"tool_use".to_string()));
        assert!(!caps.contains(&"video_in".to_string()));
    }

    #[test]
    fn catalog_extracts_efforts() {
        let m = CatalogModel {
            name: None,
            reasoning: true,
            tool_call: false,
            attachment: false,
            reasoning_options: vec![serde_json::json!({
                "type": "effort",
                "values": ["low", "high", "max"]
            })],
            modalities: None,
            limit: None,
            interleaved: Some(serde_json::json!({"field": "reasoning_content"})),
        };
        let (levels, _) = catalog_efforts(&m);
        assert_eq!(levels, vec!["low", "high", "max"]);
        assert_eq!(
            catalog_reasoning_key(&m).as_deref(),
            Some("reasoning_content")
        );
    }
}
