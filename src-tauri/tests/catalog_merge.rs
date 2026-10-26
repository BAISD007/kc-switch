//! 验证：接口只给 id、但 models.dev 有元数据时，能补全上下文/能力/档位。
//! 用 DeepSeek 与智谱（目录里都有）实测。

use kc_switch_lib::core::{discovery, model_discovery, providers};

#[test]
fn deepseek_gets_metadata_from_catalog() {
    let Ok(snap) = providers::snapshot() else {
        println!("跳过");
        return;
    };
    let Some(p) = snap
        .providers
        .iter()
        .find(|p| p.name.to_lowercase().contains("deepseek") && p.has_api_key)
    else {
        println!("跳过：没有 DeepSeek provider");
        return;
    };

    let existing: Vec<_> = snap.providers.iter().flat_map(|p| p.models.clone()).collect();
    let r = match model_discovery::discover(p, &existing, false) {
        Ok(r) => r,
        Err(e) => {
            println!("跳过（网络/密钥）：{e}");
            return;
        }
    };

    println!("provider: {}", p.name);
    for note in &r.notes {
        println!("  · {note}");
    }
    for m in &r.models {
        println!(
            "  - {:<24} ctx={:<9} out={:<8} caps=[{}] efforts=[{}] default={:?} key={:?} src={}",
            m.remote_id,
            m.entry.max_context_size.map(|c| c.to_string()).unwrap_or("-".into()),
            m.entry.max_output_size.map(|c| c.to_string()).unwrap_or("-".into()),
            m.entry.capabilities.join(","),
            m.entry.support_efforts.join(","),
            m.entry.default_effort,
            m.entry.reasoning_key,
            m.source
        );
    }

    let rich = r.models.iter().filter(|m| m.entry.max_context_size.is_some()).count();
    println!("\n拿到上下文长度的模型: {rich}/{}", r.models.len());
    assert!(rich > 0, "DeepSeek 应能从接口或目录拿到上下文长度");
}

#[test]
fn zhipu_models_merge_catalog_capabilities() {
    let Ok(cat) = model_discovery::load_catalog(false) else {
        println!("跳过：目录不可用");
        return;
    };
    let Some((id, cp)) = discovery::match_catalog_provider(&cat, "https://open.bigmodel.cn/api/paas/v4")
    else {
        println!("跳过：未匹配智谱");
        return;
    };
    println!("匹配到目录: {id}，共 {} 个模型", cp.models.len());

    // 检查几个视觉模型能识别出 image_in
    let mut vision = Vec::new();
    let mut with_ctx = 0;
    for (mid, m) in &cp.models {
        let caps = discovery::catalog_capabilities(m);
        if caps.iter().any(|c| c == "image_in") {
            vision.push(mid.clone());
        }
        if m.limit.as_ref().and_then(|l| l.context).is_some() {
            with_ctx += 1;
        }
    }
    println!("有上下文长度: {with_ctx}/{}", cp.models.len());
    println!("支持图片输入的模型: {:?}", vision);

    assert!(with_ctx > 0, "目录应能提供上下文长度");
}
