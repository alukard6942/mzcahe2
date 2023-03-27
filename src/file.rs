use endian_trait::Endian;

use crate::utils::Timestamp;

use crate::consts::*;

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

#[cfg(test)]
mod tests {
    use crate::utils::{read_struct, Timestamp, read_struct_buff};
    use std::{
        fs::{self, File},
        io::{Read, Seek, SeekFrom},
        mem::{size_of, MaybeUninit},
        time::SystemTime, ops::Shl,
    };

    use super::*;

    use colored::Colorize;

    fn parse_metadata(f: &mut File, metaoffset: u64, bufferoffset: u64) -> Option<Header> {
        None
    }

    fn readmetadata(f: &mut File) -> Option<Header> {
        let size = f.metadata().unwrap().len() as usize;


        let mut data = Vec::new();
        f.read_to_end(&mut data);

        if size <= 0 {
            return None;
        }
        if size <= size_of::<Header>() { return None; }

        let mut offset: usize = 0;
        if size < kMinMetadataRead {
            offset = 0;
        } else {
            offset = size - kMinMetadataRead;
        }
        offset = offset / kAlignSize * kAlignSize;

        let mut mbuff_size = size - offset;
        let mut mbuff = &data[offset .. offset + mbuff_size];

        // parse metadata
        let realoffset: usize = {
            let mut b = mbuff.get(mbuff_size - 4 .. ).unwrap().clone().to_vec();
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
            mbuff = &data[realoffset .. realoffset + mbuff_size];
            used_offset = size- mbuff_size;
            assert_eq!(used_offset, realoffset, "its same by definition why all the logic and two variables?")
        }

        let metaoffset = realoffset; 
        let buff_offset = realoffset - used_offset;
        let meta_pos_offset = mbuff_size - 4;
        let hashes_offset = buff_offset + 4;
        let hash_count = metaoffset / kChunkSize + !!(metaoffset % kChunkSize);
        let hashes_len = hash_count * 2;
        let hdr_offset = hashes_offset + hashes_len;
        let key_offset = hdr_offset + size_of::<Header>();
        let header_buf = mbuff.get(hdr_offset .. key_offset).unwrap();
        
		let meta_hdr: Header = read_struct_buff(header_buf).unwrap();
		let key = mbuff.get(key_offset .. key_offset + meta_hdr.key_size as usize + 1).unwrap();
		let elements_offset = meta_hdr.key_size as usize + key_offset  + 1;

        assert!(elements_offset > meta_pos_offset, "error: elements offset {} exceeds {}", elements_offset, meta_pos_offset );
		let element_buf_size = meta_pos_offset - elements_offset;

        // parse elemets
	    let mut elements = Vec::new();
	    let mut key;
	    let ebuff = mbuff.get(elements_offset .. elements_offset + element_buf_size).unwrap();

		let mut start = 0;
		for i in 0 .. element_buf_size {
		    // nullterminated keys
			if ebuff[i] == 0 {
				key = ebuff[start .. i].to_vec();
				elements.push(key);
				start = i + 1;
            }
        }

        //parse hashes
		let hash_buf = mbuff.get(hashes_offset .. hashes_offset+hashes_len).unwrap();

		let mut hash_codes = Vec::new();

		
		for i in 0 .. hash_count {
		    let hash: u16 = {
		        let high = hash_buf[i*2] as u16;
		        let low = hash_buf[i*2 +1] as u16;
		        low + high.shl(4)
		    };
			hash_codes.push(hash);
        }

        let data_buf = &data[0 .. realoffset]; 

        let mut chunks = Vec::new();

		let size = realoffset;
		for pos in (0 .. size).step_by(kChunkSize) {
			let chunk_size = std::cmp::min(kChunkSize, size - pos);
			let mut chunk_buf = data_buf.get(pos .. pos+chunk_size).unwrap().to_vec();
			if chunk_buf[0 .. 2] == b"\x1F\x8B".to_vec() {

				let plain_buf = decopres_chunk(&mut chunk_buf);

				chunks.push(plain_buf);
			}
			else {
				chunks.push(chunk_buf);
			}
		}

        println!("{:?}", chunks);







        None
    }


    fn decopres_chunk(chunk: &mut [u8]) -> Vec<u8> {

        let decompresser = flate2::Decompress::new(false);
        let mut buff = Vec::new();

        decompresser.decompress_vec(chunk, &mut buff, flate2::FlushDecompress::None);


        println!("{:?}", buff);

        buff
    }



	fn parse_elements(buf: Vec<u8>, buf_size: usize) -> Vec<u8>{
	    let mut elements = Vec::new();

		let i = 0;
		let start = 0;
		while i < buf_size {
			if buf[i] == 0 {
				let mut key = buf[start .. i].to_vec();
				elements.append(&mut key);
				start = i + 1;
            }
			i+=1;
        }

        elements
    }


    fn header_test(f: &mut File, offset: u64) -> Option<Header> {
        f.seek(SeekFrom::Start(offset)).unwrap();

        let h = match read_struct::<Header>(f) {
            Some(it) => it,
            None => return None,
        };

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let year: u64 = 12 * 60 * 60 * 24 * 365;

        let modified = h.last_modified_time.0 as u64;
        let fetched = h.last_fetched_time.0 as u64;

        if (modified > now || modified < (now - year)) || (fetched > now || fetched < (now - year))
        {
            if h.last_fetched_time.0 > 10 || h.last_modified_time.0 > 10 {
                println!("invalid time: {:#10x}", h.last_fetched_time.0);
                return None;
            }
        }

        return Some(h);
    }

    #[test]
    fn entryes_test() {
        let mut n = 0.;
        let mut hits = 0.;

        for file in fs::read_dir("./cache2/entries").unwrap() {
            let filename = file.unwrap().path();

            let mut f = File::open(filename.clone()).unwrap();

            let test = readmetadata(&mut f);

            let check = match test {
                Some(_) => {
                    hits += 1.;
                    "+".green()
                }
                None => "-".red(),
            };

            // println!("[{}] {}", check, filename.to_str().unwrap());

            n += 1.;
        }

        println!("{}/{} {}% succes", hits, n, 100. * hits / n);

        assert!(hits / n >= 99.);
    }

    fn dates_test(f: &mut File) -> Option<u64> {
        let mut buffer: [u8; 4] = unsafe { MaybeUninit::uninit().assume_init() };

        let mut i = 0;

        while f.read_exact(&mut buffer).is_ok() {
            buffer.reverse();
            let time: Timestamp = Timestamp(unsafe { *(buffer.as_ptr() as *mut u32) });

            i += 1;

            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            if (time.0 as u64) > now {
                continue;
            }
            let month: u64 = (60 * 60 * 24 * 365);
            if (time.0 as u64) < (now - month) {
                continue;
            }

            return Some((i - 1) * 4);
        }

        None
    }

    #[ignore]
    #[test]
    fn look_for_dates() {
        let mut n = 0.;
        let mut hits = 0.;
        for file in fs::read_dir("./cache2/entries").unwrap() {
            let filename = file.unwrap().path();

            let mut f = File::open(filename.clone()).unwrap();

            let test = dates_test(&mut f);

            let check = match test {
                Some(_) => {
                    hits += 1.;
                    "+".green()
                }
                None => "-".red(),
            };

            println!(
                "[{}] {} {}",
                check,
                filename.to_str().unwrap(),
                match test {
                    Some(i) => i.to_string() + "/" + &f.metadata().unwrap().len().to_string(),
                    None => "".to_string(),
                }
            );

            n += 1.;
        }

        println!("{}/{} {}% succes", hits, n, 100. * hits / n);

        assert!(hits / n >= 99.)
    }
}
