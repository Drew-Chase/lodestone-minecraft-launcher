//! Per-platform provider implementations.

pub mod atlauncher;
pub mod curseforge;
pub mod ftb;
pub mod modrinth;
pub mod technic;
mod user_agent;

pub use atlauncher::AtLauncherProvider;
pub use curseforge::CurseForgeProvider;
pub use ftb::FtbProvider;
pub use modrinth::ModrinthProvider;
pub use technic::TechnicProvider;
