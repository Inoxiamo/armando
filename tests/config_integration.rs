use armando::app_paths;
use armando::config::{Config, RagEngine, RagMode};

mod support;

#[test]
fn config_load_prefers_explicit_env_and_save_roundtrips() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("config");
    let config_path = temp_dir.join("config.yaml");
    std::fs::write(
        &config_path,
        "\
default_backend: chatgpt
auto_read_selection: false
theme:
  name: nerv-hud
ui:
  language: it
  window_height: 700
history:
  enabled: true
logging:
  enabled: true
",
    )
    .unwrap();

    let previous = std::env::var_os("ARMANDO_CONFIG");
    std::env::set_var("ARMANDO_CONFIG", &config_path);

    let mut config = Config::load().unwrap();
    assert_eq!(config.default_backend, "chatgpt");
    assert!(!config.auto_read_selection);
    assert_eq!(config.theme.name, "nerv-hud");
    assert_eq!(config.ui.language, "it");
    assert_eq!(config.ui.window_height, 700.0);
    assert!(config.history.enabled);
    assert!(config.logging.enabled);
    assert_eq!(config.loaded_from.as_deref(), Some(config_path.as_path()));

    config.default_backend = "gemini".to_string();
    config.save().unwrap();

    let persisted = std::fs::read_to_string(&config_path).unwrap();
    assert!(persisted.contains("default_backend: gemini"));
    assert!(!persisted.contains("loaded_from"));

    match previous {
        Some(value) => std::env::set_var("ARMANDO_CONFIG", value),
        None => std::env::remove_var("ARMANDO_CONFIG"),
    }

    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn config_load_falls_back_to_built_in_defaults_when_config_is_missing() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("config-fallback");
    let config_path = temp_dir.join("missing").join("config.yaml");
    let xdg_config_home = temp_dir.join("xdg-config");
    let xdg_data_home = temp_dir.join("xdg-data");
    std::fs::create_dir_all(&xdg_config_home).unwrap();
    std::fs::create_dir_all(&xdg_data_home).unwrap();

    let previous_config = std::env::var_os("ARMANDO_CONFIG");
    let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
    let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
    let previous_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();
    std::env::set_var("ARMANDO_CONFIG", &config_path);
    std::env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
    std::env::set_var("XDG_DATA_HOME", &xdg_data_home);

    let config = Config::load().unwrap();
    assert_eq!(config.loaded_from, None);
    assert_eq!(config.default_backend, "gemini");
    assert!(config.auto_read_selection);
    assert_eq!(config.theme.name, "default-dark");
    assert_eq!(config.ui.language, "en");
    assert_eq!(config.ui.window_height, 600.0);
    assert!(!config.history.enabled);
    assert!(!config.logging.enabled);

    match previous_config {
        Some(value) => std::env::set_var("ARMANDO_CONFIG", value),
        None => std::env::remove_var("ARMANDO_CONFIG"),
    }
    match previous_xdg_config {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    match previous_xdg_data {
        Some(value) => std::env::set_var("XDG_DATA_HOME", value),
        None => std::env::remove_var("XDG_DATA_HOME"),
    }
    std::env::set_current_dir(previous_dir).unwrap();

    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn config_load_uses_bundled_default_template_before_built_in_defaults() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("config-template-fallback");
    let template_dir = temp_dir.join("configs");
    let config_path = temp_dir.join("missing").join("config.yaml");
    let xdg_config_home = temp_dir.join("xdg-config");
    let xdg_data_home = temp_dir.join("xdg-data");
    std::fs::create_dir_all(&template_dir).unwrap();
    std::fs::create_dir_all(&xdg_config_home).unwrap();
    std::fs::create_dir_all(&xdg_data_home).unwrap();
    std::fs::write(
        template_dir.join("default.yaml"),
        "\
default_backend: chatgpt
auto_read_selection: false
theme:
  name: nerv-hud
ui:
  language: it
  window_height: 640
history:
  enabled: true
logging:
  enabled: false
",
    )
    .unwrap();

    let previous_config = std::env::var_os("ARMANDO_CONFIG");
    let previous_xdg_config = std::env::var_os("XDG_CONFIG_HOME");
    let previous_xdg_data = std::env::var_os("XDG_DATA_HOME");
    let previous_dir = std::env::current_dir().unwrap();

    std::env::set_current_dir(&temp_dir).unwrap();
    std::env::set_var("ARMANDO_CONFIG", &config_path);
    std::env::set_var("XDG_CONFIG_HOME", &xdg_config_home);
    std::env::set_var("XDG_DATA_HOME", &xdg_data_home);

    let config = Config::load().unwrap();
    assert_eq!(
        config.loaded_from.as_deref(),
        Some(template_dir.join("default.yaml").as_path())
    );
    assert_eq!(config.default_backend, "chatgpt");
    assert!(!config.auto_read_selection);
    assert_eq!(config.theme.name, "nerv-hud");
    assert_eq!(config.ui.language, "it");
    assert_eq!(config.ui.window_height, 640.0);
    assert!(config.history.enabled);
    assert!(!config.logging.enabled);

    match previous_config {
        Some(value) => std::env::set_var("ARMANDO_CONFIG", value),
        None => std::env::remove_var("ARMANDO_CONFIG"),
    }
    match previous_xdg_config {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    match previous_xdg_data {
        Some(value) => std::env::set_var("XDG_DATA_HOME", value),
        None => std::env::remove_var("XDG_DATA_HOME"),
    }
    std::env::set_current_dir(previous_dir).unwrap();

    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn config_load_reads_a_named_profile_template_when_available() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("config-template");
    let template_dir = temp_dir.join("configs");
    std::fs::create_dir_all(&template_dir).unwrap();
    std::fs::write(
        template_dir.join("qa-profile.yaml"),
        "\
default_backend: chatgpt
ui:
  language: en
history:
  enabled: true
",
    )
    .unwrap();

    let previous_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let template = Config::load_template("qa-profile").unwrap().unwrap();
    assert_eq!(
        template.loaded_from.as_deref(),
        Some(template_dir.join("qa-profile.yaml").as_path())
    );
    assert_eq!(template.default_backend, "chatgpt");
    assert_eq!(template.ui.language, "en");
    assert!(template.history.enabled);

    let names = app_paths::discover_config_template_names().unwrap();
    assert!(names.contains(&"qa-profile".to_string()));

    std::env::set_current_dir(previous_dir).unwrap();
    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn bundled_profile_templates_are_discovered_and_loadable() {
    let _guard = support::test_lock();
    let names = app_paths::discover_config_template_names().unwrap();
    assert!(names.contains(&"default".to_string()));
    assert!(names.contains(&"local".to_string()));
    assert!(names.contains(&"work".to_string()));
    assert!(names.contains(&"personal".to_string()));
    assert!(names.contains(&"beta".to_string()));

    let personal = Config::load_template("personal").unwrap().unwrap();
    assert_eq!(personal.default_backend, "chatgpt");
    assert_eq!(personal.ui.language, "it");
    assert!(personal.history.enabled);
    assert_eq!(personal.theme.name, "nerv-hud");

    let beta = Config::load_template("beta").unwrap().unwrap();
    assert_eq!(beta.default_backend, "gemini");
    assert_eq!(beta.ui.language, "en");
    assert!(beta.logging.enabled);
    assert!(!beta.history.enabled);
}

#[test]
fn config_save_roundtrips_rag_mode() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("config-rag-roundtrip");
    let config_path = temp_dir.join("config.yaml");
    std::fs::write(
        &config_path,
        "\
default_backend: ollama
rag:
  enabled: true
  engine: langchain
  mode: keyword
  langchain_base_url: http://127.0.0.1:18001
  langchain_timeout_ms: 5000
  langchain_retry_count: 2
",
    )
    .unwrap();

    let previous = std::env::var_os("ARMANDO_CONFIG");
    std::env::set_var("ARMANDO_CONFIG", &config_path);

    let mut config = Config::load().unwrap();
    assert_eq!(config.rag.engine, RagEngine::Langchain);
    assert_eq!(config.rag.mode, RagMode::Keyword);
    assert_eq!(config.rag.langchain_base_url, "http://127.0.0.1:18001");
    assert_eq!(config.rag.langchain_timeout_ms, 5_000);
    assert_eq!(config.rag.langchain_retry_count, 2);

    config.rag.mode = RagMode::Hybrid;
    config.rag.engine = RagEngine::Simple;
    config.save().unwrap();

    let reloaded = Config::load().unwrap();
    assert_eq!(reloaded.rag.engine, RagEngine::Simple);
    assert_eq!(reloaded.rag.mode, RagMode::Hybrid);

    match previous {
        Some(value) => std::env::set_var("ARMANDO_CONFIG", value),
        None => std::env::remove_var("ARMANDO_CONFIG"),
    }
    support::remove_dir_all_if_exists(&temp_dir);
}
