use super::*;

/// A fresh, empty folder under the system temp directory, named for the test that asked for it.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-server-pack-job-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the folder can be made");
    dir
}

/// Writes `contents` to `relative` under `root`, making the folders it needs, and returns the file's path.
fn put(root: &Path, relative: &str, contents: &[u8]) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a file has a parent")).expect("the folders can be made");
    std::fs::write(&path, contents).expect("the file can be written");
    path
}

#[test]
fn a_runtime_is_packed_under_java_without_its_install_marker() {
    let home = fresh("runtime");
    put(&home, "bin/java", b"program");
    put(&home, "lib/libjvm.so", b"library");
    put(&home, COMPLETE, b"");
    let files = runtime_files(&home).expect("the runtime is read");
    let mut paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    paths.sort_unstable();
    assert_eq!(paths, ["java/bin/java", "java/lib/libjvm.so"]);
    let program = files.iter().find(|file| file.path == "java/bin/java").expect("the program is packed");
    assert_eq!(program.size, 7);
}

#[cfg(unix)]
#[test]
fn a_link_in_the_runtime_is_packed_as_the_file_it_points_to() {
    let home = fresh("runtime-link");
    put(&home, "lib/libjvm.so", b"library");
    std::os::unix::fs::symlink("libjvm.so", home.join("lib/link.so")).expect("the link is made");
    let files = runtime_files(&home).expect("the runtime is read");
    let link = files.iter().find(|file| file.path == "java/lib/link.so").expect("the link is packed");
    assert_eq!(link.size, 7);
    assert_eq!(std::fs::read(&link.source).expect("the source reads"), b"library");
}

#[test]
fn the_packed_program_is_the_runtime_program_under_java() {
    let home = PathBuf::from("/meta/java/delta");
    let runtime = BundledJava {
        program: home.join("bin").join("java"),
        home: home.clone(),
    };
    let packed = packed_java(&runtime).expect("the program is under the runtime");
    assert_eq!(packed.program, "java/bin/java");
    assert_eq!(packed.os, host_os());
}
