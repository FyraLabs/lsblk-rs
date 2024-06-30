use std::{io::BufRead, path::PathBuf};

use crate::Res;

/// Represent a mountpoint
#[derive(Debug, Clone)]
pub struct Mount {
    /// The device name (either a path or something like zram0)
    pub device: String,
    /// The target directory it has been mounted to
    pub mountpoint: PathBuf,
    /// Type of filesystem
    pub fstype: String,
    /// Mount options
    pub mountopts: String,
}

impl Mount {
    pub fn list() -> Res<impl Iterator<Item = Mount>> {
        Ok(std::io::BufReader::new(
            std::fs::File::open(PathBuf::from("/proc/mounts"))
                .map_err(|e| crate::LsblkError::ReadFile("/proc/mounts".into(), e))?,
        )
        .lines()
        .filter_map(Result::ok)
        .filter_map(|l| {
            let mut parts = l.trim_end_matches(" 0 0").split(' ');
            Some(Mount {
                device: parts.next()?.into(),
                mountpoint: parts.next()?.into(),
                fstype: parts.next()?.into(),
                mountopts: parts.next()?.into(),
            })
        }))
    }
}

#[test]
fn test_list_mountpoints() {
    for x in Mount::list().unwrap() {
        println!("{x:?}");
    }
}
