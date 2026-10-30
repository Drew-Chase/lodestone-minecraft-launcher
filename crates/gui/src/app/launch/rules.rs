//! Mojang's rule conditions. A library or an argument can list rules naming the operating
//! system, the architecture or a feature, and only the rules that match apply. The launcher
//! runs with no optional features, so a rule that needs one to be on never applies.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The platform a launch runs on, named the way Mojang's version JSON names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    /// `windows`, `osx` or `linux`.
    pub os: &'static str,
    /// `x86` for 32-bit builds, `arm64` for ARM, and `x86_64` otherwise.
    pub arch: &'static str,
}

impl Platform {
    /// The platform this build runs on.
    pub const fn current() -> Self {
        Self {
            os: current_os(),
            arch: current_arch(),
        }
    }

    /// The separator between classpath entries.
    pub fn classpath_separator(self) -> char {
        if self.os == "windows" { ';' } else { ':' }
    }

    /// The `${arch}` a native classifier names: `64` on a 64-bit build, `32` otherwise.
    pub fn pointer_width(self) -> &'static str {
        if cfg!(target_pointer_width = "64") { "64" } else { "32" }
    }
}

const fn current_os() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

const fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    }
}

/// One rule from a library or an argument.
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: Action,
    #[serde(default)]
    pub os: Option<OsCondition>,
    #[serde(default)]
    pub features: Option<BTreeMap<String, bool>>,
}

/// Whether a matching rule allows or disallows what it is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Disallow,
}

/// The operating system and architecture a rule applies to. The version a rule may name is
/// not checked: every version matches.
#[derive(Debug, Clone, Deserialize)]
pub struct OsCondition {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

/// Whether `rules` allow `platform`. A missing list allows. A present list allows only when
/// the last rule that applies says `allow`, so a list where no rule applies allows nothing.
pub fn allowed(rules: Option<&[Rule]>, platform: Platform) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    rules
        .iter()
        .rfind(|rule| rule.applies(platform))
        .is_some_and(|rule| rule.action == Action::Allow)
}

impl Rule {
    fn applies(&self, platform: Platform) -> bool {
        let os_matches = self.os.as_ref().is_none_or(|os| os.matches(platform));
        // Every feature is off, so a rule asking for one on never applies and a rule asking
        // for it off always does.
        let features_match = self.features.as_ref().is_none_or(|features| features.values().all(|wanted| !wanted));
        os_matches && features_match
    }
}

impl OsCondition {
    fn matches(&self, platform: Platform) -> bool {
        self.name.as_deref().is_none_or(|name| name == platform.os) && self.arch.as_deref().is_none_or(|arch| arch == platform.arch)
    }
}

#[cfg(test)]
#[path = "../../../tests/app/launch/rules.rs"]
mod tests;
