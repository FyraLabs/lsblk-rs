use crate::{ItRes, LsblkError, Res};
use std::os::linux::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub(crate) fn ls_symlinks(dir: &Path) -> Res<Box<ItRes<(PathBuf, String)>>> {
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
        let mut result = std::collections::HashMap::new();
        macro_rules! insert {
            ($kind:ident) => {
                for x in ls_symlinks(Path::new(concat!("/dev/disk/by-", stringify!($kind))))? {
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
        for x in ls_symlinks(Path::new("/dev/disk/by-diskseq/"))? {
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

    /// Create a [`BlockDevice`] from a path that is either `/dev/{name}` or a path to a (sym)link
    /// that points to `/dev/{name}`.
    ///
    /// Note that this function is rather expensive (because it needs to list out all links in
    /// `/dev/disks/by-diskseq/` and other directories in the worst case scenario to find the one
    /// that links to `/dev/{name}`). Therefore, you should prefer [`BlockDevice::list()`] instead
    /// if you would like to list out more than 1 blockdevice.
    ///
    /// If you would like to not populate all fields for now, use
    /// [`BlockDevice::from_path_unpopulated()`] instead.
    ///
    /// # Panics
    /// If somehow this isn't in `/dev/`, the function panics.
    ///
    /// # Errors
    /// There are no particular errors other than IO / symlink resolution failures, etc.
    pub fn from_path<P: AsRef<Path>>(p: P) -> Result<Self, LsblkError> {
        let pathbuf = (p.as_ref().canonicalize())
            .map_err(|e| LsblkError::BadSymlink(p.as_ref().to_owned(), e))?;
        let mut res = Self::from_abs_path_unpopulated(pathbuf.clone());
        macro_rules! insert {
            ($kind:ident) => {
                if let Some(Ok((_, blk))) =
                    ls_symlinks(Path::new(concat!("/dev/disk/by-", stringify!($kind))))?
                        .find(|elm| elm.as_ref().is_ok_and(|(fullname, _)| fullname == &pathbuf))
                {
                    res.$kind = Some(blk);
                }
            };
        }
        insert!(diskseq);
        insert!(path);
        insert!(uuid);
        insert!(partuuid);
        insert!(label);
        insert!(partlabel);
        insert!(id);
        Ok(res)
    }

    /// Create a [`BlockDevice`] from a path that is either `/dev/{name}` or a path to a (sym)link
    /// that points to `/dev/{name}`.
    ///
    /// This is the same as [`BlockDevice::from_path`] except that **none of the fields other than
    /// `name` and `fullname` are populated**.
    ///
    /// To manually populate the fields, use [`crate::Populate`].
    ///
    /// # Panics
    /// If somehow this isn't in `/dev/`, the function panics.
    ///
    /// # Errors
    /// The function returns an error if the path cannot be canonicalized.
    pub fn from_path_unpopulated<P: AsRef<Path>>(p: P) -> std::io::Result<Self> {
        Ok(Self::from_abs_path_unpopulated(p.as_ref().canonicalize()?))
    }

    /// Create a [`BlockDevice`] from a path in the format of `/dev/{name}`.
    ///
    /// WARN: This function does NOT accept links or relative paths.
    /// If this is unacceptable, use [`BlockDevice::from_path_unpopulated`] instead.
    ///
    /// # Panics
    /// If somehow this isn't in `/dev/`, the function panics.
    #[must_use]
    pub fn from_abs_path_unpopulated(absolute_path: PathBuf) -> Self {
        Self {
            name: absolute_path
                .strip_prefix("/dev/")
                .expect("Cannot strip /dev")
                .to_string_lossy()
                .to_string(),
            fullname: absolute_path,
            ..Self::default()
        }
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
    /// A "physical" disk is one that has a path as in `/dev/disk/by-path`.
    ///
    /// An exception is eMMC drives which for some reason do not come with paths.
    #[must_use]
    pub fn is_physical(&self) -> bool {
        // TODO: make this const fn once as_bytes is stable
        self.path.is_some() || matches!(self.name.as_bytes().split_at(6).0, b"mmcblk")
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
    /// Failure to stat the device file using [`std::fs::metadata`] will result in [`std::io::Error`].
    pub fn sysfs(&self) -> std::io::Result<PathBuf> {
        let (major, minor) = self.major_minor()?;
        Ok(PathBuf::from(format!("/sys/dev/block/{major}:{minor}/")))
    }

    /// Get the major, minor ID of the block-device.
    ///
    /// This stats the device file in order to obtain its device ID.
    ///
    /// # Errors
    /// Failure to stat the device file using [`std::fs::metadata`] will result in [`std::io::Error`].
    pub fn major_minor(&self) -> std::io::Result<(u32, u32)> {
        let metadata = std::fs::metadata(&self.fullname)?;
        // Contains what device this file represents
        let rdev = metadata.st_rdev();

        // Adapted from https://docs.rs/nix/0.29.0/src/nix/sys/stat.rs.html#191
        let major = ((rdev >> 32) & 0xffff_f000) | ((rdev >> 8) & 0x0000_0fff);
        let minor = ((rdev >> 12) & 0xffff_ff00) | (rdev & 0x0000_00ff);
        #[allow(clippy::cast_possible_truncation)]
        Ok((major as u32, minor as u32)) // guaranteed by bit filters
    }

    /// If the block-device is a partition, look up the parent disk in sysfs and return its
    /// name. Otherwise, returns [`BlockDevice::name`] if not a partition.
    ///
    /// # Errors
    /// All IO-related failures will be stored in [`std::io::Error`].
    ///
    /// # Panics
    /// A panic will be raised if the disk name is not UTF-8 compliant or if the parent path is invalid
    pub fn disk_name(&self) -> std::io::Result<String> {
        if !self.is_part() {
            return Ok(self.name.clone());
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
#[allow(clippy::unwrap_used)]
#[test]
fn test_lsblk_smoke() {
    let devs = BlockDevice::list().expect("Valid lsblk");
    for dev in devs.iter().filter(|d| d.is_part()) {
        println!("{}", dev.capacity().unwrap().unwrap());
        println!("{:?}", dev.sysfs().unwrap());
        println!("{}", dev.disk_name().unwrap());
    }
}
