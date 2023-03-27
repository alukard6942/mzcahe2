use endian_trait::Endian;

use crate::utils::Timestamp;

#[repr(C)]
#[derive(Debug, Copy, Clone, Endian)]
struct Header {
    version: u32,
    // POSIX timestampt in UTC
    last_modification: Timestamp,
    is_dirty: u32,
    kb_writen: u32,
}

#[derive(Debug, Copy, Clone)]
struct Hash([u8; 20]);

impl Endian for Hash {
    fn to_be(self) -> Self {
        self
    }
    fn to_le(self) -> Self {
        self
    }
    fn from_be(self) -> Self {
        self
    }
    fn from_le(self) -> Self {
        self
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, Endian)]
struct Record {
    hash: Hash,
    // the fuck is frecency?
    frecency: u32,
    origin_attr_hash: u64,
    on_start_time: u16,
    on_stop_time: u16,
    content_type: u8,

    /*
     *    1000 0000 0000 0000 0000 0000 0000 0000 : initialized
     *    0100 0000 0000 0000 0000 0000 0000 0000 : anonymous
     *    0010 0000 0000 0000 0000 0000 0000 0000 : removed
     *    0001 0000 0000 0000 0000 0000 0000 0000 : dirty
     *    0000 1000 0000 0000 0000 0000 0000 0000 : fresh
     *    0000 0100 0000 0000 0000 0000 0000 0000 : pinned
     *    0000 0010 0000 0000 0000 0000 0000 0000 : has cached alt data
     *    0000 0001 0000 0000 0000 0000 0000 0000 : reserved
     *    0000 0000 1111 1111 1111 1111 1111 1111 : file size (in kB)
     */
    flags: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::File, mem::size_of};

    use crate::utils::read_struct;

    use super::*;

    #[test]
    fn sizetest() {
        assert_eq!(size_of::<Header>(), 16);
        assert_eq!(size_of::<Record>(), 41);
    }

    #[test]
    fn header_test() {
        let path = "cache2/index";
        let mut f = File::open(path.to_string()).unwrap();

        let h = read_struct::<Header>(&mut f).unwrap();

        let t = format!("{}", h.last_modification);

        assert_eq!(h.version, 10);
        assert_eq!(t, "2023-02-27 19:45:18.000000000");
        assert_eq!(h.is_dirty, 1);
    }

    #[test]
    fn record_test() {
        let path = "cache2/index";
        let mut f = File::open(path.to_string()).unwrap();

        let _ = read_struct::<Header>(&mut f).unwrap();

        while let Some(record) = read_struct::<Record>(&mut f) {
            print!("{:#?}", record);
            assert!(record.on_start_time <= record.on_stop_time);
        }
    }
}
