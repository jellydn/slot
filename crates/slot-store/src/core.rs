use std::collections::HashMap;
use std::path::Path;

use crate::Platform;

pub const SELECTED_CORE_FILE: &str = "Config/selected_core.txt";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Core {
    #[default]
    Mgba,
    Gpsp,
    Gambatte,
    PcsxRearmed,
    Fake08,
    SwanStation,
}

impl Core {
    pub const ALL: [Core; 6] = [
        Core::Mgba,
        Core::Gpsp,
        Core::Gambatte,
        Core::PcsxRearmed,
        Core::Fake08,
        Core::SwanStation,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Core::Mgba => "mgba",
            Core::Gpsp => "gpsp",
            Core::Gambatte => "gambatte",
            Core::PcsxRearmed => "pcsx_rearmed",
            Core::Fake08 => "fake08",
            Core::SwanStation => "swanstation",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            Core::Mgba => "mGBA",
            Core::Gpsp => "gpSP",
            Core::Gambatte => "Gambatte",
            Core::PcsxRearmed => "PCSX ReARMed",
            Core::Fake08 => "fake-08",
            Core::SwanStation => "SwanStation",
        }
    }

    pub fn parse(s: &str) -> Option<Core> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mgba" => Some(Core::Mgba),
            "gpsp" => Some(Core::Gpsp),
            "gambatte" => Some(Core::Gambatte),
            "pcsx_rearmed" => Some(Core::PcsxRearmed),
            "fake08" => Some(Core::Fake08),
            "swanstation" => Some(Core::SwanStation),
            _ => None,
        }
    }

    pub fn runs(self, platform: Platform) -> bool {
        match self {
            Core::Mgba => !platform.native(),
            Core::Gpsp => platform.hardware() == Platform::Gba,
            Core::Gambatte => matches!(platform, Platform::Gb | Platform::Gbc),
            Core::PcsxRearmed | Core::SwanStation => platform == Platform::Ps,
            Core::Fake08 => platform == Platform::Pico8,
        }
    }

    pub fn pair(platform: Platform) -> Option<[Core; 2]> {
        match platform {
            Platform::Gba | Platform::Gbav => Some([Core::Mgba, Core::Gpsp]),
            Platform::Gb | Platform::Gbc => Some([Core::Mgba, Core::Gambatte]),
            Platform::Ps => Some([Core::PcsxRearmed, Core::SwanStation]),
            Platform::Pico8 => None,
        }
    }
}

pub fn read_selected_cores(root: &Path) -> HashMap<String, Core> {
    crate::ini::read(root, SELECTED_CORE_FILE)
        .into_iter()
        .filter_map(|(stem, name)| Core::parse(&name).map(|core| (stem, core)))
        .collect()
}

pub fn core_for_platform(root: &Path, stem: &str, platform: Platform, gb_core: Core) -> Core {
    crate::ini::value(root, SELECTED_CORE_FILE, stem)
        .as_deref()
        .and_then(Core::parse)
        .filter(|core| core.runs(platform))
        .unwrap_or_else(|| match platform {
            Platform::Gb | Platform::Gbc if gb_core.runs(platform) => gb_core,
            Platform::Ps => Core::PcsxRearmed,
            Platform::Pico8 => Core::Fake08,
            _ => Core::Mgba,
        })
}

pub fn write_selected_core(root: &Path, stem: &str, core: Core) -> std::io::Result<()> {
    crate::ini::write(root, SELECTED_CORE_FILE, stem, core.as_str())
}
