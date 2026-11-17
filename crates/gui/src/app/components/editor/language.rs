//! Which language a file is written in, told from its name and the folder it sits in. The language picks the colours,
//! the comment mark and the indent the editor uses.

use crate::app::lang;

/// The languages the editor colours. Anything else is plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Language {
    #[default]
    Plain,
    /// JSON, JSON5, `.mcmeta` and stringified NBT.
    Json,
    Toml,
    Yaml,
    /// INI, `.cfg`, `.conf`, `.properties`, `.lang` and the game's `options.txt`.
    Ini,
    Xml,
    /// A game or launcher log.
    Log,
    /// A crash report from `crash-reports/`.
    Crash,
    /// A datapack function.
    Function,
    /// JavaScript and ZenScript, as KubeJS and CraftTweaker read them, and other languages with the same braces,
    /// comments and strings: Java, Kotlin script, Groovy and shader GLSL.
    Script,
}

impl Language {
    /// The language of the file at `path`, a path inside the instance with `/` separators.
    pub fn of(path: &str) -> Self {
        let lower = path.to_ascii_lowercase();
        let name = lower.rsplit('/').next().unwrap_or(&lower);
        let in_folder = |folder: &str| lower.starts_with(&format!("{folder}/")) || lower.contains(&format!("/{folder}/"));
        let extension = name.rsplit_once('.').map(|(_, extension)| extension);
        match extension {
            Some("json" | "json5" | "jsonc" | "mcmeta" | "snbt") => Self::Json,
            Some("toml") => Self::Toml,
            Some("yaml" | "yml") => Self::Yaml,
            Some("ini" | "cfg" | "conf" | "config" | "properties" | "lang") => Self::Ini,
            Some("xml" | "html" | "svg") => Self::Xml,
            Some("log") => Self::Log,
            Some("mcfunction") => Self::Function,
            Some("js" | "mjs" | "ts" | "zs" | "java" | "kts" | "kt" | "groovy" | "gradle" | "glsl" | "vsh" | "fsh" | "gsh" | "csh") => Self::Script,
            Some("txt") if in_folder("crash-reports") || name.starts_with("crash-") || name.starts_with("hs_err") => Self::Crash,
            Some("txt") if in_folder("logs") => Self::Log,
            // The game's own settings are `key:value` lines.
            Some("txt") if name.starts_with("options") => Self::Ini,
            _ => Self::Plain,
        }
    }

    /// The name the editor's footer shows.
    pub fn label(self) -> &'static str {
        match self {
            Self::Plain => lang::t("editor.language.plain"),
            Self::Json => "JSON",
            Self::Toml => "TOML",
            Self::Yaml => "YAML",
            Self::Ini => lang::t("editor.language.properties"),
            Self::Xml => "XML",
            Self::Log => lang::t("editor.language.log"),
            Self::Crash => lang::t("editor.language.crash"),
            Self::Function => "mcfunction",
            Self::Script => lang::t("editor.language.script"),
        }
    }

    /// The mark that starts a comment running to the end of the line, for the languages that have one.
    pub fn line_comment(self) -> Option<&'static str> {
        match self {
            Self::Toml | Self::Yaml | Self::Ini | Self::Function => Some("#"),
            Self::Json | Self::Script => Some("//"),
            Self::Plain | Self::Xml | Self::Log | Self::Crash => None,
        }
    }

    /// How many spaces one level of indent is, when the file itself does not say.
    pub fn indent_width(self) -> usize {
        match self {
            Self::Json | Self::Yaml | Self::Xml => 2,
            _ => 4,
        }
    }

    /// Whether a line that ends in a colon opens a block, as a YAML mapping does.
    pub fn colon_opens_block(self) -> bool {
        self == Self::Yaml
    }

    /// Whether brackets and quotes close themselves as they are typed. A log or a crash report is read, not written.
    pub fn pairs(self) -> bool {
        !matches!(self, Self::Log | Self::Crash)
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/components/editor/language.rs"]
mod tests;
