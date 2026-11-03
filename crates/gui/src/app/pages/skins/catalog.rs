//! The skins Lodestone ships: Mojang's vanilla default skins, and the skins from Mojang's free Java skin packs that Mojang still
//! serves. Each PNG is Mojang's own file, and each matches the SHA-256 that names it on Mojang's texture server.

/// One bundled skin: its name, the pack it belongs to, its model, and its PNG.
pub struct Bundled {
    pub name: &'static str,
    pub pack: Pack,
    pub slim: bool,
    pub png: &'static [u8],
}

/// The sections the bundled skins are grouped in: the vanilla defaults, then the packs in the order Mojang released them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pack {
    Defaults,
    Minecon2017,
    BuildersAndBiomes,
    StridingHero,
    TheGardenAwakens,
    ChaseTheSkies,
    TheCopperAge,
    MountsOfMayhem,
}

impl Pack {
    /// Every pack, in the order the page lists them.
    pub const ALL: [Pack; 8] = [
        Pack::Defaults,
        Pack::Minecon2017,
        Pack::BuildersAndBiomes,
        Pack::StridingHero,
        Pack::TheGardenAwakens,
        Pack::ChaseTheSkies,
        Pack::TheCopperAge,
        Pack::MountsOfMayhem,
    ];

    /// The pack's title as Mojang names it. The vanilla defaults have no such title: the page names them in the language, so
    /// `None` is the defaults section.
    pub fn title(self) -> Option<&'static str> {
        Some(match self {
            Pack::Defaults => return None,
            Pack::Minecon2017 => "MINECON Earth 2017",
            Pack::BuildersAndBiomes => "Builders & Biomes",
            Pack::StridingHero => "Striding Hero",
            Pack::TheGardenAwakens => "The Garden Awakens",
            Pack::ChaseTheSkies => "Chase the Skies",
            Pack::TheCopperAge => "The Copper Age",
            Pack::MountsOfMayhem => "Mounts of Mayhem",
        })
    }
}

/// Every bundled skin, in the order the page lists them within each pack.
pub const BUNDLED: &[Bundled] = &[
    Bundled {
        name: "Alex",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/alex-slim.png"),
    },
    Bundled {
        name: "Alex",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/alex-classic.png"),
    },
    Bundled {
        name: "Ari",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/ari-slim.png"),
    },
    Bundled {
        name: "Ari",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/ari-classic.png"),
    },
    Bundled {
        name: "Efe",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/efe-slim.png"),
    },
    Bundled {
        name: "Efe",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/efe-classic.png"),
    },
    Bundled {
        name: "Kai",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/kai-slim.png"),
    },
    Bundled {
        name: "Kai",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/kai-classic.png"),
    },
    Bundled {
        name: "Makena",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/makena-slim.png"),
    },
    Bundled {
        name: "Makena",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/makena-classic.png"),
    },
    Bundled {
        name: "Noor",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/noor-slim.png"),
    },
    Bundled {
        name: "Noor",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/noor-classic.png"),
    },
    Bundled {
        name: "Steve",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/steve-slim.png"),
    },
    Bundled {
        name: "Steve",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/steve-classic.png"),
    },
    Bundled {
        name: "Sunny",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/sunny-slim.png"),
    },
    Bundled {
        name: "Sunny",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/sunny-classic.png"),
    },
    Bundled {
        name: "Zuri",
        pack: Pack::Defaults,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/zuri-slim.png"),
    },
    Bundled {
        name: "Zuri",
        pack: Pack::Defaults,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/vanilla/zuri-classic.png"),
    },
    Bundled {
        name: "Globe Alex",
        pack: Pack::Minecon2017,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/minecon-earth-2017/globe-alex-classic.png"),
    },
    Bundled {
        name: "Party Alex",
        pack: Pack::Minecon2017,
        slim: true,
        png: include_bytes!("../../../../../../res/artwork/skins/minecon-earth-2017/party-alex-slim.png"),
    },
    Bundled {
        name: "Cardboard Cosplayer",
        pack: Pack::Minecon2017,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/minecon-earth-2017/cardboard-cosplayer-classic.png"),
    },
    Bundled {
        name: "Creeper Cosplayer",
        pack: Pack::Minecon2017,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/minecon-earth-2017/creeper-cosplayer-classic.png"),
    },
    Bundled {
        name: "Creeper Piñata Cosplayer",
        pack: Pack::Minecon2017,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/minecon-earth-2017/creeper-pi-ata-cosplayer-classic.png"),
    },
    Bundled {
        name: "Sheep Cosplayer",
        pack: Pack::Minecon2017,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/minecon-earth-2017/sheep-cosplayer-classic.png"),
    },
    Bundled {
        name: "Cake Steve",
        pack: Pack::Minecon2017,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/minecon-earth-2017/cake-steve-classic.png"),
    },
    Bundled {
        name: "Party Steve",
        pack: Pack::Minecon2017,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/minecon-earth-2017/party-steve-classic.png"),
    },
    Bundled {
        name: "Barn Builder",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/barn-builder-classic.png"),
    },
    Bundled {
        name: "Bee-Friender",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/bee-friender-classic.png"),
    },
    Bundled {
        name: "Bee-Friender (Alternate)",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/bee-friender-alternate-classic.png"),
    },
    Bundled {
        name: "Buff Butcher",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/buff-butcher-classic.png"),
    },
    Bundled {
        name: "Buff Butcher (Alternate)",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/buff-butcher-alternate-classic.png"),
    },
    Bundled {
        name: "Homestead Healer",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/homestead-healer-classic.png"),
    },
    Bundled {
        name: "Pig Whisperer",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/pig-whisperer-classic.png"),
    },
    Bundled {
        name: "Pig Whisperer (Alternate)",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/pig-whisperer-alternate-classic.png"),
    },
    Bundled {
        name: "Ranch Ranger",
        pack: Pack::BuildersAndBiomes,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/builders-and-biomes/ranch-ranger-classic.png"),
    },
    Bundled {
        name: "Snowfeather",
        pack: Pack::StridingHero,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/striding-hero/snowfeather-classic.png"),
    },
    Bundled {
        name: "Stray",
        pack: Pack::StridingHero,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/striding-hero/stray-classic.png"),
    },
    Bundled {
        name: "Strider",
        pack: Pack::StridingHero,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/striding-hero/strider-classic.png"),
    },
    Bundled {
        name: "Villager 1",
        pack: Pack::StridingHero,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/striding-hero/villager-1-classic.png"),
    },
    Bundled {
        name: "Villager 2",
        pack: Pack::StridingHero,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/striding-hero/villager-2-classic.png"),
    },
    Bundled {
        name: "Wither Skeleton",
        pack: Pack::StridingHero,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/striding-hero/wither-skeleton-classic.png"),
    },
    Bundled {
        name: "Pale Lumberjack",
        pack: Pack::TheGardenAwakens,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/the-garden-awakens/pale-lumberjack-classic.png"),
    },
    Bundled {
        name: "Creaking",
        pack: Pack::TheGardenAwakens,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/the-garden-awakens/creaking-classic.png"),
    },
    Bundled {
        name: "Ghast Pilot",
        pack: Pack::ChaseTheSkies,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/chase-the-skies/ghast-pilot-classic.png"),
    },
    Bundled {
        name: "Ghast Swimmer",
        pack: Pack::ChaseTheSkies,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/chase-the-skies/ghast-swimmer-classic.png"),
    },
    Bundled {
        name: "Copper Chemist",
        pack: Pack::TheCopperAge,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/the-copper-age/copper-chemist-classic.png"),
    },
    Bundled {
        name: "Copper Welder",
        pack: Pack::TheCopperAge,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/the-copper-age/copper-welder-classic.png"),
    },
    Bundled {
        name: "Zombie Horse Onesie",
        pack: Pack::MountsOfMayhem,
        slim: false,
        png: include_bytes!("../../../../../../res/artwork/skins/mounts-of-mayhem/zombie-horse-onesie-classic.png"),
    },
];

#[cfg(test)]
#[path = "../../../../tests/app/pages/skins/catalog.rs"]
mod tests;
