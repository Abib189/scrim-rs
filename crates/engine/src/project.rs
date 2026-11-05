//! Who keeps Scrim, for people to reach (Fix R-6). The one place for the address: the About
//! dialog shows it, `xtask` writes it into the Info.plist's copyright line, `package.sh` reads it
//! from here, and NOTICE and README carry it (a test in `xtask` keeps them equal, and keeps it out
//! of everything else).

/// The maintainers' address.
pub const MAINTAINER_CONTACT: &str = "hello@scrim.design";
