set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]
set shell := ["bash", "-c"]

@default:
    just --list

# Runs the launcher with the performance overlay (F3), the per-second report and the Chrome trace.
perf:
    cargo run -p lodestone_launcher --features perf

# Runs the performance instrumentation on a release-speed build with symbols, for sampling profilers.
profile:
    cargo run --profile profiling -p lodestone_launcher --features perf
