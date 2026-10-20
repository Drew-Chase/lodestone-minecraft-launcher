use chrono::Timelike;
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
