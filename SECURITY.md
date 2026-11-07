# Security policy

EarthPress is a local desktop application. It has no network client, no
telemetry, no updater and no remote content, so the attack surface is
deliberately small.

## Reporting a vulnerability

Please do **not** open a public issue for a security problem. Report it
privately through GitHub's **Security → Report a vulnerability** tab on this
repository.

Include the version, the steps to reproduce, and any files needed to trigger
the issue. You can expect an acknowledgement within a few days.

## Threat model

| Area | What EarthPress does |
|------|----------------------|
| Network | No HTTP client and no remote assets. Fonts are bundled; previews are `data:` URLs. |
| Webview | Strict CSP (`default-src 'self'`), no remote navigation, `withGlobalTauri: false`. |
| Tauri capabilities | Only `dialog:allow-open`, `opener:allow-reveal-item-in-dir`, `notification:default` and the window controls the title bar uses. No shell, fs or http plugins. |
| Rust commands | Paths must be absolute and supported, output folders must be directories, and inputs over 512 MB or 100 megapixels are rejected before decode. |
| Writes | Outputs go to a sibling temp file and are renamed into place, so a crash cannot leave a truncated file or destroy an original during overwrite. |
| Logging | `~/.earthpress/earthpress.log` records outcomes, not full source paths. |
| Installer | The NSIS installer is currently unsigned, so Windows SmartScreen may warn on first run. Code signing is tracked as a release task. |

## Out of scope

* A user deliberately converting a malicious file they already have. The app
  should fail safely, but it is not an antivirus.
* The SmartScreen warning caused by the unsigned installer.
* Issues that require an already-compromised Windows account or a modified
  installation.
