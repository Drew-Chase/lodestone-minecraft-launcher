use chrono::Timelike;
use std::fmt::Write as _;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    for entry in WalkDir::new("src") {
        let entry = entry.unwrap();
        if entry.file_type().is_file() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }
    generate_build_number();
    generate_icon();
    generate_languages();
}

fn generate_build_number() {
    let seconds_past_in_day = chrono::Local::now().num_seconds_from_midnight();
    let build = chrono::Local::now().format("%Y%m%d").to_string();
    let build = format!("{}.{:06}", build, seconds_past_in_day);
    println!("cargo:rustc-env=BUILD={build}");
}

fn generate_icon() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../res/icon/icon.ico");
        res.set("ProductName", "Lodestone Minecraft Launcher");
        res.compile().expect("Failed to compile resource");
    }
}

/// Embeds every catalogue under `res/lang`. Writes `languages.rs` into `OUT_DIR`, which
/// `app/lang/mod.rs` includes: one `(code, source)` entry per file, with `en_us` first so
/// it is the fallback.
fn generate_languages() {
    let dir = Path::new("../../res/lang");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut codes: Vec<String> = std::fs::read_dir(dir)
        .expect("reading res/lang")
        .map(|entry| entry.expect("reading res/lang").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "toml"))
        .filter_map(|path| {
            println!("cargo:rerun-if-changed={}", path.display());
            path.file_stem()?.to_str().map(str::to_owned)
        })
        .collect();
    assert!(codes.iter().any(|code| code == "en_us"), "res/lang must contain en_us.toml");
    codes.sort_by(|a, b| (a.as_str() != "en_us").cmp(&(b.as_str() != "en_us")).then_with(|| a.cmp(b)));

    let mut out = String::from("pub(crate) const LANGUAGE_FILES: &[(&str, &str)] = &[\n");
    for code in &codes {
        writeln!(
            out,
            r#"    ("{code}", include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../res/lang/{code}.toml"))),"#
        )
        .expect("formatting into a String");
    }
    out.push_str("];\n");
    let target = Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("languages.rs");
    std::fs::write(target, out).expect("writing languages.rs");
}
