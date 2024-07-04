//! A library that lists out block-devices.
//!
//! Check out [`BlockDevice::list`] and [`Mount::list`].
#![warn(clippy::complexity)]
#![warn(clippy::correctness)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![warn(clippy::suspicious)]
// followings are from clippy::restriction
#![warn(clippy::missing_errors_doc)]
#![warn(clippy::missing_panics_doc)]
#![warn(clippy::missing_safety_doc)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::format_push_string)]
#![warn(clippy::get_unwrap)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::implicit_return)]
#![allow(clippy::blanket_clippy_restriction_lints)]
#![allow(clippy::pattern_type_mismatch)]
pub mod blockdevs;
pub mod mountpoints;
pub use blockdevs::BlockDevice;
pub use mountpoints::Mount;

#[derive(thiserror::Error, Debug)]
pub enum LsblkError {
    #[error("Cannot read directory {0:?}: {1}")]
    ReadDir(std::path::PathBuf, std::io::Error),
    #[error("Cannot canonicalize broken symlink for {0:?}: {1}")]
    BadSymlink(std::path::PathBuf, std::io::Error),
    #[error("Cannot read file content from {0:?}: {1}")]
    ReadFile(std::path::PathBuf, std::io::Error),
}

pub(crate) type Res<T> = Result<T, LsblkError>;
pub(crate) type ItRes<T> = dyn Iterator<Item = Res<T>>;
