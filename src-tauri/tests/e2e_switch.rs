//! 端到端验证：在真实 config.toml 的**副本**上验证切换与写保护。
//!
//! 为何用副本：真实 config.toml 是用户当前正在使用的配置（含 API 密钥），
//! 写入它会改变用户正在用的模型，并可能泄露密钥到备份文件。
//! 副本与真实文件内容完全一致，足以验证解析、切换、格式保留的完整链路。
//!
//! 运行：`cargo test --test e2e_switch -- --nocapture --test-threads=1`

use std::path::PathBuf;

use kc_switch_lib::core::{paths, providers, write_guard};

fn project_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

#[test]
fn switch_default_model_on_copy_of_real_config() {
    let Ok(config) = paths::config_file() else {
        println!("跳过：无法定位 config.toml");
        return;
    };
    if !config.exists() {
        println!("跳过：config.toml 不存在");
        return;
    }

    // 把真实配置复制到项目内临时目录（读 C 盘，写 F 盘）
    let work = project_dir().join(".verify");
    std::fs::create_dir_all(&work).unwrap();
    let copy = work.join("config.toml");
    let original_text = std::fs::read_to_string(&config).unwrap();
    std::fs::write(&copy, &original_text).unwrap();
    println!("已复制真实配置 -> {}", copy.display());

    let before = providers::load_snapshot(&copy).expect("解析失败");
    let original_default = before.default_model.clone();
    println!("当前 default_model = {:?}", original_default);

    let candidate = before
        .providers
        .iter()
        .filter(|p| !p.managed)
        .flat_map(|p| p.models.iter())
        .map(|m| m.id.clone())
        .find(|id| Some(id) != original_default.as_ref());

    let Some(candidate) = candidate else {
        println!("跳过：没有可切换的非托管模型");
        return;
    };
    println!("切换到 -> {}", candidate);

    providers::set_default_model(&copy, &candidate).expect("切换失败");

    let after = providers::load_snapshot(&copy).expect("切换后解析失败");
    assert_eq!(after.default_model.as_deref(), Some(candidate.as_str()));

    // 磁盘内容确实变了
    let switched_text = std::fs::read_to_string(&copy).unwrap();
    assert_ne!(original_text, switched_text, "文件内容没有变化");
    assert!(switched_text.contains(&format!("default_model = \"{candidate}\"")));

    // 结构完整性
    assert_eq!(before.providers.len(), after.providers.len());
    let count_models = |s: &providers::ProvidersSnapshot| -> usize {
        s.providers.iter().map(|p| p.models.len()).sum()
    };
    assert_eq!(count_models(&before), count_models(&after));

    // 切回原值并校验：除 default_model 行外逐行一致（等价于格式/注释零损伤）
    if let Some(orig) = original_default.clone() {
        providers::set_default_model(&copy, &orig).expect("切回原值失败");
        let restored = providers::load_snapshot(&copy).unwrap();
        assert_eq!(restored.default_model, original_default);
    }
    let final_text = std::fs::read_to_string(&copy).unwrap();
    let norm = |t: &str| -> Vec<String> {
        t.lines()
            .filter(|l| !l.trim_start().starts_with("default_model"))
            .map(|l| l.trim_end().to_string())
            .collect()
    };
    assert_eq!(
        norm(&original_text),
        norm(&final_text),
        "除 default_model 外，文件其余内容发生了变化（格式/注释受损）"
    );
    println!("格式与结构一致性校验通过");
}

#[test]
fn format_preservation_with_comments() {
    let work = project_dir().join(".verify");
    std::fs::create_dir_all(&work).unwrap();
    let f = work.join("commented.toml");
    let src = "# 顶部注释：不应丢失\ndefault_model = \"p/m1\"\n\n[providers.p]\n# provider 注释\nbase_url = \"https://x\"\napi_key = \"sk-abcdefghijkl\"\n\n[models.\"p/m1\"]\nprovider = \"p\"\nmodel = \"m1\"\n\n[models.\"p/m2\"]\nprovider = \"p\"\nmodel = \"m2\"\n";
    std::fs::write(&f, src).unwrap();

    providers::set_default_model(&f, "p/m2").unwrap();
    let out = std::fs::read_to_string(&f).unwrap();
    assert!(out.contains("# 顶部注释：不应丢失"), "顶部注释丢失");
    assert!(out.contains("# provider 注释"), "provider 注释丢失");
    assert!(out.contains("default_model = \"p/m2\""));

    let before: Vec<&str> = src.lines().filter(|l| l.contains('#')).collect();
    let after: Vec<&str> = out.lines().filter(|l| l.contains('#')).collect();
    assert_eq!(before, after, "注释内容发生变化");
    println!("注释保留校验通过（{} 行注释）", after.len());
}

#[test]
fn concurrent_write_is_rejected() {
    let work = project_dir().join(".verify");
    std::fs::create_dir_all(&work).unwrap();
    let target = work.join("concurrent.toml");
    std::fs::write(&target, "default_model = \"a\"\n").unwrap();

    let (_, snapshot) = write_guard::read_tracked(&target).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));
    std::fs::write(&target, "default_model = \"b\"\n").unwrap();

    let res = write_guard::write_if_unchanged(&target, snapshot, b"default_model = \"c\"\n");
    assert!(res.is_err(), "并发修改应被拒绝");
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "default_model = \"b\"\n",
        "被拒绝时不得改动原文件"
    );
    println!("并发写保护生效：{}", res.unwrap_err());
}

#[test]
fn no_files_written_outside_project() {
    // 这条断言防止未来有人把测试改成写真实配置
    let cfg = paths::config_file().unwrap();
    let meta = std::fs::metadata(&cfg).unwrap();
    let modified = meta.modified().unwrap();
    let elapsed = std::time::SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    assert!(
        elapsed.as_secs() > 5,
        "真实 config.toml 刚刚被修改过——测试不应写入真实配置"
    );
}
