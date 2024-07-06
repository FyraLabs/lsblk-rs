use crate::{ItRes, LsblkError, Res};
use std::{collections::HashMap, os::linux::fs::MetadataExt, path::PathBuf};

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

    /// Get the sysfs path for this block device.
    ///
    /// This relies on `sysfs(5)`, i.e. the file system mounted at `/sys`.
    ///
    /// # Errors
    /// All IO-related failures (including UTF-8 parsing) will be stored in [`std::io::Error`].
    pub fn sysfs(&self) -> std::io::Result<PathBuf> {
        let metadata = std::fs::metadata(&self.fullname)?;
        // Contains what device this file represents
        let rdev = metadata.st_rdev();

        // Adapted from https://docs.rs/nix/0.29.0/src/nix/sys/stat.rs.html#191
        let major = ((rdev >> 32) & 0xffff_f000) | ((rdev >> 8) & 0x0000_0fff);
        let minor = ((rdev >> 12) & 0xffff_ff00) | ((rdev) & 0x0000_00ff);

        Ok(PathBuf::from(format!("/sys/dev/block/{major}:{minor}/")))
    }

    /// If the block-device is a partition, trim out the partition from name and return the
    /// name of the disk.
    ///
    /// # Errors
    /// All IO-related failures will be stored in [`std::io::Error`].
    ///
    /// # Panics
    /// A panic will be raised if the disk name is not UTF-8 compliant or if the parent path is invalid
    pub fn disk_name(&self) -> std::io::Result<String> {
        if !self.is_part() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "can't get disk name for a block device that is not a partition",
            ));
        }

        let parent = std::fs::canonicalize(self.sysfs()?.join(".."))?;
        Ok(parent
            .file_name()
            .expect("file name is invalid, this shouldn't happen")
            .to_str()
            .expect("file name is not UTF-8 compliant")
            .to_owned())
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
    pub fn capacity(&self) -> std::io::Result<Option<u64>> {
        let p = self.sysfs()?.join("size");
        let s = std::fs::read_to_string(p)?;
        // remove new line char
        Ok(s[..s.len() - 1].parse().ok())
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
#[test]
fn test_lsblk_smoke() {
    let devs = BlockDevice::list().expect("Valid lsblk");
    for dev in devs.iter().filter(|d| d.is_part()) {
        let _ = dev.capacity().unwrap().unwrap();
        let _ = dev.sysfs().unwrap();
        let _ = dev.disk_name().unwrap();
    }
}
