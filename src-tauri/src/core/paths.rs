use std::path::PathBuf;

use serde::Serialize;

use super::error::{KcError, Result};

pub fn home_dir() -> Result<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| KcError::NotFound("无法定位用户主目录".into()))
}

pub fn kimi_home() -> Result<PathBuf> {
    if let Some(v) = std::env::var_os("KIMI_CODE_HOME") {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    Ok(home_dir()?.join(".kimi-code"))
}

pub fn config_file() -> Result<PathBuf> {
    Ok(kimi_home()?.join("config.toml"))
}

pub fn mcp_file() -> Result<PathBuf> {
    Ok(kimi_home()?.join("mcp.json"))
}

pub fn plugins_file() -> Result<PathBuf> {
    Ok(kimi_home()?.join("plugins").join("installed.json"))
}

pub fn managed_plugins_dir() -> Result<PathBuf> {
    Ok(kimi_home()?.join("plugins").join("managed"))
}

pub fn agents_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".agents"))
}

pub fn user_skills_dir() -> Result<PathBuf> {
    Ok(agents_dir()?.join("skills"))
}

pub fn disabled_skills_dir() -> Result<PathBuf> {
    Ok(agents_dir()?.join("skills-disabled"))
}

pub fn project_skills_dir(project_root: &str) -> PathBuf {
    PathBuf::from(project_root).join(".agents").join("skills")
}

pub fn project_disabled_skills_dir(project_root: &str) -> PathBuf {
    PathBuf::from(project_root)
        .join(".agents")
        .join("skills-disabled")
}

/// KC-Switch 自己的数据目录（备份、回收站、UI 状态）。
/// 可用 KC_SWITCH_DATA_DIR 覆盖；开发/合规场景请指向非 C 盘。
pub fn data_dir() -> Result<PathBuf> {
    if let Some(v) = std::env::var_os("KC_SWITCH_DATA_DIR") {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    let base = if let Some(v) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(v)
    } else if let Some(v) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(v)
    } else {
        home_dir()?.join(".local").join("share")
    };
    Ok(base.join("KC-Switch"))
}

pub fn backups_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("backups"))
}

pub fn trash_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("trash"))
}

pub fn state_file() -> Result<PathBuf> {
    Ok(data_dir()?.join("state.json"))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub kimi_home: String,
    pub config_file: String,
    pub mcp_file: String,
    pub plugins_file: String,
    pub user_skills_dir: String,
    pub disabled_skills_dir: String,
    pub data_dir: String,
}

pub fn describe() -> Result<AppPaths> {
    Ok(AppPaths {
        kimi_home: kimi_home()?.display().to_string(),
        config_file: config_file()?.display().to_string(),
        mcp_file: mcp_file()?.display().to_string(),
        plugins_file: plugins_file()?.display().to_string(),
        user_skills_dir: user_skills_dir()?.display().to_string(),
        disabled_skills_dir: disabled_skills_dir()?.display().to_string(),
        data_dir: data_dir()?.display().to_string(),
    })
}
