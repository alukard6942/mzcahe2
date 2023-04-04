use std::string::FromUtf8Error;



pub type CResult<T> = Result<T, Error>;



#[derive(Debug)]
pub enum Error {
    None, 
    Io(std::io::Error),
    MissingHeader,
    FileTooSmall,
    Utf(FromUtf8Error),
}

impl From<std::io::Error> for Error {
    fn from(t: std::io::Error) -> Self {
        Error::Io(t)
    }
}


type T = FromUtf8Error;
impl From<T> for Error {
    fn from(t: T) -> Self {
        Error::Utf(t)
    }
}
