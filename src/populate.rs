use crate::blockdevs::ls_symlinks;
use crate::BlockDevice;
use std::path::Path;

macro_rules! generate_populate_trait {
    ($Populate:ident $($by:ident)+) => { ::paste::paste! {
        pub trait $Populate {
            fn as_mut(&mut self) -> &mut BlockDevice;
            fn as_ref(&self) -> &BlockDevice;
            $(
                /// Populate the field by reading symbolic links in `/dev/disk/` until the correct
                /// blockdevice is found.
                ///
                /// This could be expensive depending on the amount of blockdevices avaiable for
                /// the current device.
                ///
                /// # Errors
                /// There are no particular errors other than IO / symlink resolution failures, etc.
                fn [<populate_ $by>](&mut self) -> $crate::Res<Option<&str>> {
                    for x in ls_symlinks(Path::new(concat!("/dev/disk/by-", stringify!($by))))? {
                        let (p, s) = x?;
                        if p != self.as_ref().fullname {
                            continue;
                        }
                        self.as_mut().$by = Some(s);
                        return Ok(self.as_ref().$by.as_deref());
                    }
                    Ok(None)
                }
            )+
        }
    }};
}

generate_populate_trait!(Populate diskseq path uuid partuuid label partlabel id);

impl Populate for BlockDevice {
    fn as_mut(&mut self) -> &mut BlockDevice {
        self
    }

    fn as_ref(&self) -> &BlockDevice {
        self
    }
}
