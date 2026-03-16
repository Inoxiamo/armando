use armando::config::Config;

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
