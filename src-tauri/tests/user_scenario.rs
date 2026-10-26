//! 用户核心场景验证：给官方未收录的模型补齐「图片识别 + 思考强度」字段。
//!
//! 场景：用户在 GUI 里新增一个 GLM 视觉模型，勾选 image_in、设置思考档位，
//! 保存后 config.toml 必须出现正确字段，Kimi Code 才能识图/切档位。

use std::path::PathBuf;

use kc_switch_lib::core::{providers, providers::ModelEntry};

fn work_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    let p = p.join(".verify");
    std::fs::create_dir_all(&p).unwrap();
    p
}

const BASE: &str = r#"# 我的配置
default_model = "zhipu/glm-4.6"

[providers."zhipu"]
base_url = "https://open.bigmodel.cn/api/paas/v4"
type = "openai"
api_key = "sk-test-1234567890"

[models."zhipu/glm-4.6"]
provider = "zhipu"
model = "glm-4.6"
display_name = "GLM-4.6"
max_context_size = 204800
capabilities = [ "thinking", "tool_use" ]
support_efforts = [ "low", "high" ]
default_effort = "high"
"#;

#[test]
fn add_vision_model_with_thinking() {
    let f = work_dir().join("add-model.toml");
    std::fs::write(&f, BASE).unwrap();

    // 用户想加的新模型：GLM-4.6V（视觉版），带思考档位
    let entry = ModelEntry {
        id: "zhipu/glm-4.6v".into(),
        provider: "zhipu".into(),
        model: "glm-4.6v".into(),
        display_name: Some("GLM-4.6V".into()),
        max_context_size: Some(131072),
        max_output_size: None,
        capabilities: vec![
            "image_in".into(),
            "thinking".into(),
            "tool_use".into(),
        ],
        support_efforts: vec!["low".into(), "high".into(), "max".into()],
        default_effort: Some("high".into()),
        adaptive_thinking: None,
        reasoning_key: None,
        unknown: vec![],
        _used_catalog: false,
    };

    // 保存前先诊断：不应有 error
    let issues = providers::diagnose_model(&entry);
    let errors: Vec<_> = issues.iter().filter(|i| i.level == "error").collect();
    assert!(errors.is_empty(), "保存前不应有 error 级问题: {errors:?}");

    providers::upsert_model(&f, &entry, None).expect("保存失败");

    // 读回验证：Kimi Code 真正关心的字段都在
    let snap = providers::load_snapshot(&f).unwrap();
    let m = snap
        .providers
        .iter()
        .find(|p| p.name == "zhipu")
        .unwrap()
        .models
        .iter()
        .find(|m| m.id == "zhipu/glm-4.6v")
        .expect("模型未写入");

    assert!(m.capabilities.contains(&"image_in".to_string()), "缺少图片识别");
    assert!(m.capabilities.contains(&"thinking".to_string()), "缺少思考");
    assert_eq!(m.support_efforts, vec!["low", "high", "max"]);
    assert_eq!(m.default_effort.as_deref(), Some("high"));
    assert_eq!(m.max_context_size, Some(131072));

    // 原始注释与其他模型不得受影响
    let text = std::fs::read_to_string(&f).unwrap();
    assert!(text.contains("# 我的配置"), "注释丢失");
    assert!(text.contains(r#"[models."zhipu/glm-4.6"]"#), "原有模型丢失");
    assert!(text.contains("sk-test-1234567890"), "api_key 被改动");
    println!("新增视觉模型验证通过");
}

#[test]
fn fix_existing_model_missing_image_in() {
    let f = work_dir().join("fix-model.toml");
    // 用户踩坑现场：模型能识图，但配置漏了 image_in，也没设思考档位
    let broken = r#"default_model = "zhipu/glm-4.5v"

[providers."zhipu"]
base_url = "https://open.bigmodel.cn/api/paas/v4"
type = "openai"
api_key = "sk-test"

[models."zhipu/glm-4.5v"]
provider = "zhipu"
model = "glm-4.5v"
"#;
    std::fs::write(&f, broken).unwrap();

    let before = providers::load_snapshot(&f).unwrap();
    let mut m = before.providers[0].models[0].clone();
    assert!(m.capabilities.is_empty());

    // 诊断应该提示缺 image_in 和 max_context_size
    let issues = providers::diagnose_model(&m);
    assert!(
        issues.iter().any(|i| i.message.contains("image_in")),
        "应提示缺图片识别"
    );
    assert!(
        issues.iter().any(|i| i.message.contains("max_context_size")),
        "应提示缺上下文长度"
    );

    // 用户在 GUI 里补齐
    m.capabilities = vec!["image_in".into(), "thinking".into(), "tool_use".into()];
    m.support_efforts = vec!["low".into(), "high".into(), "max".into()];
    m.default_effort = Some("high".into());
    m.max_context_size = Some(131072);
    providers::upsert_model(&f, &m, Some("zhipu/glm-4.5v")).unwrap();

    let after = providers::load_snapshot(&f).unwrap();
    let fixed = &after.providers[0].models[0];
    assert!(fixed.capabilities.contains(&"image_in".to_string()));
    assert_eq!(fixed.support_efforts.len(), 3);
    // 修好后诊断不应再有 warn/error
    let remaining: Vec<_> = providers::diagnose_model(fixed)
        .into_iter()
        .filter(|i| i.level != "info")
        .collect();
    assert!(remaining.is_empty(), "修好后仍有问题: {remaining:?}");
    println!("修复已有模型验证通过");
}

#[test]
fn provider_with_new_model_end_to_end() {
    let f = work_dir().join("new-provider.toml");
    std::fs::write(
        f.as_path(),
        "default_model = \"\"\n\n[providers.\"existing\"]\nbase_url = \"https://a\"\ntype = \"openai\"\n",
    )
    .unwrap();

    // 1. 先加一个全新 provider（用户自建中转站）
    providers::upsert_provider(
        &f,
        &providers::ProviderUpsert {
            name: "my-relay".into(),
            original_name: None,
            base_url: "https://relay.example/v1".into(),
            api_key: Some("sk-relay-abcdefgh".into()),
            provider_type: Some("openai".into()),
        },
    )
    .unwrap();

    // 2. 在新 provider 下加模型
    let entry = ModelEntry {
        id: "my-relay/some-vision-model".into(),
        provider: "my-relay".into(),
        model: "some-vision-model".into(),
        display_name: Some("My Vision".into()),
        max_context_size: Some(262144),
        max_output_size: None,
        capabilities: vec!["image_in".into(), "tool_use".into(), "thinking".into()],
        support_efforts: vec!["low".into(), "high".into()],
        default_effort: Some("high".into()),
        adaptive_thinking: None,
        reasoning_key: None,
        unknown: vec![],
        _used_catalog: false,
    };
    providers::upsert_model(&f, &entry, None).unwrap();

    let snap = providers::load_snapshot(&f).unwrap();
    let relay = snap.providers.iter().find(|p| p.name == "my-relay").unwrap();
    assert_eq!(relay.base_url.as_deref(), Some("https://relay.example/v1"));
    assert!(relay.api_key_masked.as_ref().unwrap().contains("****"));
    assert_eq!(relay.models.len(), 1);
    assert!(relay.models[0].capabilities.contains(&"image_in".to_string()));
    println!("新建 provider + 模型 全流程通过");
}
