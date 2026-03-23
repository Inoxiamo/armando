use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "armando";
const CONFIGS_DIR_NAME: &str = "configs";
const PROMPTS_DIR_NAME: &str = "prompts";
const THEMES_DIR_NAME: &str = "themes";
const LOCALES_DIR_NAME: &str = "locales";
const DEFAULT_CONFIG_FILE_NAME: &str = "default.yaml";
const LEGACY_CONFIG_FILE_NAME: &str = "config.yaml";
const PROMPT_TAGS_FILE_NAME: &str = "prompt-tags.yaml";
const GENERIC_PROMPTS_FILE_NAME: &str = "generic-prompts.yaml";

pub fn central_config_root() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join(APP_DIR_NAME))
}

pub fn data_root() -> Option<PathBuf> {
    dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .map(|dir| dir.join(APP_DIR_NAME))
}

pub fn history_file_path() -> anyhow::Result<PathBuf> {
    data_root()
        .or_else(central_config_root)
        .map(|dir| dir.join("history").join("history.jsonl"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine a writable application data directory"))
}

pub fn debug_log_file_path() -> anyhow::Result<PathBuf> {
    data_root()
        .or_else(central_config_root)
        .map(|dir| dir.join("logs").join("debug.jsonl"))
        .ok_or_else(|| anyhow::anyhow!("Could not determine a writable application log directory"))
}

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    central_config_root()
        .map(|dir| dir.join(CONFIGS_DIR_NAME).join(DEFAULT_CONFIG_FILE_NAME))
        .ok_or_else(|| {
            anyhow::anyhow!("Could not determine a writable application config directory")
        })
}

pub fn bundled_default_config_template_path() -> anyhow::Result<Option<PathBuf>> {
    bundled_config_template_path(DEFAULT_CONFIG_FILE_NAME)
}

pub fn candidate_config_paths() -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    if let Ok(explicit) = std::env::var("ARMANDO_CONFIG") {
        let explicit_path = PathBuf::from(explicit);
        push_unique(&mut paths, explicit_path);
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            collect_config_candidates_for_root(parent, &mut paths);
            if let Some(grandparent) = parent.parent().and_then(|path| path.parent()) {
                collect_config_candidates_for_root(grandparent, &mut paths);
            }
        }
    }

    collect_config_candidates_for_root(&std::env::current_dir()?, &mut paths);

    if let Some(root) = central_config_root() {
        collect_config_candidates_for_root(&root, &mut paths);
    }

    Ok(paths)
}

pub fn discover_config_template_names() -> anyhow::Result<Vec<String>> {
    let mut names = discover_named_files(CONFIGS_DIR_NAME, "yaml")?;
    names.retain(|name| name != "config");
    Ok(names)
}

pub fn bundled_config_template_path(template_name: &str) -> anyhow::Result<Option<PathBuf>> {
    let target = default_config_path().ok();

    for path in candidate_config_template_paths(template_name)? {
        if !path.exists() {
            continue;
        }
        if target.as_ref().is_some_and(|candidate| candidate == &path) {
            continue;
        }
        return Ok(Some(path));
    }

    Ok(None)
}

pub fn candidate_config_template_paths(template_name: &str) -> anyhow::Result<Vec<PathBuf>> {
    let file_name = format!("{template_name}.yaml");
    let mut paths = Vec::new();

    if let Ok(explicit) = std::env::var("ARMANDO_CONFIG") {
        let explicit_path = PathBuf::from(explicit);
        push_unique(&mut paths, explicit_path.with_file_name(&file_name));
        push_unique(
            &mut paths,
            explicit_path
                .with_file_name(CONFIGS_DIR_NAME)
                .join(&file_name),
        );
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            collect_named_config_candidates_for_root(parent, &file_name, &mut paths);
            if let Some(grandparent) = parent.parent().and_then(|path| path.parent()) {
                collect_named_config_candidates_for_root(grandparent, &file_name, &mut paths);
            }
        }
    }

    collect_named_config_candidates_for_root(&std::env::current_dir()?, &file_name, &mut paths);

    if let Some(root) = central_config_root() {
        collect_named_config_candidates_for_root(&root, &file_name, &mut paths);
    }

    Ok(paths)
}

pub fn candidate_theme_paths(
    name: &str,
    loaded_from: Option<&Path>,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let file_name = format!("{name}.yaml");

    if let Some(config_path) = loaded_from {
        for root in theme_roots_from_config(config_path) {
            push_unique(&mut paths, root.join(&file_name));
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            for root in theme_roots_from_root(parent) {
                push_unique(&mut paths, root.join(&file_name));
            }
            if let Some(grandparent) = parent.parent().and_then(|path| path.parent()) {
                for root in theme_roots_from_root(grandparent) {
                    push_unique(&mut paths, root.join(&file_name));
                }
            }
        }
    }

    for root in theme_roots_from_root(&std::env::current_dir()?) {
        push_unique(&mut paths, root.join(&file_name));
    }

    if let Some(root) = central_config_root() {
        for dir in theme_roots_from_root(&root) {
            push_unique(&mut paths, dir.join(&file_name));
        }
    }

    Ok(paths)
}

pub fn candidate_locale_paths(code: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let file_name = format!("{code}.yaml");

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            for root in locale_roots_from_root(parent) {
                push_unique(&mut paths, root.join(&file_name));
            }
            if let Some(grandparent) = parent.parent().and_then(|path| path.parent()) {
                for root in locale_roots_from_root(grandparent) {
                    push_unique(&mut paths, root.join(&file_name));
                }
            }
        }
    }

    for root in locale_roots_from_root(&std::env::current_dir()?) {
        push_unique(&mut paths, root.join(&file_name));
    }

    if let Some(root) = central_config_root() {
        for dir in locale_roots_from_root(&root) {
            push_unique(&mut paths, dir.join(&file_name));
        }
    }

    Ok(paths)
}

pub fn candidate_prompt_tags_paths(loaded_from: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    candidate_prompt_profile_paths(PROMPT_TAGS_FILE_NAME, loaded_from)
}

pub fn candidate_generic_prompt_paths(loaded_from: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    candidate_prompt_profile_paths(GENERIC_PROMPTS_FILE_NAME, loaded_from)
}

pub fn discover_named_files(dir_name: &str, extension: &str) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    let ext = extension.trim_start_matches('.');

    for root in candidate_resource_roots(dir_name)? {
        if !root.exists() {
            continue;
        }

        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some(ext) {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
                let stem = stem.to_string();
                if !names.iter().any(|name| name == &stem) {
                    names.push(stem);
                }
            }
        }
    }

    names.sort();
    Ok(names)
}

fn collect_config_candidates_for_root(root: &Path, paths: &mut Vec<PathBuf>) {
    push_unique(
        paths,
        root.join(CONFIGS_DIR_NAME).join(DEFAULT_CONFIG_FILE_NAME),
    );
    push_unique(paths, root.join(LEGACY_CONFIG_FILE_NAME));
}

fn collect_named_config_candidates_for_root(
    root: &Path,
    file_name: &str,
    paths: &mut Vec<PathBuf>,
) {
    push_unique(paths, root.join(CONFIGS_DIR_NAME).join(file_name));
    push_unique(paths, root.join(file_name));
}

fn theme_roots_from_config(config_path: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(parent) = config_path.parent() {
        if parent.file_name().and_then(|name| name.to_str()) == Some(CONFIGS_DIR_NAME) {
            if let Some(root) = parent.parent() {
                roots.push(root.join(THEMES_DIR_NAME));
            }
        }
        roots.push(parent.join(THEMES_DIR_NAME));
        roots.push(parent.to_path_buf());
    }

    roots
}

fn theme_roots_from_root(root: &Path) -> Vec<PathBuf> {
    vec![root.join(THEMES_DIR_NAME), root.to_path_buf()]
}

fn locale_roots_from_root(root: &Path) -> Vec<PathBuf> {
    vec![root.join(LOCALES_DIR_NAME), root.to_path_buf()]
}

fn candidate_prompt_profile_paths(
    file_name: &str,
    loaded_from: Option<&Path>,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    if let Some(config_path) = loaded_from {
        for root in prompt_profile_roots_from_config(config_path) {
            push_unique(&mut paths, root.join(file_name));
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            for root in prompt_profile_roots_from_root(parent) {
                push_unique(&mut paths, root.join(file_name));
            }
            if let Some(grandparent) = parent.parent().and_then(|path| path.parent()) {
                for root in prompt_profile_roots_from_root(grandparent) {
                    push_unique(&mut paths, root.join(file_name));
                }
            }
        }
    }

    for root in prompt_profile_roots_from_root(&std::env::current_dir()?) {
        push_unique(&mut paths, root.join(file_name));
    }

    if let Some(root) = central_config_root() {
        for dir in prompt_profile_roots_from_root(&root) {
            push_unique(&mut paths, dir.join(file_name));
        }
    }

    Ok(paths)
}

fn prompt_profile_roots_from_config(config_path: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Some(parent) = config_path.parent() {
        if parent.file_name().and_then(|name| name.to_str()) == Some(CONFIGS_DIR_NAME) {
            if let Some(root) = parent.parent() {
                for candidate in prompt_profile_roots_from_root(root) {
                    roots.push(candidate);
                }
            }
        } else {
            for candidate in prompt_profile_roots_from_root(parent) {
                roots.push(candidate);
            }
        }
        roots.push(parent.join(PROMPTS_DIR_NAME));
        roots.push(parent.to_path_buf());
    }

    roots
}

fn prompt_profile_roots_from_root(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join(CONFIGS_DIR_NAME).join(PROMPTS_DIR_NAME),
        root.to_path_buf(),
        root.join(CONFIGS_DIR_NAME),
    ]
}

fn candidate_resource_roots(dir_name: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut roots = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            push_unique(&mut roots, parent.join(dir_name));
            if let Some(grandparent) = parent.parent().and_then(|path| path.parent()) {
                push_unique(&mut roots, grandparent.join(dir_name));
            }
        }
    }

    push_unique(&mut roots, std::env::current_dir()?.join(dir_name));

    if let Some(root) = central_config_root() {
        push_unique(&mut roots, root.join(dir_name));
    }

    Ok(roots)
}

fn push_unique(paths: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !paths.iter().any(|path| path == &candidate) {
        paths.push(candidate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_candidates_include_configs_directory() {
        let root = Path::new("/tmp/armando");
        let mut candidates = Vec::new();

        collect_config_candidates_for_root(root, &mut candidates);

        assert!(candidates.contains(&PathBuf::from("/tmp/armando/configs/default.yaml")));
        assert!(candidates.contains(&PathBuf::from("/tmp/armando/config.yaml")));
    }

    #[test]
    fn config_relative_theme_roots_include_central_theme_dir() {
        let roots = theme_roots_from_config(Path::new("/tmp/armando/configs/default.yaml"));

        assert!(roots.contains(&PathBuf::from("/tmp/armando/themes")));
    }

    #[test]
    fn discover_config_template_names_includes_bundled_profiles() {
        let names = discover_config_template_names().unwrap();

        assert!(names.contains(&"default".to_string()));
        assert!(names.contains(&"local".to_string()));
        assert!(names.contains(&"work".to_string()));
        assert!(names.contains(&"personal".to_string()));
        assert!(names.contains(&"beta".to_string()));
    }

    #[test]
    fn prompt_profile_roots_prefer_configs_prompts_then_legacy_root() {
        let roots = prompt_profile_roots_from_root(Path::new("/tmp/armando"));
        assert_eq!(roots[0], PathBuf::from("/tmp/armando/configs/prompts"));
        assert_eq!(roots[1], PathBuf::from("/tmp/armando"));
    }

    #[test]
    fn config_relative_prompt_roots_include_configs_prompts_first() {
        let roots =
            prompt_profile_roots_from_config(Path::new("/tmp/armando/configs/default.yaml"));

        assert_eq!(roots[0], PathBuf::from("/tmp/armando/configs/prompts"));
        assert!(roots.contains(&PathBuf::from("/tmp/armando")));
    }
}
