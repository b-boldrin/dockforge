//! Everything that talks to the operating system lives behind this module.
//!
//! On Windows it uses the Win32 API. On other systems a stub returns sample data so the
//! interface and themes can be worked on from macOS or Linux too.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(not(windows))]
mod other;
#[cfg(not(windows))]
pub use self::other::*;
