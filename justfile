set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]
set shell := ["bash", "-c"]
# The Windows release recipes run script files, which Windows PowerShell's default execution policy can block.
export PSExecutionPolicyPreference := "Bypass"

# The release version, read from the launcher's Cargo.toml the same way the release workflow reads it.
version := replace_regex(`cargo pkgid -p lodestone_launcher`, '^.*[#@]', '')
base := replace_regex(version, '-.*$', '')
tag := "v" + version
prerelease := if version == base { "" } else { "--prerelease" }
artifacts := if os() == "windows" { "Lodestone-" + tag + "-windows-x64.zip Lodestone-" + tag + "-setup.exe" } else if os() == "macos" { "Lodestone-" + tag + "-macos-universal.zip Lodestone-" + tag + "-macos-universal.dmg" } else { "Lodestone-" + tag + "-linux-x86_64.tar.gz" }

@default:
    just --list

build:
    cargo build --release

# Builds Lodestone.app and a DMG for this Mac's architecture, into dist/. Run on macOS.
dmg:
    cargo build --profile dist -p lodestone_launcher
    version="$(cargo pkgid -p lodestone_launcher | sed 's/.*[#@]//')" && cargo xtask bundle-macos --binary target/dist/lodestone_launcher --version "$version" --out dist --dmg "dist/Lodestone-v${version}-macos.dmg"

# This will app update the version and publish a tag to github to initiate CICD
publish version="":
    uv -g commit-push-tag --path ./crates/gui/ {{version}}

dev:
    cargo run --bin lodestone_launcher

# Runs the launcher with the performance overlay (F3), the per-second report and the Chrome trace.
perf:
    cargo run -p lodestone_launcher --features perf

# Runs the performance instrumentation on a release-speed build with symbols, for sampling profilers.
profile:
    cargo run --profile profiling -p lodestone_launcher --features perf

# Installs every Minecraft version's latest Forge build with real installers: about 63 installs, many gigabytes, an hour or
# more. Narrow it with LODESTONE_FORGE_VERSIONS=1.20.1,26.3 and set LODESTONE_FORGE_JOBS (default 4). The table is written to
# target/tmp/forge-install/report.md.
forge-matrix:
    cargo test -p lodestone_loaders --test forge_install -- --ignored --nocapture

# Builds this machine's release files into dist/, named and zipped as .github/workflows/release.yml names them.
package:
    @just _package

# Each machine uploads its own files. SHA256SUMS.txt keeps the lines the other machines already uploaded.

# Builds this machine's release files and uploads them to this version's GitHub release, creating it if needed.
dist: _gh package
    @just _publish "{{artifacts}}"

# Stops before a long build when the GitHub CLI is not signed in.
_gh:
    gh auth status

[macos]
_package:
    #!/usr/bin/env bash
    set -euo pipefail
    key="${LODESTONE_CURSEFORGE_KEY:-}"
    [ -n "${key//[[:space:]]/}" ] || { echo "Set LODESTONE_CURSEFORGE_KEY to the CurseForge API key first. CI reads it from the CURSEFORGE_API_KEY secret." >&2; exit 1; }
    # The same two targets and deployment target as the release workflow, joined into one universal binary.
    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    export MACOSX_DEPLOYMENT_TARGET=11.0
    cargo build --profile dist -p lodestone_launcher --target aarch64-apple-darwin
    cargo build --profile dist -p lodestone_launcher --target x86_64-apple-darwin
    mkdir -p target/universal dist
    lipo -create -output target/universal/lodestone_launcher \
        target/aarch64-apple-darwin/dist/lodestone_launcher \
        target/x86_64-apple-darwin/dist/lodestone_launcher
    cargo xtask bundle-macos --binary target/universal/lodestone_launcher --version "{{base}}" --out dist --dmg "dist/Lodestone-{{tag}}-macos-universal.dmg"
    ditto -c -k --sequesterRsrc --keepParent dist/Lodestone.app "dist/Lodestone-{{tag}}-macos-universal.zip"

[linux]
_package:
    #!/usr/bin/env bash
    set -euo pipefail
    key="${LODESTONE_CURSEFORGE_KEY:-}"
    [ -n "${key//[[:space:]]/}" ] || { echo "Set LODESTONE_CURSEFORGE_KEY to the CurseForge API key first. CI reads it from the CURSEFORGE_API_KEY secret." >&2; exit 1; }
    [ "$(uname -m)" = x86_64 ] || { echo "The Linux release is x86_64 only, and this machine is $(uname -m)." >&2; exit 1; }
    # CI builds on Ubuntu 22.04 so the binary runs on older glibc. A newer distro can produce a binary that does not.
    cargo build --profile dist -p lodestone_launcher
    mkdir -p dist
    tar -czf "dist/Lodestone-{{tag}}-linux-x86_64.tar.gz" -C target/dist lodestone_launcher

[windows]
_package:
    #!powershell.exe -NoProfile
    if ([string]::IsNullOrWhiteSpace($env:LODESTONE_CURSEFORGE_KEY)) { [Console]::Error.WriteLine('Set LODESTONE_CURSEFORGE_KEY to the CurseForge API key first. CI reads it from the CURSEFORGE_API_KEY secret.'); exit 1 }
    cargo build --profile dist -p lodestone_launcher
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    New-Item -ItemType Directory -Force dist | Out-Null
    Compress-Archive -Path target/dist/lodestone_launcher.exe -DestinationPath dist/Lodestone-{{tag}}-windows-x64.zip -Force -ErrorAction Stop
    $nsis = (Get-Command makensis -ErrorAction SilentlyContinue).Source
    if (-not $nsis) { $nsis = @("${env:ProgramFiles(x86)}\NSIS\makensis.exe", "${env:ProgramFiles(x86)}\NSIS\Bin\makensis.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1 }
    if (-not $nsis) { [Console]::Error.WriteLine('makensis was not found. Install NSIS (winget install NSIS.NSIS) and run this again.'); exit 1 }
    & $nsis "/DAPP_VERSION={{base}}" "/DBIN_DIR=$((Resolve-Path target/dist).Path)" "/DOUT_FILE=$((Resolve-Path dist).Path)\Lodestone-{{tag}}-setup.exe" res\installer.nsi
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

[unix]
_publish names:
    #!/usr/bin/env bash
    set -euo pipefail
    tag="{{tag}}"
    names=({{names}} installer.sh)
    mkdir -p dist
    # CI ships the Linux installer script too. Strip carriage returns so sh can run it.
    tr -d '\r' < res/installer.sh > dist/installer.sh
    if command -v sha256sum >/dev/null 2>&1; then hasher=(sha256sum); else hasher=(shasum -a 256); fi
    (cd dist && "${hasher[@]}" "${names[@]}") > dist/SHA256SUMS.new
    exists=0
    if gh release view "$tag" >/dev/null 2>&1; then exists=1; fi
    cp dist/SHA256SUMS.new dist/SHA256SUMS.txt
    if [ "$exists" = 1 ]; then
        assets=$(gh release view "$tag" --json assets --jq '.assets[].name')
        # Keep the lines for files this run does not upload, so one machine's upload does not drop another's.
        if grep -qx SHA256SUMS.txt <<< "$assets"; then
            mkdir -p dist/remote
            gh release download "$tag" --pattern SHA256SUMS.txt --dir dist/remote
            awk 'NR == FNR { mine[$2] = 1; print; next } !($2 in mine)' dist/SHA256SUMS.new dist/remote/SHA256SUMS.txt > dist/SHA256SUMS.txt
            rm -rf dist/remote
        fi
    fi
    rm -f dist/SHA256SUMS.new
    files=()
    for name in "${names[@]}"; do files+=("dist/$name"); done
    files+=(dist/SHA256SUMS.txt)
    if [ "$exists" = 1 ]; then
        gh release upload "$tag" "${files[@]}" --clobber
    else
        gh release create "$tag" "${files[@]}" --verify-tag --title "$tag" --notes "" {{prerelease}}
    fi

[windows]
_publish names:
    #!powershell.exe -NoProfile
    $tag = '{{tag}}'
    $names = @('{{names}}'.Split(' ')) + 'installer.sh'
    $dist = Join-Path (Get-Location).Path 'dist'
    New-Item -ItemType Directory -Force $dist | Out-Null
    # CI ships the Linux installer script too. Write it with LF endings, as sh needs.
    [IO.File]::WriteAllText((Join-Path $dist 'installer.sh'), ((Get-Content res/installer.sh -Raw) -replace "`r`n", "`n"))
    $sums = @(foreach ($name in $names) { '{0}  {1}' -f (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $dist $name) -ErrorAction Stop).Hash.ToLower(), $name })
    gh release view $tag *> $null
    $exists = $LASTEXITCODE -eq 0
    if ($exists -and ((@(gh release view $tag --json assets --jq '.assets[].name')) -contains 'SHA256SUMS.txt')) {
        New-Item -ItemType Directory -Force (Join-Path $dist 'remote') | Out-Null
        gh release download $tag --pattern SHA256SUMS.txt --dir (Join-Path $dist 'remote')
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        # Keep the lines for files this run does not upload, so one machine's upload does not drop another's.
        $kept = @(Get-Content (Join-Path $dist 'remote/SHA256SUMS.txt') | Where-Object { $_ -and ($names -notcontains @($_ -split '\s+')[1]) })
        $sums = @($sums) + $kept
        Remove-Item (Join-Path $dist 'remote') -Recurse -Force
    }
    [IO.File]::WriteAllText((Join-Path $dist 'SHA256SUMS.txt'), (($sums -join "`n") + "`n"))
    $files = @($names | ForEach-Object { "dist/$_" }) + 'dist/SHA256SUMS.txt'
    if ($exists) {
        gh release upload $tag $files --clobber
    } else {
        gh release create $tag $files --verify-tag --title $tag --notes= {{prerelease}}
    }
    exit $LASTEXITCODE
