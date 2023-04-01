

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

    use crate::{consts::DEFAULT_PATH, index::read_index_file, error::CResult};


    // must be able to parse chace
    #[test]
    fn paserig() -> CResult<()> {

        let mut index_buff = {
            let mut buf = Vec::new();
            println!("{}", DEFAULT_PATH.to_string() + "/index");
            File::open(DEFAULT_PATH.to_string() + "/index")?.read_to_end(&mut buf)?;
            buf
        };

        let index = read_index_file(&mut index_buff)?;




        Ok(())
    }

}
