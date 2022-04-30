use crate::lrit::LRIT;

mod debug;
mod image;
mod text;

pub use self::debug::*;
pub use self::image::*;
pub use self::text::*;

#[derive(Debug)]
pub enum HandlerError {
    /// The handler was skipped because the LRIT packet wasn't the right type
    ///
    /// This isn't an Error per, and can be ignored most of the time.
    Skipped,
    /// Some IO error (generally from writing data to disk)
    Io(std::io::Error),
    /// A ZIP error
    Zip(zip::result::ZipError),
}

impl From<std::io::Error> for HandlerError {
    fn from(io: std::io::Error) -> Self {
        Self::Io(io)
    }
}

impl From<zip::result::ZipError> for HandlerError {
    fn from(zip: zip::result::ZipError) -> Self {
        Self::Zip(zip)
    }
}

pub trait Handler {
    fn handle(&mut self, lrit: &LRIT) -> Result<(), HandlerError>;
}
