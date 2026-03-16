# Shortcuts

This page explains the practical way to launch `armando` with a keyboard combination on Linux, macOS, and Windows.

## What Works Today

`armando` can be launched with a keyboard shortcut on all major desktop operating systems, but the shortcut is still registered by the OS, not by the app itself.

The portable setup is:

1. install `armando`
2. locate the installed executable
3. create an OS-level shortcut that launches it
4. assign the key combination there

Installed executable paths:

- Linux: `~/.local/bin/armando`
- macOS: `~/.local/bin/armando`
- Windows: `%LOCALAPPDATA%\armando\bin\armando.exe`

## Linux

Most desktop environments support a direct custom command shortcut.

Command to launch:

```bash
~/.local/bin/armando
```

Common places to configure it:

- GNOME: `Settings` -> `Keyboard` -> `Keyboard Shortcuts` -> `Custom Shortcuts`
- KDE Plasma: `System Settings` -> `Shortcuts` -> `Custom Shortcuts`
- Xfce: `Settings` -> `Keyboard` -> `Application Shortcuts`

## macOS

macOS usually needs a small wrapper step because it does not expose a simple universal UI to bind an arbitrary binary path directly as a global shortcut.

Practical options:

1. `Shortcuts` app
   Create a shortcut that runs a shell script calling `~/.local/bin/armando`, then assign a keyboard shortcut in the Shortcuts settings.

2. `Automator`
   Create a Quick Action or Application that runs:

```bash
~/.local/bin/armando
```

Then bind the shortcut in `System Settings` -> `Keyboard` -> `Keyboard Shortcuts` -> `Services`.

## Windows

Windows gives you two simple options.

1. Desktop shortcut
   Create a shortcut to `%LOCALAPPDATA%\armando\bin\armando.exe`, open its properties, and set a `Shortcut key`.

2. PowerToys Keyboard Manager
   Use PowerToys to bind a keyboard combination that launches `%LOCALAPPDATA%\armando\bin\armando.exe`.

## Can One Keyboard Combo Work Identically Everywhere?

Not from documentation or installer setup alone.

Linux is easy because desktop environments natively support command shortcuts.
macOS and Windows can do the same job, but they need OS-specific registration steps.

If you want one built-in global hotkey feature managed directly by `armando`, it has to be implemented with native platform APIs inside the application.

## Next Steps

- For release download and install steps, see [`INSTALL.md`](INSTALL.md).
- For versioning and release artifacts, see [`RELEASES.md`](RELEASES.md).
- For the repository layout, see [`STRUCTURE.md`](STRUCTURE.md).
