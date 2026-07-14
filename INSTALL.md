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

1. Double-click **`install.bat`** in this folder.
2. A black window will appear and show progress. **Leave it open.**
3. If Windows asks _"Do you want to allow this app to make changes?"_ while installing Microsoft C++ Build Tools, click **Yes**.
4. When the app window appears, setup is complete! 🎉

> **Tip:** If you see a blue "Windows protected your PC" screen, click **"More info"**  
> then **"Run anyway"**. This happens because the script isn't signed.

### Subsequent launches

Just double-click **`install.bat`** again — it detects what's already installed and  
goes straight to launching the app (takes a few seconds).

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

## ❓ Troubleshooting

| Problem | Solution |
|---------|----------|
| "incomplete Agentic Council folder" | Re-download and fully extract the project. The installer checks required app files before downloading any tools. |
| A Windows permission prompt appears | Click **Yes**. It is the official Microsoft C++ Build Tools installer. |
| macOS shows the Command Line Tools dialog | Click **Install** and leave Terminal open. |
| The window closed before the app appeared | Re-run the installer — it will resume where it left off. |
| "npm install failed" or "cargo build failed" | Check your internet connection and re-run the installer. |

If the problem persists, please share the text from the installer window with the developer.

---

## 🔒 Is this safe?

- The installer downloads only from official sources:
  - **Node.js**: from `nodejs.org` (kept in a private Agentic Council tools folder)
  - **Rust**: from `rust-lang.org` (the official Rust website)
  - **Windows build tools and WebView2**: from Microsoft
  - **npm packages**: from `npmjs.com`
- No third-party or unverified software is installed.
- The app itself is **fully offline** — your data never leaves your computer.
