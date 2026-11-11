//! The matrix's table: one row per Minecraft version, written to a Markdown file and printed.

use std::fmt::Write as _;
use std::path::Path;

/// How a version came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Every step passed, and the game folder was removed.
    Pass,
    /// A step failed. The note names it and the log that holds the installer's output.
    Fail,
    /// The version publishes no installer, and its download fails with the expected 404.
    KnownGap,
    /// A known gap downloads now, so its entry has to come out of the list.
    NowInstalls,
}

impl Outcome {
    /// Whether this outcome fails the matrix.
    pub(crate) fn is_problem(self) -> bool {
        matches!(self, Outcome::Fail | Outcome::NowInstalls)
    }

    fn label(self) -> &'static str {
        match self {
            Outcome::Pass => "PASS",
            Outcome::Fail => "FAIL",
            Outcome::KnownGap => "known gap",
            Outcome::NowInstalls => "NOW INSTALLS",
        }
    }
}

/// One version's row: what each step did, how long the install took, and how the version came out.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub(crate) minecraft: String,
    pub(crate) forge: String,
    pub(crate) layout: String,
    pub(crate) java: String,
    pub(crate) vanilla: String,
    pub(crate) download: String,
    pub(crate) install: String,
    pub(crate) verify: String,
    pub(crate) reinstall: String,
    pub(crate) seconds: u64,
    pub(crate) outcome: Outcome,
    pub(crate) note: String,
}

impl Row {
    /// A row with every step not yet run.
    pub(crate) fn new(minecraft: &str, forge: &str) -> Self {
        Self {
            minecraft: minecraft.to_string(),
            forge: forge.to_string(),
            layout: "-".to_string(),
            java: "-".to_string(),
            vanilla: "-".to_string(),
            download: "-".to_string(),
            install: "-".to_string(),
            verify: "-".to_string(),
            reinstall: "-".to_string(),
            seconds: 0,
            outcome: Outcome::Fail,
            note: String::new(),
        }
    }
}

/// The table as Markdown, with a line of totals above it.
pub(crate) fn render(rows: &[Row]) -> String {
    let count = |outcome: Outcome| rows.iter().filter(|row| row.outcome == outcome).count();
    let mut out = String::from("# Forge install matrix\n\n");
    writeln!(
        out,
        "{} installed, {} known gaps, {} problems, across {} Minecraft versions.\n",
        count(Outcome::Pass),
        count(Outcome::KnownGap),
        rows.iter().filter(|row| row.outcome.is_problem()).count(),
        rows.len()
    )
    .expect("writing to a String does not fail");
    out.push_str("| Minecraft | Forge build | Installer | Java | Vanilla | Download | Install | Verify | Reinstall | Seconds | Result | Note |\n");
    out.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for row in rows {
        let cells = [
            row.minecraft.as_str(),
            row.forge.as_str(),
            row.layout.as_str(),
            row.java.as_str(),
            row.vanilla.as_str(),
            row.download.as_str(),
            row.install.as_str(),
            row.verify.as_str(),
            row.reinstall.as_str(),
        ];
        let seconds = row.seconds.to_string();
        let mut line = String::from("|");
        for cell in cells.iter().copied().chain([seconds.as_str(), row.outcome.label(), row.note.as_str()]) {
            line.push(' ');
            line.push_str(&cell_text(cell));
            line.push_str(" |");
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Writes the table to `path`, and prints it. A failed write is reported and the test goes on, so the table still prints.
pub(crate) fn write_and_print(rows: &[Row], path: &Path) {
    let text = render(rows);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(error) = std::fs::write(path, &text) {
        eprintln!("could not write the report to {}: {error}", path.display());
    }
    println!("{text}");
}

/// A cell's text on one line, with the pipes that would end the table escaped.
fn cell_text(text: &str) -> String {
    text.replace('\n', " ").replace('|', "\\|")
}
