use armando::i18n::{available_locales, I18n};
use armando::theme::available_theme_names;

mod support;

#[test]
fn themes_and_locales_are_discovered_from_working_directory() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("resources");
    let themes_dir = temp_dir.join("themes");
    let locales_dir = temp_dir.join("locales");
    std::fs::create_dir_all(&themes_dir).unwrap();
    std::fs::create_dir_all(&locales_dir).unwrap();

    std::fs::write(
        themes_dir.join("custom.yaml"),
        "\
name: Custom
window_fill: \"#010203\"
panel_fill: \"#111213\"
panel_fill_soft: \"#212223\"
panel_fill_raised: \"#313233\"
accent_color: \"#414243\"
accent_hover_color: \"#515253\"
accent_text_color: \"#616263\"
text_color: \"#717273\"
weak_text_color: \"#818283\"
border_color: \"#919293\"
danger_color: \"#A1A2A3\"
",
    )
    .unwrap();
    std::fs::write(
        locales_dir.join("en.yaml"),
        "\
code: en
name: English
strings:
  hello: Hello
  fallback_only: Fallback text
",
    )
    .unwrap();
    std::fs::write(
        locales_dir.join("it.yaml"),
        "\
code: it
name: Italiano
strings:
  hello: Ciao
",
    )
    .unwrap();

    let previous_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&temp_dir).unwrap();

    let theme_names = available_theme_names().unwrap();
    assert!(theme_names.contains(&"custom".to_string()));

    let locale_names = available_locales()
        .unwrap()
        .into_iter()
        .map(|locale| locale.name)
        .collect::<Vec<_>>();
    assert!(locale_names.contains(&"English".to_string()));
    assert!(locale_names.contains(&"Italiano".to_string()));

    let i18n = I18n::load("it").unwrap();
    assert_eq!(i18n.code(), "it");
    assert_eq!(i18n.language_name(), "Italiano");
    assert_eq!(i18n.tr("hello"), "Ciao");
    assert_eq!(i18n.tr("fallback_only"), "Fallback text");
    assert_eq!(i18n.tr("missing_key"), "missing_key");

    std::env::set_current_dir(previous_dir).unwrap();
    support::remove_dir_all_if_exists(&temp_dir);
}
