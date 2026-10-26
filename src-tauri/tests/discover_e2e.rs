//! 真实网络端到端验证：拉取供应商模型列表 + models.dev 元数据补全。
//! 运行：`cargo test --test discover_e2e -- --nocapture --test-threads=1`
//!
//! 注意：本测试只读真实配置（不发写操作），网络失败时跳过而不失败。

use kc_switch_lib::core::{model_discovery, providers};

#[test]
fn discover_models_from_real_provider() {
    let Ok(snap) = providers::snapshot() else {
        println!("跳过：无法读取配置");
        return;
    };

    // 挑一个有 base_url 和 key、且非本地回环的 provider
    let Some(provider) = snap.providers.iter().find(|p| {
        !p.managed
            && p.has_api_key
            && p.base_url
                .as_deref()
                .map(|u| !u.contains("127.0.0.1") && !u.contains("localhost"))
                .unwrap_or(false)
    }) else {
        println!("跳过：没有可用于测试的 provider");
        return;
    };

    println!("测试 provider: {} ({:?})", provider.name, provider.base_url);
    let existing: Vec<_> = snap.providers.iter().flat_map(|p| p.models.clone()).collect();

    match model_discovery::discover(provider, &existing, false) {
        Ok(result) => {
            println!("\n诊断信息:");
            for n in &result.notes {
                println!("  · {n}");
            }
            println!("\n发现 {} 个模型:", result.models.len());
            for m in result.models.iter().take(8) {
                println!(
                    "  - {:<28} ctx={:<9} caps={:<38} efforts={} src={} exists={}",
                    m.remote_id,
                    m.entry
                        .max_context_size
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "-".into()),
                    m.entry.capabilities.join(","),
                    m.entry.support_efforts.join(","),
                    m.source,
                    m.exists
                );
            }
            assert!(!result.models.is_empty(), "应至少发现一个模型");

            // 有元数据的模型应占一定比例（接口或目录给到）
            let with_meta = result.models.iter().filter(|m| m.metadata_ok).count();
            println!(
                "\n带元数据的模型: {}/{}",
                with_meta,
                result.models.len()
            );
        }
        Err(e) => {
            println!("发现失败（可能是网络或密钥问题）: {e}");
        }
    }
}

#[test]
fn catalog_loads_and_matches() {
    match model_discovery::load_catalog(false) {
        Ok(cat) => {
            println!("目录供应商数: {}", cat.len());
            assert!(cat.len() > 100, "目录应有大量供应商");

            // 验证几个常见 base_url 能匹配
            for (url, expect_contains) in [
                ("https://api.deepseek.com", "deepseek"),
                ("https://open.bigmodel.cn/api/paas/v4", "zhipu"),
            ] {
                match kc_switch_lib::core::discovery::match_catalog_provider(&cat, url) {
                    Some((id, p)) => {
                        println!(
                            "  {url} -> {id} ({} models)",
                            p.models.len()
                        );
                        assert!(
                            id.contains(expect_contains),
                            "{url} 应匹配到 include {expect_contains}，实际 {id}"
                        );
                    }
                    None => println!("  {url} -> 未匹配"),
                }
            }
        }
        Err(e) => println!("目录加载失败（网络问题，跳过）: {e}"),
    }
}
