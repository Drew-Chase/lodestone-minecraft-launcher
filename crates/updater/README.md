# lodestone_updater

The launcher's update check. It asks GitHub for the newest release of `Drew-Chase/lodestone-minecraft-launcher`,
downloads the Windows installer, checks the download against the release's `SHA256SUMS.txt`, and runs the installer
silently.

The crate has no UI. The launcher decides when each step runs: the check at launch, the download when an update is
found, and the install once no game is running. Only the Windows installer path is Windows-specific, and it sits behind
`cfg(windows)`.

- `latest` and `newer`: the release check, comparing semantic versions.
- `fetch_installer` and `download_verified`: the download, verified before it is renamed into place.
- `installed_with_installer`: whether the running launcher came from the Windows installer.
- `run_silent_elevated`: starts the installer with `/S` under the UAC prompt.
