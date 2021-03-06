use std::error::Error;

use crate::lrit::LRIT;

mod dcs;
mod debug;
mod image;
mod text;

pub use self::dcs::*;
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
    /// A handler is missing a header
    ///
    /// This is unexpected, and is either a bug in this code or a corrupt packet.
    MissingHeader(&'static str),

    /// Some parsing error
    Parse(&'static str),

    Other(Box<dyn Error>),
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

impl From<::image::ImageError> for HandlerError {
    fn from(e: ::image::ImageError) -> Self {
        match e {
            ::image::ImageError::IoError(io) => Self::Io(io),
            other => Self::Other(Box::new(other)),
        }
    }
}

pub trait Handler {
    fn handle(&mut self, lrit: &LRIT) -> Result<(), HandlerError>;
}
