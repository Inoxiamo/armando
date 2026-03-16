#![cfg(unix)]

use std::process::Command;

mod support;

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
        .arg("scripts/package-release.sh")
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
    assert!(listing.contains(&format!("{bundle_name}/themes/default-dark.yaml")));
    assert!(listing.contains(&format!("{bundle_name}/locales/en.yaml")));
    assert!(listing.contains(&format!("{bundle_name}/assets/armando.desktop")));
    assert!(listing.contains(&format!("{bundle_name}/scripts/install.sh")));

    let _ = std::fs::remove_file(&archive_path);
    support::remove_dir_all_if_exists(&temp_dir);
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
        support::repo_root().join("scripts/release-install.sh"),
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
        support::repo_root().join("scripts/release-install.sh"),
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
fn windows_install_script_targets_appdata_and_assets() {
    let script =
        std::fs::read_to_string(support::repo_root().join("scripts/release-install.ps1")).unwrap();

    assert!(script.contains("$env:APPDATA"));
    assert!(script.contains("$env:LOCALAPPDATA"));
    assert!(script.contains("Join-Path $DataRoot \"bin\""));
    assert!(script.contains("Join-Path $DataRoot \"assets\""));
    assert!(script.contains("Copy-Item (Join-Path $BundleAssetsDir \"*\") $AssetsDir"));
}
