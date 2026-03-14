use crate::config::Config;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::process::Command;
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

fn parse_hotkey(hotkey_str: &str) -> anyhow::Result<HotKey> {
    // Basic parser for something like <ctrl>+<space> or <ctrl>+<alt>+a
    let s = hotkey_str.to_lowercase();
    let mut modifiers = Modifiers::empty();

    if s.contains("<ctrl>") || s.contains("ctrl") {
        modifiers |= Modifiers::CONTROL;
    }
    if s.contains("<alt>") || s.contains("alt") {
        modifiers |= Modifiers::ALT;
    }
    if s.contains("<shift>") || s.contains("shift") {
        modifiers |= Modifiers::SHIFT;
    }
    if s.contains("<super>") || s.contains("<cmd>") || s.contains("super") || s.contains("cmd") {
        modifiers |= Modifiers::META;
    }

    let code = if s.contains("space") {
        Code::Space
    } else if s.ends_with('a') {
        Code::KeyA
    } else if s.ends_with('b') {
        Code::KeyB
    } else {
        // Default to space if we can't parse it well for now
        Code::Space
    };

    Ok(HotKey::new(Some(modifiers), code))
}

pub fn run(config: Config) -> anyhow::Result<()> {
    let manager = GlobalHotKeyManager::new()
        .map_err(|e| anyhow::anyhow!("Failed to init hotkey manager: {:?}", e))?;

    let hotkey =
        parse_hotkey(&config.hotkey).unwrap_or(HotKey::new(Some(Modifiers::CONTROL), Code::Space));
    manager
        .register(hotkey)
        .map_err(|e| anyhow::anyhow!("Failed to register hotkey: {:?}", e))?;

    log::info!("✦ AI Popup Daemon started.");
    log::info!("  Hotkey   : {}", config.hotkey);
    log::info!("  Backend  : {}", config.default_backend);
    log::info!("  Press Ctrl+C to quit.");

    let receiver = GlobalHotKeyEvent::receiver();

    loop {
        if let Ok(_event) = receiver.try_recv() {
            log::info!("Hotkey triggered. Spawning UI...");
            spawn_ui()?;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn spawn_ui() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let mut cmd = Command::new(exe);
    cmd.arg("--ui");

    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Failed to spawn UI process: {:?}", e);
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hotkey_supports_ctrl_space() {
        let hotkey = parse_hotkey("<ctrl>+<space>").unwrap();
        assert_eq!(hotkey.key, Code::Space);
        assert_eq!(hotkey.mods, Modifiers::CONTROL);
    }

    #[test]
    fn parse_hotkey_supports_meta_and_letter() {
        let hotkey = parse_hotkey("<cmd>+a").unwrap();
        assert_eq!(hotkey.key, Code::KeyA);
        assert_eq!(hotkey.mods, Modifiers::SUPER);
    }

    #[test]
    fn parse_hotkey_defaults_unknown_keys_to_space() {
        let hotkey = parse_hotkey("<ctrl>+z").unwrap();
        assert_eq!(hotkey.key, Code::Space);
        assert_eq!(hotkey.mods, Modifiers::CONTROL);
    }
}
