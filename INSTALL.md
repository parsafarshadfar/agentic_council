# How to Run Agentic Council from Source

> **No technical experience required.** Follow the steps for your computer below.
> The installer will automatically download and set up everything it needs.

---

## ⏱ What to expect

| Phase | Time |
|-------|------|
| Downloading tools (Node, Rust, etc.) | 5–15 min |
| First-time Rust compilation | 10–20 min |
| Later launches | Usually much faster; changed code may be recompiled |

Your computer needs an **internet connection during initial setup**. Remote AI providers also require internet access when you use them; the Local Demo provider can run offline after setup.

These scripts prepare the source tree and start Tauri in development mode. They do not install a packaged desktop app or create a Start menu or Applications shortcut. Keep the installer window open while Agentic Council is running.

---

## 🪟 Windows

### One-time setup

1. Double-click **`install.bat`** in this folder.
2. A black window will appear and show progress. **Leave it open.**
3. If Windows asks _"Do you want to allow this app to make changes?"_ while installing Microsoft C++ Build Tools, click **Yes**.
4. When the app window appears, setup is complete! 🎉

> **Tip:** If you see a blue "Windows protected your PC" screen, click **"More info"**  
> then **"Run anyway"**. This happens because the script isn't signed.

### Subsequent launches

Just double-click **`install.bat`** again. It rechecks the prerequisites and dependencies, then launches the app. This is usually much faster than the first run, although changed code may be recompiled.

`install.bat` is the recommended Windows entry point. It changes to the project folder and invokes **`install-windows.ps1`** with a process-scoped execution-policy bypass, so you do not need to change your system PowerShell policy.

---

## 🍎 macOS

### One-time setup

1. Open **Terminal** (press `⌘ Space`, type `Terminal`, press Enter).
2. Type `bash ` (including the space), drag **`install.sh`** from Finder into the Terminal window, and press **Enter**.

   The result will look similar to:

   ```
   bash "/path/to/agentic_council/install.sh"
   ```

3. If a dialog pops up saying **"Install Command Line Developer Tools?"** — click **Install**. The script waits for it to finish.
4. Leave Terminal open until the Agentic Council window appears. 🎉

### Subsequent launches

Open Terminal and run the same command — it skips everything already installed.

---

## 🐧 Linux

From a terminal in the project folder, run:

```bash
bash install.sh
```

The script supports Debian/Ubuntu-style systems using `apt-get` and Fedora-style systems using `dnf`. It may request your password through `sudo` to install the required desktop build libraries. Other Linux distributions require manual installation of the [Tauri v2 prerequisites](https://tauri.app/start/prerequisites/).

---

## ❓ Troubleshooting

| Problem | Solution |
|---------|----------|
| "incomplete Agentic Council folder" | Re-download and fully extract the project. The installer checks required app files before downloading any tools. |
| A Windows permission prompt appears | Click **Yes**. It is the official Microsoft C++ Build Tools installer. |
| macOS shows the Command Line Tools dialog | Click **Install** and leave Terminal open. |
| Linux reports that the distribution is unsupported | Install the Tauri v2 system prerequisites manually, then re-run the script. |
| The window closed before the app appeared | Re-run the installer — it will resume where it left off. |
| "npm install failed" or "cargo build failed" | Check your internet connection and re-run the installer. |

If the problem persists, please share the text from the installer window with the developer.

---

## 🔒 What the installer downloads

- **Node.js** is downloaded from `nodejs.org`, checksum-verified, and kept in a private Agentic Council tools folder when a compatible system version is unavailable.
- **Rust** and its components are installed through the official Rust toolchain service.
- **Windows C++ Build Tools and WebView2** are downloaded from Microsoft and their installer signatures are checked.
- Project dependencies are installed from the npm and Cargo registries according to the repository's lockfiles.
- The Local Demo provider works offline after setup. When you configure a remote AI provider, prompts and related request content are sent only to that provider's API.
