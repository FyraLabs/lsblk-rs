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
            let target = target.strip_prefix("/dev/")?.to_string_lossy().to_string();
            let source = f.file_name().to_string_lossy().to_string();
            Ok((target, source))
        }))
}

#[derive(Debug, Clone, Default)]
pub struct BlockDevice {
    pub name: String,
    pub diskseq: Option<String>,
    pub path: Option<String>,
    pub uuid: Option<String>,
    pub partuuid: Option<String>,
    pub label: Option<String>,
    pub partlabel: Option<String>,
    pub id: Option<String>,
}

impl BlockDevice {
    #[must_use]
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

    #[must_use]
    pub fn is_part(&self) -> bool {
        self.uuid.is_some()
    }
}
