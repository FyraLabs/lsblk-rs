use std::{io::BufRead, path::PathBuf};

use crate::Res;

/// Represent a mountpoint
#[derive(Debug, Clone, Default)]
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
    /// Since this function depends on the existence of `/proc/mounts`, failures to open the file
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
    #[rustfmt::skip] //? https://github.com/rust-lang/rustfmt/issues/3157#issuecomment-2213427895
    pub fn list() -> Res<impl Iterator<Item = Self>> {
        Ok(
            std::io::BufReader::new(
                std::fs::File::open(PathBuf::from("/proc/mounts"))
                    .map_err(|e| crate::LsblkError::ReadFile("/proc/mounts".into(), e))?,
            )
            .lines()
            .map_while(Result::ok)
            .filter_map(|l| {
                let mut parts = l.trim_end_matches(" 0 0").split(' ');
                Some(Self {
                    device: parts.next()?.into(),
                    mountpoint: parts.next()?.into(),
                    fstype: parts.next()?.into(),
                    mountopts: parts.next()?.into(),
                })
            })
        )
    }
    /// List out the mounting options (`fs_mntopts`).
    ///
    /// This returns an iterator of (key, optional value).
    ///
    /// # Examples
    /// ```
    /// let mountopts = String::from("rw,relatime,compress=zstd:1,ssd,discard=async,subvol=/root");
    /// let m = lsblk::Mount {
    ///     mountopts,
    ///     ..lsblk::Mount::default()
    /// };
    /// let mut it = m.iter_mountopts();
    /// assert_eq!(it.next(), Some(("rw", None)));
    /// assert_eq!(it.next(), Some(("relatime", None)));
    /// assert_eq!(it.next(), Some(("compress", Some("zstd:1"))));
    /// assert_eq!(it.next(), Some(("ssd", None)));
    /// assert_eq!(it.next(), Some(("discard", Some("async"))));
    /// assert_eq!(it.next(), Some(("subvol", Some("/root"))));
    /// assert_eq!(it.next(), None);
    /// ```
    pub fn iter_mountopts(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.mountopts
            .split(',')
            .map(|x| x.split_once('=').map_or((x, None), |(k, v)| (k, Some(v))))
    }
}

#[test]
fn test_list_mountpoints() -> Res<()> {
    for x in Mount::list()? {
        println!("{x:?}");
    }
    Ok(())
}
