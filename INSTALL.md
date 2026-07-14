# How to Run Agentic Council

> **No technical experience required.** Follow the steps for your computer below.
> The installer will automatically download and set up everything it needs.

---

## ⏱ What to expect

| Phase | Time |
|-------|------|
| Downloading tools (Node, Rust, etc.) | 5–15 min |
| First-time Rust compilation | 10–20 min |
| Every launch after the first | < 5 seconds |

Your computer needs an **internet connection** for the first setup only.

---

## 🪟 Windows

### One-time setup

1. Right-click **`install.bat`** in this folder.
2. Choose **"Run as administrator"**.
3. If Windows asks _"Do you want to allow this app to make changes?"_ — click **Yes**.
4. A black window will appear and show progress. **Leave it open.**
5. When the app window appears, setup is complete! 🎉

> **Tip:** If you see a blue "Windows protected your PC" screen, click **"More info"**  
> then **"Run anyway"**. This happens because the script isn't signed.

### Subsequent launches

Just double-click **`install.bat`** again — it detects what's already installed and  
goes straight to launching the app (takes a few seconds).

---

## 🍎 macOS

### One-time setup

1. Open **Terminal** (press `⌘ Space`, type `Terminal`, press Enter).
2. Copy and paste this command, then press **Enter**:

   ```
   bash "/path/to/agentic_council/install.sh"
   ```

   Replace `/path/to/agentic_council` with the actual folder path. The easiest way:  
   type `bash ` (with a space), then **drag the `install.sh` file** from Finder into  
   the Terminal window, then press **Enter**.

3. If macOS asks for your password, type it (nothing will appear while you type — that's normal) and press Enter.
4. If a dialog pops up saying **"Install Command Line Developer Tools?"** — click **Install**.
5. Leave Terminal open until the Agentic Council window appears. 🎉

### Subsequent launches

Open Terminal and run the same command — it skips everything already installed.

---

## ❓ Troubleshooting

| Problem | Solution |
|---------|----------|
| "winget is not recognized" (Windows) | Update Windows. Open Settings → Windows Update → Check for updates. |
| "Permission denied" (macOS) | Run: `chmod +x install.sh` in Terminal, then try again. |
| The window closed before the app appeared | Re-run the installer — it will resume where it left off. |
| "npm install failed" or "cargo build failed" | Check your internet connection and re-run the installer. |

If the problem persists, please share the text from the installer window with the developer.

---

## 🔒 Is this safe?

- The installer downloads only from official sources:
  - **Node.js**: from `winget` (Microsoft's official package manager) or `Homebrew`
  - **Rust**: from `rust-lang.org` (the official Rust website)
  - **npm packages**: from `npmjs.com`
- No third-party or unverified software is installed.
- The app itself is **fully offline** — your data never leaves your computer.
