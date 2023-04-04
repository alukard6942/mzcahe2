use endian_trait::Endian;

use crate::{consts::*, error::{CResult, Error}};

use core::slice;
use std::{io, fmt::{Display, write}};

use crate::utils::{read_struct_buff, Timestamp};
use flate2::read::GzDecoder;
use std::{
    io::Read,
    mem::size_of,
    ops::Shl,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, Endian)]
struct Header {
    version: u32,
    fetch_count: u32,
    last_fetched_time: Timestamp,
    last_modified_time: Timestamp,
    frecency: u32,
    expiration_time: u32,
    key_size: u32,
    flags: u32,
}

pub struct CacheFile {
    key: String,
    header: Header,
    body: Vec<u8>,
    hash: Vec<u8>,
}

impl CacheFile {
    fn url(&self) -> &str {

        let key = &self.key;

        let mut s = key.rsplit(",");

        &s.nth(0).unwrap()[1..]
    }
}


impl Display for CacheFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        let key = self.url();
        let size = self.body.len();
        let fetched = self.header.last_fetched_time;
        let modified = self.header.last_modified_time;
        let hash = &self.hash;

        write(f, format_args!("{:80} {:10} \t {} {}\n{:?}", key, size, fetched, modified, hash))
    }
}



// parses out the cache file, one big fc do not tuch
pub fn parse_cachefile(data: &Vec<u8>) -> CResult<CacheFile> {
    let size = data.len();
    if size <= size_of::<Header>() {
        return Err(Error::FileTooSmall);
    }

    let offset: usize = {
        let offset;
        if size < kMinMetadataRead {
            offset = 0;
        } else {
            offset = size - kMinMetadataRead;
        }
        offset / kAlignSize * kAlignSize
    };

    let mut mbuff_size = size - offset;
    let mut mbuff = &data[offset..offset + mbuff_size];

    // parse metadata
    let realoffset: usize = {
        let mut b = mbuff.get(mbuff_size - 4..).unwrap().clone().to_vec();
        b.reverse();
        let realoffset = unsafe { *(b.as_ptr() as *mut u32) } as usize;

        // this is actualy big
        assert!(realoffset < size, "offset of bounds");
        realoffset
    };

    // wtf is usedoffset how does one visualize it
    let mut used_offset = size - mbuff_size;
    if realoffset < used_offset {
        let missing = used_offset - realoffset;
        mbuff_size += missing;
        mbuff = &data[realoffset..realoffset + mbuff_size];
        used_offset = size - mbuff_size;
        assert_eq!(
            used_offset, realoffset,
            "its same by definition why all the logic and two variables?"
        )
    }

    let metaoffset = realoffset;
    let buff_offset = realoffset - used_offset;
    let meta_pos_offset = mbuff_size - 4;
    let hashes_offset = buff_offset + 4;
    let hash_count = metaoffset / kChunkSize + {
        if metaoffset % kChunkSize != 0 {
            1
        } else {
            0
        }
    };

    let hashes_len = hash_count * 2;
    let hdr_offset = hashes_offset + hashes_len;
    let key_offset = hdr_offset + size_of::<Header>();
    let header_buf = mbuff.get(hdr_offset..key_offset).unwrap();

    let meta_hdr: Header = read_struct_buff(header_buf).unwrap();
    let key = mbuff
        .get(key_offset..key_offset + meta_hdr.key_size as usize + 1)
        .unwrap().to_vec();

    let elements_offset = meta_hdr.key_size as usize + key_offset + 1;

    assert!(
        elements_offset < meta_pos_offset,
        "error: elements offset {} exceeds {}",
        elements_offset,
        meta_pos_offset
    );
    let element_buf_size = meta_pos_offset - elements_offset;

    // parse elemets
    let mut elements = Vec::new();
    let ebuff = mbuff
        .get(elements_offset..elements_offset + element_buf_size)
        .unwrap();

    let mut start = 0;
    for i in 0..element_buf_size {
        // nullterminated keys
        if ebuff[i] == 0 {
            let key = ebuff[start..i].to_vec();
            elements.push(key);
            start = i + 1;
        }
    }
    for e in elements {
        println!("{}", String::from_utf8_lossy(&e));
    }

    //parse hashes
    let hash_buf = mbuff
        .get(hashes_offset..hashes_offset + hashes_len)
        .unwrap();

    let hash: Vec<u8> = {
       let mut buff = Vec::<u8>::with_capacity(hash_count *2);
       let hashes = unsafe { slice::from_raw_parts_mut(buff.as_mut_ptr() as *mut u16, hash_count) };
       for i in 0..hash_count {
           let hash: u16 = {
               let high = hash_buf[i * 2] as u16;
               let low = hash_buf[i * 2 + 1] as u16;
               low + high.shl(4)
           };
           hashes[i] = hash;
       }
       unsafe {buff.set_len(hash_count *2)}
       buff
    };

    println!("{:?}", hash);

    let data_buff = &data[0..realoffset];

    let chunks = parse_chunks(data_buff)?;

    Ok(CacheFile {
        key: String::from_utf8(key)?,
        header: meta_hdr,
        body: chunks,
        hash,
    })
}

fn parse_chunks(data_buff: &[u8]) -> io::Result<Vec<u8>> {
    let size = data_buff.len();
    let mut chunks = Vec::new();
    for pos in (0..size).step_by(kChunkSize) {
        let chunk_size = std::cmp::min(kChunkSize, size - pos);
        let chunk_buf = match data_buff.get(pos..pos + chunk_size) {
            Some(it) => it,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::OutOfMemory,
                    "this shuld matematicly not have happend",
                ))
            }
        };
        if chunk_buf[0..2] == b"\x1F\x8B".to_vec() {
            let mut plain_buf = decopres_chunk(chunk_buf)?;
            chunks.append(&mut plain_buf);
        } else {
            chunks.extend_from_slice(chunk_buf);
        }
    }

    Ok(chunks)
}

fn decopres_chunk(chunk: &[u8]) -> io::Result<Vec<u8>> {
    let mut buff = Vec::new();
    GzDecoder::new(chunk).read_to_end(&mut buff)?;
    Ok(buff)
}

#[cfg(test)]
mod tests {

    use std::{time::SystemTime, fs::{self, File}};

    use super::*;

    use colored::Colorize;

    // we can safaly asume cache file for our control data will be from the last year
    fn header_validity_test(h: Header) -> bool {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let year: u64 = 60 * 60 * 24 * 365;

        let modified = h.last_modified_time.0 as u64;
        let fetched = h.last_fetched_time.0 as u64;

        // is not form last year
        if (modified > now || modified < (now - year)) || (fetched > now || fetched < (now - year))
        {
            // is not a specila value
            if modified > 10 && fetched > 10 {
                println!("invalid time: {:#10x} {:#10x}", modified, fetched);
                return false;
            }
        }
        return true;
    }

    #[test]
    fn entryes_test() {
        let mut n = 0.;
        let mut hits = 0.;

        for file in fs::read_dir("./cache2/entries").unwrap() {
            let filename = file.unwrap().path();

            let mut f = File::open(filename.clone()).unwrap();
            let data = {
                let mut buff = Vec::new();
                f.read_to_end(&mut buff).unwrap();
                buff
            };

            let cache = parse_cachefile(&data).unwrap();

            let check = match header_validity_test(cache.header) {
                true => {
                    hits += 1.;
                    "+".green()
                }
                false => "-".red(),
            };

            println!("[{}] {}", check, filename.to_str().unwrap());

            n += 1.;
        }

        println!("{}/{} {}% succes", hits, n, 100. * hits / n);

        assert!(hits / n >= 0.99);
    }
}
