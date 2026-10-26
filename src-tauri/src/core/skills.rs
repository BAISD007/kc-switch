use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::error::{KcError, Result};
use super::fs_ops;
use super::paths;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub source: String,
    pub enabled: bool,
    pub path: String,
    pub plugin: Option<String>,
    pub has_skill_md: bool,
}

/// 极简 frontmatter 解析：只取 name / description，支持跨行的单/双引号字符串。
fn parse_frontmatter(text: &str) -> (Option<String>, Option<String>) {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default().trim();
    if first != "---" {
        return (None, None);
    }
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut current: Option<&str> = None;

    for raw in lines {
        let line = raw.trim_end();
        if line.trim() == "---" {
            break;
        }
        if let Some(key) = current {
            let value = line.trim();
            let target = if key == "name" { &mut name } else { &mut description };
            let joined = format!("{} {}", target.clone().unwrap_or_default(), value);
            *target = Some(joined.trim().to_string());
            if !value.ends_with('\'') && !value.ends_with('"') {
                continue;
            }
            current = None;
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            if key != "name" && key != "description" {
                continue;
            }
            let value = value.trim();
            let (quote_open, inner) = if let Some(rest) = value.strip_prefix('\'') {
                (!rest.ends_with('\'') || rest.len() == 1, rest)
            } else if let Some(rest) = value.strip_prefix('"') {
                (!rest.ends_with('"') || rest.len() == 1, rest)
            } else {
                (false, value)
            };
            let cleaned = inner
                .trim_end_matches('\'')
                .trim_end_matches('"')
                .trim()
                .to_string();
            let target = if key == "name" { &mut name } else { &mut description };
            *target = Some(cleaned);
            if quote_open {
                current = Some(if key == "name" { "name" } else { "description" });
            }
        }
    }
    (name, description)
}

fn read_skill_dir(dir: &Path) -> Option<(Option<String>, Option<String>)> {
    let md = dir.join("SKILL.md");
    if !md.is_file() {
        return None;
    }
    let text = fs::read_to_string(&md).ok()?;
    Some(parse_frontmatter(&text))
}

fn scan_dir(
    root: &Path,
    enabled: bool,
    source: &str,
    plugin: Option<&str>,
    out: &mut Vec<Skill>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if dir_name.starts_with('.') {
            continue;
        }
        let Some((name, description)) = read_skill_dir(&path) else {
            continue;
        };
        out.push(Skill {
            name: name.unwrap_or_else(|| dir_name.clone()),
            description: description.unwrap_or_default(),
            source: source.to_string(),
            enabled,
            path: path.display().to_string(),
            plugin: plugin.map(str::to_string),
            has_skill_md: true,
        });
    }
}

pub fn list(project_root: Option<&str>) -> Result<Vec<Skill>> {
    let mut out: Vec<Skill> = Vec::new();

    if let Ok(dir) = paths::user_skills_dir() {
        scan_dir(&dir, true, "user", None, &mut out);
    }
    if let Ok(dir) = paths::disabled_skills_dir() {
        scan_dir(&dir, false, "user", None, &mut out);
    }
    if let Some(root) = project_root.filter(|r| !r.trim().is_empty()) {
        scan_dir(
            &paths::project_skills_dir(root),
            true,
            "project",
            None,
            &mut out,
        );
        scan_dir(
            &paths::project_disabled_skills_dir(root),
            false,
            "project",
            None,
            &mut out,
        );
    }

    for plugin in super::plugins::list_default()? {
        let skills_dir = PathBuf::from(&plugin.root).join("skills");
        scan_dir(
            &skills_dir,
            plugin.enabled,
            "plugin",
            Some(&plugin.id),
            &mut out,
        );
    }

    out.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    out.dedup_by(|a, b| a.path == b.path);
    Ok(out)
}

fn sibling_targets(from: &Path) -> Result<(PathBuf, PathBuf)> {
    let parent = from
        .parent()
        .ok_or_else(|| KcError::Invalid("无效的 skill 路径".into()))?;
    if parent
        .file_name()
        .map(|s| s == "skills-disabled")
        .unwrap_or(false)
    {
        let base = parent
            .parent()
            .ok_or_else(|| KcError::Invalid("无效的 skills-disabled 路径".into()))?;
        Ok((base.join("skills"), parent.to_path_buf()))
    } else {
        // skills 与 skills-disabled 是同级兄弟目录（~/.agents/skills[-disabled]）
        let base = parent
            .parent()
            .ok_or_else(|| KcError::Invalid("无效的 skills 路径".into()))?;
        Ok((parent.to_path_buf(), base.join("skills-disabled")))
    }
}

pub fn toggle(path: &Path, enable: bool) -> Result<()> {
    if !path.is_dir() {
        return Err(KcError::NotFound(format!(
            "skill 目录不存在: {}",
            path.display()
        )));
    }
    let (skills_root, disabled_root) = sibling_targets(path)?;
    let dir_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| KcError::Invalid("无效的 skill 目录名".into()))?;

    let currently_disabled = path
        .parent()
        .map(|p| p.file_name().map(|s| s == "skills-disabled").unwrap_or(false))
        .unwrap_or(false);

    if enable && currently_disabled {
        let dest = skills_root.join(&dir_name);
        if dest.exists() {
            return Err(KcError::Invalid(format!(
                "目标位置已存在同名 skill: {}",
                dest.display()
            )));
        }
        fs::create_dir_all(&skills_root)?;
        fs::rename(path, &dest)?;
    } else if !enable && !currently_disabled {
        let dest = disabled_root.join(&dir_name);
        if dest.exists() {
            return Err(KcError::Invalid(format!(
                "禁用目录已存在同名 skill: {}",
                dest.display()
            )));
        }
        fs::create_dir_all(&disabled_root)?;
        fs::rename(path, &dest)?;
    }
    Ok(())
}

pub fn delete(path: &Path) -> Result<()> {
    if !path.is_dir() {
        return Err(KcError::NotFound(format!(
            "skill 目录不存在: {}",
            path.display()
        )));
    }
    fs_ops::move_to_trash(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_line_frontmatter() {
        let text = "---\nname: demo\ndescription: A demo skill.\n---\n\n# Body\n";
        let (n, d) = parse_frontmatter(text);
        assert_eq!(n.as_deref(), Some("demo"));
        assert_eq!(d.as_deref(), Some("A demo skill."));
    }

    #[test]
    fn parses_multiline_quoted_description() {
        let text = "---\nname: x\ndescription: 'First part\n  and second part.'\nlicense: MIT\n---\n";
        let (n, d) = parse_frontmatter(text);
        assert_eq!(n.as_deref(), Some("x"));
        let desc = d.unwrap();
        assert!(desc.starts_with("First part"));
        assert!(desc.contains("and second part."));
    }

    #[test]
    fn no_frontmatter_returns_none() {
        let (n, d) = parse_frontmatter("# Just markdown\n");
        assert!(n.is_none() && d.is_none());
    }

    #[test]
    fn toggle_moves_between_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let skills = tmp.path().join("skills");
        let target = skills.join("demo");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("SKILL.md"), "---\nname: demo\n---\n").unwrap();

        toggle(&target, false).unwrap();
        assert!(!target.exists());
        let disabled = tmp.path().join("skills-disabled").join("demo");
        assert!(disabled.exists());

        toggle(&disabled, true).unwrap();
        assert!(target.exists());
        assert!(!disabled.exists());
    }
}
