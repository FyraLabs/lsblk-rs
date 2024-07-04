//! A library that lists out block-devices.
//!
//! Check out [`BlockDevice::list`].
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
pub mod mountpoints;
pub use mountpoints::Mount;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(thiserror::Error, Debug)]
pub enum LsblkError {
    #[error("Cannot read directory {0:?}: {1}")]
    ReadDir(PathBuf, std::io::Error),
    #[error("Cannot canonicalize broken symlink for {0:?}: {1}")]
    BadSymlink(PathBuf, std::io::Error),
    #[error("Cannot read file content from {0:?}: {1}")]
    ReadFile(PathBuf, std::io::Error),
}

pub(crate) type Res<T> = Result<T, LsblkError>;
pub(crate) type ItRes<T> = dyn Iterator<Item = Res<T>>;

fn ls_symlinks(dir: &std::path::Path) -> Res<Box<ItRes<(PathBuf, String)>>> {
    Ok(if dir.exists() {
        Box::new(
            std::fs::read_dir(dir)
                .map_err(|e| LsblkError::ReadDir(dir.to_path_buf(), e))?
                .filter_map(Result::ok)
                .filter(|f| f.metadata().is_ok_and(|f| f.is_symlink()))
                .map(|f| {
                    let dest = (f.path().canonicalize()) // this also resolves the symlink
                        .map_err(|e| LsblkError::BadSymlink(f.path(), e))?;
                    let src = f.file_name().to_string_lossy().to_string();
                    Ok((dest, src))
                }),
        )
    } else {
        Box::new(std::iter::empty())
    })
}

/// A representation of a block-device
#[derive(Debug, Clone, Default)]
pub struct BlockDevice {
    /// the filename of the block-device.
    pub name: String,
    /// The full name of the block-device, which is basically `/dev/{name}`.
    pub fullname: PathBuf,
    /// The diskseq of the device as in `/dev/disk/by-diskseq/`.
    pub diskseq: Option<String>,
    /// The path (not the filesystem!) of the device as in `/dev/disk/by-path`.
    pub path: Option<String>,
    /// The device UUID.
    pub uuid: Option<String>,
    /// The UUID of a partition (not the same as device UUID).
    pub partuuid: Option<String>,
    /// The label of the partition.
    pub label: Option<String>,
    /// The partition label (not the same as `label`), as in `/dev/disk/by-partlabel`)
    pub partlabel: Option<String>,
    /// The id of the device as in `/dev/disk/by-id/`.
    pub id: Option<String>,
}

impl BlockDevice {
    /// List out all found block devices and populate all fields.
    ///
    /// # Panics
    /// If somehow there exists a device that isn't in `/dev/`, the function panics.
    ///
    /// # Errors
    /// There are no particular errors other than IO / symlink resolution failures, etc.
    pub fn list() -> Result<Vec<Self>, LsblkError> {
        let mut result = HashMap::new();
        macro_rules! insert {
            ($kind:ident) => {
                for x in ls_symlinks(&PathBuf::from(concat!("/dev/disk/by-", stringify!($kind))))? {
                    let (fullname, blk) = x?;
                    let name = fullname
                        .strip_prefix("/dev/")
                        .expect("Cannot strip /dev")
                        .to_string_lossy()
                        .to_string();
                    if let Some(bd) = result.get_mut(&name) {
                        bd.$kind = Some(blk);
                    } else {
                        result.insert(
                            name.to_string(),
                            Self {
                                name,
                                fullname,
                                $kind: Some(blk),
                                ..Self::default()
                            },
                        );
                    }
                }
            };
        }
        for x in ls_symlinks(&PathBuf::from("/dev/disk/by-diskseq/"))? {
            let (fullname, blk) = x?;
            let name = fullname
                .strip_prefix("/dev/")
                .expect("Cannot strip /dev")
                .to_string_lossy()
                .to_string();
            result.insert(
                name.to_string(), // FIXME: clone shouldn't be needed theoretically
                Self {
                    name,
                    fullname,
                    diskseq: Some(blk),
                    ..Self::default()
                },
            );
        }
        insert!(path);
        insert!(uuid);
        insert!(partuuid);
        insert!(label);
        insert!(partlabel);
        insert!(id);
        Ok(result.into_values().collect())
    }

    /// Returns true if and only if the device is a storage disk and is not a partition.
    ///
    /// The implementation currently is just:
    /// ```rs
    /// !self.is_part()
    /// ```
    #[must_use]
    pub const fn is_disk(&self) -> bool {
        !self.is_part()
    }

    /// Determines if the block-device is considered to be physical.
    /// This can be a partition or a disk.
    ///
    /// A "physical" disk is one that has a path as in `/dev/disk/by-path`
    ///
    /// The implementation currently is just:
    /// ```rs
    /// self.path.is_some()
    /// ```
    #[must_use]
    pub const fn is_physical(&self) -> bool {
        self.path.is_some()
    }

    /// Returns true if and only if the device is a partition.
    ///
    /// The implementation currently is just:
    /// ```rs
    /// self.partuuid.is_some()
    /// ```
    #[must_use]
    pub const fn is_part(&self) -> bool {
        self.partuuid.is_some()
    }

    /// If the block-device is a partition, trim out the partition from name and return the
    /// name of the disk.
    ///
    /// This function is **_EXPENSIVE_** because IO is involved. Specifically, this function reads
    /// the content of the directory `/sys/block` for a list of disks.
    ///
    /// # Assumptions
    /// - All disk names are UTF-8 compliant
    /// - All files in the directory `/sys/block` (not recursively) are accessible.
    #[must_use]
    pub fn disk_name(&self) -> Option<String> {
        for disk in std::fs::read_dir(Path::new("/sys/block")).ok()? {
            let diskname = disk.ok()?.file_name();
            let diskname = diskname.to_str()?;
            if self.name.starts_with(diskname) {
                return Some(diskname.to_owned());
            }
        }
        None
    }

    /// Fetch the capacity of the block-device.
    ///
    /// This relies on `sysfs(5)`, i.e. the file system mounted at `/sys`.
    ///
    /// The returned value * 512 = size in bytes.
    ///
    /// # Errors
    /// All IO-related failures (including UTF-8 parsing) will be stored in [`std::io::Error`]. If
    /// the output is `Ok(None)`, that means there was a failure trying to parse the text inside
    /// `/sys/block/<device>/size`.
    ///
    /// # Panics
    /// A panic will be raised if there exists a partition identified via [`Self::is_part`] that
    /// does not have a [`Self::disk_name`]. This assumes any partition should belong to a disk.
    pub fn capacity(&self) -> std::io::Result<Option<u64>> {
        let p = Path::new("/sys/block");
        let p = if self.is_part() {
            p.join(self.disk_name().expect("Can't determine disk of part"))
                .join(&self.name)
        } else {
            p.join(&self.name)
        };
        let s = std::fs::read_to_string(p)?;
        Ok(s.parse().ok())
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
#[test]
fn test_lsblk_smoke() {
    BlockDevice::list().expect("Valid lsblk");
}
