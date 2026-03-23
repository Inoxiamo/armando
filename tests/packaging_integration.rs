#![cfg(unix)]

use std::process::Command;

mod support;

fn write_executable_script(path: &std::path::Path, contents: &str) {
    std::fs::write(path, contents).unwrap();
    support::make_executable(path);
}

fn package_version() -> String {
    format!("itest-{}", std::process::id())
}

fn package_archive_name(version: &str) -> String {
    format!("armando-{version}-x86_64-unknown-linux-gnu")
}

fn package_archive(version: &str) -> std::path::PathBuf {
    support::repo_root()
        .join("target/dist")
        .join(format!("{}.tar.gz", package_archive_name(version)))
}

fn bootstrap_release_dry_run_output(os: &str, arch: &str) -> String {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("bootstrap-dry-run");
    let fake_bin_dir = temp_dir.join("fake-bin");
    std::fs::create_dir_all(&fake_bin_dir).unwrap();

    write_executable_script(
        &fake_bin_dir.join("uname"),
        &format!(
            "#!/usr/bin/env bash\ncase \"$1\" in\n  -s) echo {os} ;;\n  -m) echo {arch} ;;\n  *) exit 1 ;;\nesac\n"
        ),
    );
    write_executable_script(&fake_bin_dir.join("curl"), "#!/usr/bin/env bash\nexit 0\n");

    let path = format!(
        "{}:{}",
        fake_bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = Command::new("bash")
        .arg("scripts/release/bootstrap-release.sh")
        .arg("1.2.3")
        .current_dir(support::repo_root())
        .env("ARMANDO_INSTALL_DRY_RUN", "1")
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "bootstrap dry run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn package_release_bundle_contains_runtime_assets() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("package");
    let binary_path = temp_dir.join("armando");
    std::fs::write(&binary_path, "#!/usr/bin/env bash\necho armando-test\n").unwrap();
    support::make_executable(&binary_path);

    let version = package_version();
    let archive_path = package_archive(&version);
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    if archive_path.exists() {
        std::fs::remove_file(&archive_path).unwrap();
    }

    let output = Command::new("bash")
        .arg("scripts/release/package-release.sh")
        .arg(&binary_path)
        .arg("x86_64-unknown-linux-gnu")
        .arg(&version)
        .current_dir(support::repo_root())
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "packaging failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(archive_path.exists());

    let listing = Command::new("tar")
        .arg("-tzf")
        .arg(&archive_path)
        .output()
        .unwrap();
    assert!(listing.status.success());
    let listing = String::from_utf8(listing.stdout).unwrap();
    let bundle_name = package_archive_name(&version);

    assert!(listing.contains(&format!("{bundle_name}/armando")));
    assert!(listing.contains(&format!("{bundle_name}/configs/default.yaml")));
    assert!(listing.contains(&format!("{bundle_name}/prompt-tags.yaml")));
    assert!(listing.contains(&format!("{bundle_name}/generic-prompts.yaml")));
    assert!(listing.contains(&format!("{bundle_name}/themes/default-dark.yaml")));
    assert!(listing.contains(&format!("{bundle_name}/locales/en.yaml")));
    assert!(listing.contains(&format!("{bundle_name}/assets/armando.desktop")));
    assert!(listing.contains(&format!("{bundle_name}/scripts/install.sh")));

    let _ = std::fs::remove_file(&archive_path);
    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn bootstrap_release_dry_run_reports_linux_and_macos_targets() {
    let linux_output = bootstrap_release_dry_run_output("Linux", "x86_64");
    assert!(linux_output.contains("x86_64-unknown-linux-gnu"));
    assert!(linux_output.contains("armando-1.2.3-x86_64-unknown-linux-gnu.tar.gz"));
    assert!(linux_output.contains("Dry run for armando bootstrap"));

    let macos_output = bootstrap_release_dry_run_output("Darwin", "arm64");
    assert!(macos_output.contains("aarch64-apple-darwin"));
    assert!(macos_output.contains("armando-1.2.3-aarch64-apple-darwin.tar.gz"));
    assert!(macos_output.contains("Dry run for armando bootstrap"));
}

#[test]
fn bundled_install_script_populates_home_profile_layout() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("bundle");
    let bundle_root = temp_dir.join("bundle");
    let scripts_dir = bundle_root.join("scripts");
    let configs_dir = bundle_root.join("configs");
    let themes_dir = bundle_root.join("themes");
    let locales_dir = bundle_root.join("locales");
    let assets_dir = bundle_root.join("assets");
    std::fs::create_dir_all(&scripts_dir).unwrap();
    std::fs::create_dir_all(&configs_dir).unwrap();
    std::fs::create_dir_all(&themes_dir).unwrap();
    std::fs::create_dir_all(&locales_dir).unwrap();
    std::fs::create_dir_all(&assets_dir).unwrap();

    let binary_path = bundle_root.join("armando");
    std::fs::write(&binary_path, "#!/usr/bin/env bash\necho installed\n").unwrap();
    support::make_executable(&binary_path);
    std::fs::copy(
        support::repo_root().join("scripts/release/release-install.sh"),
        scripts_dir.join("install.sh"),
    )
    .unwrap();
    support::make_executable(&scripts_dir.join("install.sh"));
    std::fs::copy(
        support::repo_root().join("configs/default.yaml"),
        configs_dir.join("default.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("prompt-tags.yaml"),
        bundle_root.join("prompt-tags.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("generic-prompts.yaml"),
        bundle_root.join("generic-prompts.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("themes/default-dark.yaml"),
        themes_dir.join("default-dark.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("locales/en.yaml"),
        locales_dir.join("en.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("assets/armando.svg"),
        assets_dir.join("armando.svg"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("assets/armando.desktop"),
        assets_dir.join("armando.desktop"),
    )
    .unwrap();

    let home_dir = temp_dir.join("home");
    std::fs::create_dir_all(&home_dir).unwrap();
    let output = Command::new("bash")
        .arg("scripts/install.sh")
        .current_dir(&bundle_root)
        .env("HOME", &home_dir)
        .env("XDG_CONFIG_HOME", home_dir.join(".config"))
        .env("XDG_DATA_HOME", home_dir.join(".local/share"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(home_dir.join(".local/bin/armando").exists());
    assert!(home_dir
        .join(".config/armando/configs/default.yaml")
        .exists());
    assert!(home_dir.join(".config/armando/prompt-tags.yaml").exists());
    assert!(home_dir
        .join(".config/armando/generic-prompts.yaml")
        .exists());
    assert!(home_dir
        .join(".config/armando/themes/default-dark.yaml")
        .exists());
    assert!(home_dir.join(".config/armando/locales/en.yaml").exists());
    assert!(home_dir
        .join(".local/share/armando/assets/armando.svg")
        .exists());
    assert!(home_dir
        .join(".local/share/icons/hicolor/scalable/apps/armando.svg")
        .exists());

    let desktop_entry =
        std::fs::read_to_string(home_dir.join(".local/share/applications/armando.desktop"))
            .unwrap();
    assert!(desktop_entry.contains(home_dir.to_string_lossy().as_ref()));
    assert!(!desktop_entry.contains("${HOME}"));

    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn bundled_install_script_uses_macos_application_support_layout() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("bundle-macos");
    let bundle_root = temp_dir.join("bundle");
    let scripts_dir = bundle_root.join("scripts");
    let configs_dir = bundle_root.join("configs");
    let themes_dir = bundle_root.join("themes");
    let locales_dir = bundle_root.join("locales");
    let assets_dir = bundle_root.join("assets");
    std::fs::create_dir_all(&scripts_dir).unwrap();
    std::fs::create_dir_all(&configs_dir).unwrap();
    std::fs::create_dir_all(&themes_dir).unwrap();
    std::fs::create_dir_all(&locales_dir).unwrap();
    std::fs::create_dir_all(&assets_dir).unwrap();

    let binary_path = bundle_root.join("armando");
    std::fs::write(&binary_path, "#!/usr/bin/env bash\necho installed\n").unwrap();
    support::make_executable(&binary_path);
    std::fs::copy(
        support::repo_root().join("scripts/release/release-install.sh"),
        scripts_dir.join("install.sh"),
    )
    .unwrap();
    support::make_executable(&scripts_dir.join("install.sh"));
    std::fs::copy(
        support::repo_root().join("configs/default.yaml"),
        configs_dir.join("default.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("prompt-tags.yaml"),
        bundle_root.join("prompt-tags.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("generic-prompts.yaml"),
        bundle_root.join("generic-prompts.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("themes/default-dark.yaml"),
        themes_dir.join("default-dark.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("locales/en.yaml"),
        locales_dir.join("en.yaml"),
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("assets/armando.svg"),
        assets_dir.join("armando.svg"),
    )
    .unwrap();

    let home_dir = temp_dir.join("home");
    std::fs::create_dir_all(&home_dir).unwrap();
    let output = Command::new("bash")
        .arg("scripts/install.sh")
        .current_dir(&bundle_root)
        .env("HOME", &home_dir)
        .env("ARMANDO_INSTALL_OS", "Darwin")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_DATA_HOME")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(home_dir.join(".local/bin/armando").exists());
    assert!(home_dir
        .join("Library/Application Support/armando/configs/default.yaml")
        .exists());
    assert!(home_dir
        .join("Library/Application Support/armando/prompt-tags.yaml")
        .exists());
    assert!(home_dir
        .join("Library/Application Support/armando/generic-prompts.yaml")
        .exists());
    assert!(home_dir
        .join("Library/Application Support/armando/themes/default-dark.yaml")
        .exists());
    assert!(home_dir
        .join("Library/Application Support/armando/locales/en.yaml")
        .exists());
    assert!(home_dir
        .join("Library/Application Support/armando/assets/armando.svg")
        .exists());

    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn bundled_install_script_preserves_existing_config_theme_and_locale_files() {
    let _guard = support::test_lock();
    let temp_dir = support::unique_temp_dir("bundle-preserve");
    let bundle_root = temp_dir.join("bundle");
    let scripts_dir = bundle_root.join("scripts");
    let configs_dir = bundle_root.join("configs");
    let themes_dir = bundle_root.join("themes");
    let locales_dir = bundle_root.join("locales");
    let assets_dir = bundle_root.join("assets");
    std::fs::create_dir_all(&scripts_dir).unwrap();
    std::fs::create_dir_all(&configs_dir).unwrap();
    std::fs::create_dir_all(&themes_dir).unwrap();
    std::fs::create_dir_all(&locales_dir).unwrap();
    std::fs::create_dir_all(&assets_dir).unwrap();

    let binary_path = bundle_root.join("armando");
    std::fs::write(&binary_path, "#!/usr/bin/env bash\necho installed\n").unwrap();
    support::make_executable(&binary_path);
    std::fs::copy(
        support::repo_root().join("scripts/release/release-install.sh"),
        scripts_dir.join("install.sh"),
    )
    .unwrap();
    support::make_executable(&scripts_dir.join("install.sh"));
    std::fs::write(
        configs_dir.join("default.yaml"),
        "default_backend: ollama\n",
    )
    .unwrap();
    std::fs::write(
        bundle_root.join("prompt-tags.yaml"),
        "tags:\n  ITA: bundled\n",
    )
    .unwrap();
    std::fs::write(
        bundle_root.join("generic-prompts.yaml"),
        "tags:\n  CMD:\n    instruction: bundled\n    strip_header: true\n",
    )
    .unwrap();
    std::fs::write(
        themes_dir.join("default-dark.yaml"),
        "name: bundled-theme\n",
    )
    .unwrap();
    std::fs::write(
        locales_dir.join("en.yaml"),
        "code: en\nname: English\nstrings: {}\n",
    )
    .unwrap();
    std::fs::copy(
        support::repo_root().join("assets/armando.svg"),
        assets_dir.join("armando.svg"),
    )
    .unwrap();

    let home_dir = temp_dir.join("home");
    let config_root = home_dir.join(".config/armando");
    let data_root = home_dir.join(".local/share/armando");
    std::fs::create_dir_all(config_root.join("configs")).unwrap();
    std::fs::create_dir_all(config_root.join("themes")).unwrap();
    std::fs::create_dir_all(config_root.join("locales")).unwrap();
    std::fs::create_dir_all(data_root.join("assets")).unwrap();
    std::fs::write(
        config_root.join("configs/default.yaml"),
        "default_backend: chatgpt\n",
    )
    .unwrap();
    std::fs::write(
        config_root.join("themes/default-dark.yaml"),
        "name: user-theme\n",
    )
    .unwrap();
    std::fs::write(
        config_root.join("locales/en.yaml"),
        "code: en\nname: User English\nstrings: {}\n",
    )
    .unwrap();
    std::fs::write(config_root.join("prompt-tags.yaml"), "tags:\n  ITA: user\n").unwrap();
    std::fs::write(
        config_root.join("generic-prompts.yaml"),
        "tags:\n  CMD:\n    instruction: user\n    strip_header: true\n",
    )
    .unwrap();

    let output = Command::new("bash")
        .arg("scripts/install.sh")
        .current_dir(&bundle_root)
        .env("HOME", &home_dir)
        .env("XDG_CONFIG_HOME", home_dir.join(".config"))
        .env("XDG_DATA_HOME", home_dir.join(".local/share"))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(config_root.join("configs/default.yaml")).unwrap(),
        "default_backend: chatgpt\n"
    );
    assert_eq!(
        std::fs::read_to_string(config_root.join("themes/default-dark.yaml")).unwrap(),
        "name: user-theme\n"
    );
    assert_eq!(
        std::fs::read_to_string(config_root.join("locales/en.yaml")).unwrap(),
        "code: en\nname: User English\nstrings: {}\n"
    );
    assert_eq!(
        std::fs::read_to_string(config_root.join("prompt-tags.yaml")).unwrap(),
        "tags:\n  ITA: user\n"
    );
    assert_eq!(
        std::fs::read_to_string(config_root.join("generic-prompts.yaml")).unwrap(),
        "tags:\n  CMD:\n    instruction: user\n    strip_header: true\n"
    );

    support::remove_dir_all_if_exists(&temp_dir);
}

#[test]
fn windows_install_script_targets_appdata_and_assets() {
    let script =
        std::fs::read_to_string(support::repo_root().join("scripts/release/release-install.ps1"))
            .unwrap();

    assert!(script.contains("$env:APPDATA"));
    assert!(script.contains("$env:LOCALAPPDATA"));
    assert!(script.contains("Join-Path $DataRoot \"bin\""));
    assert!(script.contains("Join-Path $DataRoot \"assets\""));
    assert!(script.contains("Copy-Item (Join-Path $BundleAssetsDir \"*\") $AssetsDir"));
    assert!(script.contains("Install-ConfigFile"));
    assert!(script.contains("prompt-tags.yaml"));
    assert!(script.contains("generic-prompts.yaml"));
}
