use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Platform {
    #[default]
    Gba,
    Gb,
    Gbc,
    Gbav,
    Ps,
    Pico8,
}

impl Platform {
    pub const ALL: [Platform; 6] = [
        Platform::Gba,
        Platform::Gbav,
        Platform::Gbc,
        Platform::Gb,
        Platform::Ps,
        Platform::Pico8,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Platform::Gba => "Game Boy Advance",
            Platform::Gb => "Game Boy",
            Platform::Gbc => "Game Boy Color",
            Platform::Gbav => "Game Boy Advance Video",
            Platform::Ps => "PlayStation",
            Platform::Pico8 => "PICO-8",
        }
    }

    pub fn dir_name(self) -> &'static str {
        match self {
            Platform::Gba => "GBA",
            Platform::Gb => "GB",
            Platform::Gbc => "GBC",
            Platform::Gbav => "GBAV",
            Platform::Ps => "PS",
            Platform::Pico8 => "PICO8",
        }
    }

    pub fn native(self) -> bool {
        matches!(self, Platform::Ps | Platform::Pico8)
    }

    /// Native formats can share a title; keep their complete filenames as identities.
    pub fn stem(self, path: &Path) -> Option<&str> {
        if self.native() {
            path.file_name()
        } else {
            path.file_stem()
        }
        .and_then(|s| s.to_str())
    }

    pub fn hardware(self) -> Platform {
        match self {
            Platform::Gbav => Platform::Gba,
            p => p,
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Platform::Gba | Platform::Gbav => &["gba"],
            Platform::Gb | Platform::Gbc => &["gb", "gbc"],
            Platform::Ps => &["cue", "chd", "pbp", "m3u"],
            Platform::Pico8 => &["p8", "p8.png"],
        }
    }

    pub fn picture(self) -> (u32, u32) {
        match self {
            Platform::Gba | Platform::Gbav => (240, 160),
            Platform::Gb | Platform::Gbc => (160, 144),
            Platform::Ps => (320, 240),
            Platform::Pico8 => (128, 128),
        }
    }

    pub fn accepts(self, path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                let name = name.to_ascii_lowercase();
                self.extensions().iter().any(|ext| {
                    name.strip_suffix(ext)
                        .is_some_and(|prefix| prefix.ends_with('.'))
                })
            })
    }
}
