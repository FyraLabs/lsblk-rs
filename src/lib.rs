use std::{collections::HashMap, path::PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum LsblkError {
    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Fail to strip prefix `/dev/` from path")]
    StripPrefix(#[from] std::path::StripPrefixError),
}

fn ls_symlinks(
    dir: &std::path::Path,
) -> std::io::Result<impl Iterator<Item = Result<(String, String), LsblkError>>> {
    Ok(std::fs::read_dir(dir)?
        .filter_map(|f| f.ok())
        .filter(|f| f.metadata().is_ok_and(|f| f.is_symlink()))
        .map(|f| {
            let target = std::fs::read_link(f.path())?;
            let target = f
                .path()
                .parent()
                .unwrap()
                .join(target)
                .canonicalize()?
                .to_string_lossy()
                .to_string();
            let source = f.file_name().to_string_lossy().to_string();
            Ok((target, source))
        }))
}

/// A representation of a block-device
#[derive(Debug, Clone, Default)]
pub struct BlockDevice {
    /// the filename of the block-device.
    ///
    /// If the drive is deemed to be storage by the kernel, this is usually prefixed by one of the
    /// followings:
    /// - `sd`
    /// - `hd`
    /// - `vd`
    /// - `nvme`
    /// - `mmcblk`
    /// - `loop`
    pub name: String,
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
    pub fn list() -> Result<Vec<Self>, LsblkError> {
        let mut result = HashMap::new();
        macro_rules! insert {
            ($kind:ident) => {
                for x in ls_symlinks(&PathBuf::from(concat!("/dev/disk/by-", stringify!($kind))))? {
                    let (name, blk) = x?;
                    if let Some(bd) = result.get_mut(&name) {
                        bd.$kind = Some(blk);
                    } else {
                        result.insert(
                            name.to_string(),
                            Self {
                                name,
                                $kind: Some(blk),
                                ..Self::default()
                            },
                        );
                    }
                }
            };
        }
        for x in ls_symlinks(&PathBuf::from("/dev/disk/by-diskseq/"))? {
            let (name, blk) = x?;
            result.insert(
                name.to_string(), // FIXME: clone shouldn't be needed theoretically
                Self {
                    name,
                    diskseq: Some(blk),
                    ..BlockDevice::default()
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
    #[must_use]
    pub fn is_disk(&self) -> bool {
        !self.is_part()
            && (self.name.starts_with("sd")
                || self.name.starts_with("hd")
                || self.name.starts_with("vd")
                || self.name.starts_with("nvme")
                || self.name.starts_with("mmcblk")
                || self.name.starts_with("loop"))
    }

    /// Returns true if and only if the device is a partition.
    ///
    /// The implementation currently is just:
    /// ```rs
    /// self.uuid.is_some()
    /// ```
    #[must_use]
    pub fn is_part(&self) -> bool {
        self.uuid.is_some()
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
#[test]
fn test_lsblk_smoke() {
    let a = BlockDevice::list().expect("Valid lsblk");

    println!("{:#?}", a);
}
