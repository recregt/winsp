# Microsoft Store submission content

Reference text for the Partner Center submission form. Copy directly into the corresponding fields; nothing here needs to be filled out through this repo.

## Listing description

WinSP is a lightning-fast, Spotlight-style app launcher for Windows. Press Alt+Space from anywhere to bring up a floating search bar, start typing, and instantly find installed applications and Windows Settings pages — or type a math expression to get an answer on the spot.

WinSP runs quietly from the system tray. It can optionally start with Windows, and it stays out of your way until you summon it.

Everything happens locally on your device: WinSP has no network access, collects no data, and requires no account. See the [privacy policy](https://recregt.github.io/winsp/privacy-policy.html) for details.

## Search terms

- launcher
- spotlight
- search
- productivity
- quick launch

## Category

Productivity

## runFullTrust capability justification

WinSP is a Win32 desktop application distributed via Desktop Bridge and requires the runFullTrust capability to use three standard Win32 APIs that are unavailable to sandboxed apps:

- **RegisterHotKey** — registers a system-wide Alt+Space hotkey so the search bar can be summoned from any application.
- **Shell_NotifyIconW** — shows a system tray icon, letting WinSP run in the background with quick access to toggle the search bar, enable/disable autostart, and exit.
- **HKCU Run registry key** — writes a single optional autostart entry (only when the user enables "Start with Windows" from the tray menu) under `Software\Microsoft\Windows\CurrentVersion\Run`.

WinSP does not request administrator privileges, does not access other applications' data, and makes no network connections.

## Privacy policy URL

https://recregt.github.io/winsp/privacy-policy.html

## Age rating questionnaire guidance

WinSP contains no violence, fear content, sexual content, profanity, references to controlled substances, gambling, or user-generated content. It does not share the user's location, does not process digital purchases, does not require unrestricted internet access, and does not collect personal data (see privacy policy). Every content-related question on the questionnaire should be answered "No."
