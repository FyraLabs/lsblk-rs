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
    /// List out all mountpoints and populate all fields.
    ///
    /// # Errors
    /// Since this function depends on the existance of `/proc/mounts`, failures to open the file
    /// will cause [`crate::LsblkError::ReadFile`].
    ///
    /// # Caveats
    /// If for some reason `/proc/mounts` is not formatted properly, the iterator will skip those
    /// lines. This includes
    /// - trailing whitespace
    /// - `fs_freq` and `fs_passno` (which are the last 2 fields on each line) not set to 0
    /// - not separating the fields with only 1 single space (`' '`)
    ///
    /// For more information, visit [`proc_pid_mounts(5)`](https://man.archlinux.org/man/proc_mounts.5.en).
    ///
    /// # Examples
    /// ```
    /// for m in lsblk::Mount::list()? {
    ///     println!("{} mounted at {}", m.device, m.mountpoint.display());
    /// }
    /// # Ok::<(), lsblk::LsblkError>(())
    /// ```
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
