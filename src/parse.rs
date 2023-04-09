
#[cfg(test)]
mod tests {
    use std::{fs::{File, self}, io::Read};

    use crate::{consts::DEFAULT_PATH, index::read_index_file, error::CResult, file::parse_cachefile};


    // must be able to parse chace
    #[test]
    fn paserig() -> CResult<()> {

        let path = "./cache2";

        let mut index_buff = {
            let mut buf = Vec::new();
            File::open(path.to_string() + "/index")?.read_to_end(&mut buf)?;
            buf
        };

        let index = read_index_file(&mut index_buff)?;

        // let mut doomed = Vec::new();
        let mut entries = Vec::new();

        for f in fs::read_dir(DEFAULT_PATH.to_string() + "/entries")? {
            let mut file = File::open( f?.path() )?;

            let mut data = Vec::new();
            file.read_to_end(&mut data)?;

            let cache = match parse_cachefile(&data) {
                Ok(it) => it,
                Err(_err) => continue,
            };

            println!("{}", cache);
            entries.push(cache);
        }
        // assert!(false);
        Ok(())
    }

}
