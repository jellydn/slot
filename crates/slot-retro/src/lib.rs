macro_rules! eprintln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

mod core;
mod ffi;
mod libretro;
mod link;
mod mock;
pub mod rtc;
mod rumble;

#[cfg(feature = "host-probe")]
pub mod host_probe {
    pub use crate::ffi::*;
}

pub use core::{AvInfo, ButtonMask, Controller, CoreError, FrameGeometry, RetroCore, GBA_H, GBA_W};
pub use ffi::{
    NETPACKET_BROADCAST, NETPACKET_FLUSH_HINT, NETPACKET_RELIABLE, NETPACKET_UNRELIABLE,
    NETPACKET_UNSEQUENCED,
};
pub use libretro::LibretroCore;
pub use link::{Link, LinkChannel, LoopbackLink};
pub use mock::MockCore;
pub use rumble::Rumble;
