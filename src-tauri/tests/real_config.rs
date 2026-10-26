//! 只读集成校验：直接读取本机真实的 Kimi Code 配置，确认解析器能正确处理现有数据。
//! 不做任何写操作；通过 `cargo test --test real_config -- --nocapture` 手动运行。

use std::path::Path;

use kc_switch_lib::core::{mcp, paths, plugins, providers, skills};

#[test]
fn reads_real_kimi_config_read_only() {
    let config = match paths::config_file() {
        Ok(p) => p,
        Err(_) => {
            println!("跳过：无法定位 Kimi Code 配置目录");
            return;
        }
    };
    if !config.exists() {
        println!("跳过：{} 不存在", config.display());
        return;
    }

    let snap = providers::load_snapshot(&config).expect("解析 config.toml 失败");
    println!("providers: {}", snap.providers.len());
    println!("default_model: {:?}", snap.default_model);
    for p in &snap.providers {
        println!(
            "  - {} (managed={}, models={}, key={:?})",
            p.name,
            p.managed,
            p.models.len(),
            p.api_key_masked.is_some()
        );
    }
    assert!(
        !snap.providers.is_empty(),
        "config.toml 中应至少解析出一个 provider"
    );

    // 密钥必须被打码
    for p in &snap.providers {
        if let Some(masked) = &p.api_key_masked {
            assert!(
                masked.contains("****") || masked.chars().all(|c| c == '*'),
                "api_key 未打码: {masked}"
            );
        }
    }

    if let Ok(mcp_path) = paths::mcp_file() {
        if mcp_path.exists() {
            let servers = mcp::list(&mcp_path).expect("解析 mcp.json 失败");
            println!("mcp servers: {}", servers.len());
            for s in &servers {
                println!("  - {} [{}] deferred={}", s.name, s.transport, s.deferred);
            }
        }
    }

    if let Ok(pf) = paths::plugins_file() {
        if pf.exists() {
            let list = plugins::list(&pf).expect("解析 installed.json 失败");
            println!("plugins: {}", list.len());
            for p in &list {
                println!("  - {} enabled={} ref={:?}", p.id, p.enabled, p.ref_label);
            }
        }
    }

    let all = skills::list(None).expect("扫描 skills 失败");
    let user = all.iter().filter(|s| s.source == "user").count();
    let disabled = all.iter().filter(|s| !s.enabled).count();
    let plugin = all.iter().filter(|s| s.source == "plugin").count();
    println!("skills: total={} user={} plugin={} disabled={}", all.len(), user, plugin, disabled);
    assert!(all.len() > 0, "应至少扫描到一个 skill");
    for s in all.iter().take(3) {
        println!("  - {} :: {}", s.name, truncate(&s.description, 60));
    }
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// 验证「备份 + 原子写 + 保留注释」在真实 config.toml 的副本上成立。
#[test]
fn roundtrip_on_copy_of_real_config() {
    let Ok(config) = paths::config_file() else {
        return;
    };
    if !config.exists() {
        println!("跳过：真实 config.toml 不存在");
        return;
    }
    let text = std::fs::read_to_string(&config).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let copy = tmp.path().join("config.toml");
    std::fs::write(&copy, &text).unwrap();

    let before = providers::load_snapshot(&copy).unwrap();
    // 选一个非托管 provider 下的模型做切换验证
    let target = before
        .providers
        .iter()
        .filter(|p| !p.managed)
        .flat_map(|p| p.models.iter())
        .map(|m| m.id.clone())
        .next();
    let Some(target) = target else {
        println!("跳过：没有可切换的非托管模型");
        return;
    };

    providers::set_default_model(&copy, &target).unwrap();
    let after = providers::load_snapshot(&copy).unwrap();
    assert_eq!(after.default_model.as_deref(), Some(target.as_str()));

    let new_text = std::fs::read_to_string(&copy).unwrap();
    let comment_lines_before = text.lines().filter(|l| l.trim_start().starts_with('#')).count();
    let comment_lines_after = new_text
        .lines()
        .filter(|l| l.trim_start().starts_with('#'))
        .count();
    println!(
        "注释行数 before={} after={}",
        comment_lines_before, comment_lines_after
    );
    assert_eq!(
        comment_lines_before, comment_lines_after,
        "写回后注释行数发生变化，toml_edit 未保住格式"
    );

    // 结构完整性：provider / model 数量不变
    assert_eq!(before.providers.len(), after.providers.len());
    let models_before: Vec<_> = before
        .providers
        .iter()
        .flat_map(|p| p.models.iter().map(|m| m.id.clone()))
        .collect();
    let models_after: Vec<_> = after
        .providers
        .iter()
        .flat_map(|p| p.models.iter().map(|m| m.id.clone()))
        .collect();
    assert_eq!(models_before, models_after);
}

#[test]
fn path_helpers_resolve() {
    let p = paths::describe().expect("解析路径失败");
    println!("{}", serde_json::to_string_pretty(&p).unwrap());
    for field in [
        &p.kimi_home,
        &p.config_file,
        &p.mcp_file,
        &p.plugins_file,
        &p.user_skills_dir,
    ] {
        assert!(!field.is_empty());
    }
    assert!(Path::new(&p.config_file).is_absolute());
}
