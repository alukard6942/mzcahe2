

pub type CResult<T> = Result<T, Error>;



#[derive(Debug)]
pub enum Error {
    None, 
    Io(std::io::Error),
    MissingHeader,
}

impl From<std::io::Error> for Error {
    fn from(t: std::io::Error) -> Self {
        Error::Io(t)
    }
}
