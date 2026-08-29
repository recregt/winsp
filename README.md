[![CI](https://github.com/recregt/winsp/actions/workflows/ci.yml/badge.svg)](https://github.com/recregt/winsp/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/recregt/winsp)](https://github.com/recregt/winsp/releases/latest)
[![License: MIT](https://img.shields.io/github/license/recregt/winsp)](LICENSE)

## What is WinSP?

`WinSP` is a blazing-fast application launcher heavily inspired by [MacOS Spotlight](https://en.wikipedia.org/wiki/Spotlight_(Apple)).

## What does WinSP do?

| Feature | Description | Trigger / Detail |
| --- | --- | --- |
| **Global Hotkey** | Toggles a floating, borderless search bar | `Alt+Space` *(currently fixed)* |
| **Fuzzy App Search** | Multi-tier ranking algorithm with frequency boost | `Exact` → `Prefix` → `Acronym` (`vsc` → Visual Studio Code) → `Substring` → `Fuzzy` → `Keyword` |
| **Instant Calculator** | Evaluates math expressions instantly & locally | Type any expression (e.g. `128 * 4`) |
| **App Discovery** | Recursively scans Start Menu shortcuts (`.lnk`/`.url`) and built-in Windows tools | Ignores uninstallers/help links; includes Notepad, Terminal, PowerShell, Registry Editor, etc. |
| **Settings Shortcuts** | Deep links to Windows Settings via `ms-settings:` URIs | Display, Sound, Bluetooth, Network, Installed Apps, Windows Update, Power, etc. |
| **Universal Launching** | Executes Win32 paths, UWP/Store apps (AUMID), URIs, and raw commands | Handled natively via `ShellExecuteW` |
| **Keyboard-Driven UI** | Fast, mouse-free navigation | `Arrows` / `Tab` to navigate, `Enter` to launch, live typing filter |

## How fast is that?

Well... I was also wondering that and created a benchmark for it. Here are the results:

| Indexed Items | Empty Query | Prefix Match | Acronym Match | No-Match (Worst-Case)* |
| --- | --- | --- | --- | --- |
| **1,000** | 79 µs | 146 µs | 229 µs | 162 µs |
| **10,000** | 1.12 ms | 1.61 ms | 2.44 ms | 1.66 ms |
| **50,000** | 6.69 ms | 9.19 ms | 11.80 ms | 8.45 ms |

*Forces every matching strategy (exact → prefix → acronym → substring → fuzzy) to evaluate and fail across all items.*

### Key Takeaways

* **Sub-millisecond in Practice:** Typical Start Menu sizes (~500–1,000 items) respond in **under 250 µs**.
* **Predictable $O(n)$ Scaling:** Performance scales linearly even under heavy load.
* **Worst-Case Ceiling:** Under an extreme stress test of 50,000 items, search latency stays firmly below **12 ms**.

### In Plain English
Basically, It's ridiculously fast. It finds what you want before `Windows Search` even decides whether to show you Bing web search results.

## Getting Started

### Installation

Download the latest `winsp.msix` from the [Releases](https://github.com/recregt/winsp/releases) page and install it (`Add-AppxPackage`), or build it from source.

### Build from Source

Ensure you have the Rust toolchain installed:

```bash
# Clone the repository
git clone https://github.com/recregt/winsp.git
cd winsp

# Build the release binary
cargo build -p winsp-app --release --locked

```

The compiled binary will be located at `target/release/winsp.exe`.

## Contributing

Want to contribute? Check out our [Contributing Guide](CONTRIBUTING.md).

## License

This project is licensed under the [MIT License](LICENSE).