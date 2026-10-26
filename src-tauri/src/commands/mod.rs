use tauri::State;

use crate::core::{mcp, model_discovery, paths, plugins, providers, skills, Result};

#[tauri::command]
pub fn get_paths() -> Result<paths::AppPaths> {
    paths::describe()
}

#[tauri::command]
pub fn providers_list() -> Result<providers::ProvidersSnapshot> {
    providers::snapshot()
}

#[tauri::command]
pub fn providers_set_default_model(model: String) -> Result<()> {
    providers::set_default_model(&paths::config_file()?, &model)
}

#[tauri::command]
pub fn providers_upsert(input: providers::ProviderUpsert) -> Result<()> {
    providers::upsert_provider(&paths::config_file()?, &input)
}

#[tauri::command]
pub fn providers_remove(name: String, cascade_models: bool) -> Result<()> {
    providers::remove_provider(&paths::config_file()?, &name, cascade_models)
}

#[tauri::command]
pub fn providers_upsert_model(
    entry: providers::ModelEntry,
    original_id: Option<String>,
) -> Result<()> {
    providers::upsert_model(&paths::config_file()?, &entry, original_id.as_deref())
}

#[tauri::command]
pub fn providers_remove_model(id: String) -> Result<()> {
    providers::remove_model(&paths::config_file()?, &id)
}

#[tauri::command]
pub fn providers_diagnose(entry: providers::ModelEntry) -> Vec<providers::ModelIssue> {
    providers::diagnose_model(&entry)
}

#[tauri::command]
pub fn providers_meta() -> providers::MetaInfo {
    providers::meta()
}

/// 拉取某个 provider 的模型列表，并用 models.dev 目录补全参数。
///
/// async + spawn_blocking：网络请求可能要几十秒，若在命令线程里同步跑会让窗口「未响应」。
#[tauri::command]
pub async fn providers_discover_models(
    provider_name: String,
    refresh_catalog: bool,
) -> Result<model_discovery::DiscoverResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let snap = providers::snapshot()?;
        let provider = snap
            .providers
            .iter()
            .find(|p| p.name == provider_name)
            .ok_or_else(|| {
                crate::core::error::KcError::NotFound(format!("provider 不存在: {provider_name}"))
            })?;
        let existing: Vec<providers::ModelEntry> =
            snap.providers.iter().flat_map(|p| p.models.clone()).collect();
        model_discovery::discover(provider, &existing, refresh_catalog)
    })
    .await
    .map_err(|e| crate::core::error::KcError::Io(format!("任务失败: {e}")))?
}

/// 批量导入模型条目（已存在同名 ID 则覆盖）。
#[tauri::command]
pub fn providers_import_models(entries: Vec<providers::ModelEntry>) -> Result<usize> {
    let path = paths::config_file()?;
    let mut count = 0;
    for e in entries {
        providers::upsert_model(&path, &e, None)?;
        count += 1;
    }
    Ok(count)
}

#[tauri::command]
pub fn mcp_list() -> Result<Vec<mcp::McpServer>> {
    mcp::list(&paths::mcp_file()?)
}

#[tauri::command]
pub fn mcp_upsert(input: mcp::McpUpsert) -> Result<()> {
    mcp::upsert(&paths::mcp_file()?, &input)
}

#[tauri::command]
pub fn mcp_remove(name: String) -> Result<()> {
    mcp::remove(&paths::mcp_file()?, &name)
}

#[tauri::command]
pub fn mcp_set_deferred(name: String, deferred: bool) -> Result<()> {
    mcp::set_deferred(&paths::mcp_file()?, &name, deferred)
}

#[tauri::command]
pub fn skills_list(project_root: Option<String>) -> Result<Vec<skills::Skill>> {
    skills::list(project_root.as_deref())
}

#[tauri::command]
pub fn skills_toggle(path: String, enable: bool) -> Result<()> {
    skills::toggle(std::path::Path::new(&path), enable)
}

#[tauri::command]
pub fn skills_delete(path: String) -> Result<()> {
    skills::delete(std::path::Path::new(&path))
}

#[tauri::command]
pub fn plugins_list() -> Result<Vec<plugins::Plugin>> {
    plugins::list_default()
}

#[tauri::command]
pub fn plugins_toggle(id: String, enabled: bool) -> Result<()> {
    plugins::set_enabled(&paths::plugins_file()?, &id, enabled)
}

#[tauri::command]
pub fn plugins_remove(id: String, delete_files: bool) -> Result<()> {
    plugins::remove(&paths::plugins_file()?, &id, delete_files)
}

/// 切换窗口（含原生标题栏）的明暗主题。
#[tauri::command]
pub fn set_window_theme(window: tauri::Window, theme: String) -> Result<()> {
    use tauri::Theme;
    let t = if theme == "dark" { Theme::Dark } else { Theme::Light };
    window
        .set_theme(Some(t))
        .map_err(|e| crate::core::error::KcError::Io(format!("设置标题栏主题失败: {e}")))
}

#[allow(dead_code)]
pub struct AppState;

#[tauri::command]
pub fn app_version(_state: State<'_, AppState>) -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
