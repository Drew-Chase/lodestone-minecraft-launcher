set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]
set shell := ["bash", "-c"]

@default:
    just --list