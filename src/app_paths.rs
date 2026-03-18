use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "armando";
const CONFIGS_DIR_NAME: &str = "configs";
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
                roots.push(root.to_path_buf());
                roots.push(root.join(CONFIGS_DIR_NAME));
            }
        }
        roots.push(parent.to_path_buf());
    }

    roots
}

fn prompt_profile_roots_from_root(root: &Path) -> Vec<PathBuf> {
    vec![root.to_path_buf(), root.join(CONFIGS_DIR_NAME)]
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
}
