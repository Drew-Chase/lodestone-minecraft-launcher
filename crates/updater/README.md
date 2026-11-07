# lodestone_updater

The launcher's update check. It asks GitHub for the newest release of `Drew-Chase/lodestone-minecraft-launcher`, downloads
the file that this system installs from, and checks the download against the release's `SHA256SUMS.txt` before anything
unpacks or runs it.

| System | File downloaded | How it is put in place |
|---|---|---|
| Windows (installed copy) | `Lodestone-<tag>-setup.exe` | Run silently and elevated with `/S`; the setup starts the launcher again |
| Linux | `Lodestone-<tag>-linux-x86_64.tar.gz` | The executable is unpacked and renamed over the running one; the launcher restarts |
| macOS | `Lodestone-<tag>-macos-universal.zip` | `Lodestone.app` is copied beside the running bundle and swapped in; the launcher restarts |

The crate has no UI. The launcher decides when each step runs: the check at launch and then periodically, the download in
the background, and the install when the user asks for it.

- `check` and `Answer`: the release check. It sends the tag of the last answer, so GitHub can answer `Unchanged` without a body.
- `newer`: the version comparison.
- `Payload`: the file each system takes, named as the release workflow names it.
- `fetch_asset` and `verify_file`: the download, verified before it is renamed into place.
- `installed_with_installer`: whether the running Windows launcher came from the installer.
- `run_silent_elevated`: starts the Windows installer with `/S` under the UAC prompt.
- `can_write_to`, `extract_executable`, `extract_bundle`, `replace_executable`, `replace_bundle`: the Unix placement. Each copy is made beside its target, so the final step is one rename.
- `relaunch_after_exit`: starts the launcher again once the process that asked has ended.
