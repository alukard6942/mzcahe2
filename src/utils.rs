#[repr(C)]
#[derive(Copy, Clone, Endian)]
pub struct Timestamp(pub u32);

use chrono::{DateTime, Utc};
use endian_trait::Endian;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::Read;
use std::mem::{size_of, MaybeUninit};
use std::slice;
use std::time::{Duration, UNIX_EPOCH};

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Creates a new SystemTime from the specified number of whole seconds
        let d = UNIX_EPOCH + Duration::from_secs(self.0 as u64);
        // Create DateTime from SystemTime
        let datetime = DateTime::<Utc>::from(d);
        // Formats the combined date and time with the specified format string.
        let timestamp_str = datetime.format("%Y-%m-%dT%H:%M:%S").to_string();
        write!(f, "{}", timestamp_str)
    }
}
impl Debug for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

pub fn read_struct_buff<Struct: Endian>(b: &[u8]) -> Option<Struct> {
    let size = size_of::<Struct>();
    if b.len() < size {return None};
    Some(unsafe{
        let mut header = MaybeUninit::<Struct>::uninit();
        // btw how does this work as slice gets deleted shouldnt be the buffer be deleted as well
        // what am i missing
        let slice = 
            slice::from_raw_parts_mut(&mut header as *mut _ as *mut u8, size);
        slice.clone_from_slice(b.get_unchecked(0 .. size)); 
        header.assume_init()
    }.from_be())
}

// this may allow the implementation to use streams and spare some memory
pub fn read_struct<Struct>(f: &mut impl Read) -> Option<Struct> where Struct: Endian
{
    Some( unsafe {
        let mut header = MaybeUninit::<Struct>::uninit();
        let config_slice =
            slice::from_raw_parts_mut(&mut header as *mut _ as *mut u8, size_of::<Struct>());
        // `read_exact()` comes from `Read` impl for `&[u8]`
        match f.read_exact(config_slice) {
            Ok(it) => it,
            Err(_) => return None,
        };
        header.assume_init()
    } .from_be(),)
}
