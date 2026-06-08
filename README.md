<div align="center">

<img src="docs/assets/banner.png" width="820" alt="Pebble — Your Personal AI Assistant" />

<p>
  <img alt="version" src="https://img.shields.io/badge/version-0.10.5-e7a6bd?style=for-the-badge" />
  <img alt="platform" src="https://img.shields.io/badge/Windows-10%20%2F%2011-9aa4d4?style=for-the-badge&logo=windows&logoColor=white" />
  <img alt="local" src="https://img.shields.io/badge/100%25-local-7cc6a6?style=for-the-badge" />
  <img alt="license" src="https://img.shields.io/badge/license-MIT-cbb9c9?style=for-the-badge" />
</p>

### A little rock that helps tidy your PC, runs on your own machine, and can't break anything. 🪨

<p>
  <a href="https://github.com/dasler08/Pebble---Your-Personal-Ai-Assistant/releases/latest"><b>⬇&nbsp; Download for Windows</b></a>
  &nbsp;•&nbsp;
  <a href="https://dasler08.github.io/Pebble---Your-Personal-Ai-Assistant/"><b>🌐&nbsp; Website</b></a>
  &nbsp;•&nbsp;
  <a href="https://discord.gg/ATXTFSmX6N"><b>💬&nbsp; Discord</b></a>
</p>

</div>

Tell Pebble what you'd like cleaned up, in plain English, and he figures it out. The catch is that he never actually does anything on his own. He suggests the change, shows you exactly what it would do, and waits for your go-ahead. Nothing is ever deleted for good, and the whole thing runs on your own computer through [Ollama](https://ollama.com). No cloud, no sign-up, nothing leaving your machine.

<div align="center"><img src="docs/assets/card-chat.png" width="720" alt="Chatting with Pebble" /></div>

## Why I made him

Most AI assistants land in one of two camps. Some live in the cloud, which means your files end up sitting on someone else's computer. Others will gladly run whatever command they come up with and hope nothing goes sideways. Neither one sat right with me.

Pebble takes the opposite road on both fronts. He runs on your machine, so your files and your conversations stay with you. And he genuinely can't go rogue, because the AI is only ever allowed to *suggest* things. A completely separate piece of code, independent from the model, looks at every suggestion, refuses to touch system folders, and does nothing at all until you approve it. Anything he removes goes to a Trash you can restore from, and every action can be undone.

<div align="center"><img src="docs/assets/card-features.png" width="840" alt="Safety-first · 100% local · Finds anything · Trash + Undo" /></div>

## What it's actually like to use

**Clearing out a folder.** Say "clean my downloads" and Pebble lays out a plan to move everything into the recoverable Trash. One click does it, and if you change your mind, one click brings it all back.

<div align="center"><img src="docs/assets/card-clean.png" width="700" alt="One-click clean: clear Downloads to the recoverable Trash" /></div>

**Sorting out a mess.** "organize my desktop by type" turns into a preview card that spells out what moves where. You see the entire plan before a single file shifts.

<div align="center"><img src="docs/assets/card-plan.png" width="640" alt="A proposed-action card with counts, warnings, and Approve / Reject" /></div>

**Tracking things down.** Ask him to find your biggest files, hunt for duplicates, work out what's eating your storage, or even point you to wherever Steam tucked your games away. He'll dig through your folders, Program Files, and drives until he finds them.

**Making him yours.** Pick one of five themes, tell him your name, and set the personality that suits you.

<div align="center">
  <img src="docs/assets/themes.png" width="820" alt="Pebble themes: Pebble, Matcha, Cloud, Stars, Liquid Glass" />
  <br /><sub>Pebble · Matcha · Cloud · Stars · Liquid Glass</sub>
</div>

## How he stays safe

This is the part I care about most. The model never touches your files, full stop. All it can do is hand over a list of *proposed* actions. From there a separate validator takes over, and it's the only code in the whole app that's allowed to greenlight anything. That rule is enforced by the type system itself, so an unapproved action simply cannot reach your disk, even if something else went wrong.

<div align="center"><img src="docs/assets/card-safety.png" width="900" alt="You ask → Pebble proposes → Safety Validator → You approve → Done" /></div>

Here's the path every request walks:

1. **You ask** for something in your own words, like "tidy my desktop", "find big files", or "clean downloads".
2. **Pebble proposes** the exact steps. He won't be vague, and he won't claim he's finished something he hasn't actually done.
3. **The validator checks** each step against protected folders, the area you've allowed, and how risky the action is.
4. **You decide** from a clear preview. The riskier the action, the more deliberate the confirmation.
5. **It happens**, with the Trash and one-click Undo always sitting there to catch you.

The actions come in three levels, and the riskier something is, the more it takes to go ahead. The safe stuff, like finding files, searching, or checking what's eating your storage, just runs after a quick path check. Everyday changes such as moving, renaming, and organizing need a single click to approve. The serious things, like deleting, emptying the Recycle Bin, or running a program, ask for a deliberate confirmation that's sometimes typed out before anything happens.

A few places are simply off-limits no matter what: `C:\Windows`, `Program Files`, `ProgramData`, `AppData`, and your drive roots. By default he only works inside your home folder, and you can widen that in Settings whenever you want.

## What's new in 0.10.5

**He can match your tone.** Pebble mirrors your energy and length now — playful when you're playful, gentle when you're down, brief when you're brief. It's on by default, and there's a switch in Settings → You & Pebble if you'd rather he didn't.

**Show him an image.** Drag any image onto the window (the whole app turns into a drop zone) or use the new image button in the chat box, and Pebble reads the text out of it. Flip OCR on in Settings → Extensions first.

**Replies type themselves in.** Streamed text fades in gently as he writes, and the little mode hints under the chat box now drift in softly instead of sitting there like a warning.

**A warmer home screen.** He greets you by the time of day, and the chat room got a light visual polish.

Before this came the 0.10.1 friend update: three chat modes (Chat / Do / Plan), a warmer voice, an opt-in memory he curates, a "let Pebble ask you something" button, and date plus weather. And 0.10.0 brought live streaming with a Stop button, a friendlier first run, the Content Search / OCR / Duplicate Cleaner extensions, and the safety hardening.

## Getting started

1. **Install [Ollama](https://ollama.com)** and pull a model for it to run:
   ```bash
   ollama pull llama3.2
   ```
2. **Grab Pebble** from the [latest release](https://github.com/dasler08/Pebble---Your-Personal-Ai-Assistant/releases/latest) and run **`Pebble_0.10.5_x64-setup.exe`**. Would rather skip the installer? There's a portable `Pebble.exe` too.
3. **Say hi.** He'll introduce himself, ask your name, take a look at your PC, and suggest a model that'll run nicely on it.

Everything lives locally in `C:\AI_Assistant\`, which is your database and the recoverable Trash.

## Giving him a bigger brain

The default `llama3.2:3b` is small and quick. It's lovely for chatting but starts to struggle with longer, multi-step tidying jobs. Pebble looks at your hardware when he starts up and tells you if something better would fit, and you can pull and switch models right from **Settings → AI Model**.

| Model | Best at | Roughly needs |
|-------|---------|---------------|
| `llama3.2:3b` | quick chats and simple lookups (the default) | ~2 GB |
| `llama3.1:8b` | organizing and following multi-step asks | ~6 GB |
| `qwen2.5:7b` / `:14b` | strong all-rounder / precise tidying | ~6 / ~12 GB |
| `deepseek-r1:8b` | careful, step-by-step reasoning | ~6 GB |

## If Windows says "protected your PC"

That warning is a false alarm. Pebble is open source and unsigned, simply because code-signing certificates cost money I haven't spent. Click **More info → Run anyway**, or add an exclusion in Windows Security, or build it yourself from the source below. Whichever you pick, the only thing he ever connects to is your local Ollama.

## Built with

Tauri 2, Rust, React, TypeScript, SQLite, and Ollama. A small, fast stack that runs entirely on your machine.

```bash
npm install
npm run tauri dev      # run it in development
npm run tauri build    # build the installer (src-tauri/target/release/bundle/nsis/)
```

## Questions people ask

<details>
<summary><b>Is it really free and private?</b></summary>

Yes. It's free, open source, and runs entirely on your own machine through Ollama. There's no cloud, no account, and nothing leaves your PC.
</details>

<details>
<summary><b>Could it delete my files by accident?</b></summary>

That's exactly what the whole design fights against. Pebble can only suggest, the validator blocks system folders and waits for your approval, deletes go to a recoverable Trash, and anything can be undone.
</details>

<details>
<summary><b>Which file do I download?</b></summary>

The <b>setup .exe</b>. It bundles everything it needs, including WebView2. If you'd rather not install anything, grab the portable <code>Pebble.exe</code> instead.
</details>

<details>
<summary><b>My PC is decent, can he be smarter?</b></summary>

Definitely. He'll recommend a stronger model based on your hardware, and you can pull and switch to it in Settings → AI Model.
</details>

## ⭐ Star history

<a href="https://star-history.com/#dasler08/Pebble---Your-Personal-Ai-Assistant&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=dasler08/Pebble---Your-Personal-Ai-Assistant&type=Date&theme=dark" />
    <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=dasler08/Pebble---Your-Personal-Ai-Assistant&type=Date" />
  </picture>
</a>

## License

[MIT](LICENSE). Do whatever you like with it, just be kind.

<div align="center"><sub>Made with 🤍, and be nice to Pebble.</sub></div>
