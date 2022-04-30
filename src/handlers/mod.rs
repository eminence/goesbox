use crate::lrit::LRIT;

mod debug;
mod image;
mod text;

pub use self::debug::*;
pub use self::image::*;
pub use self::text::*;

pub trait Handler {
    fn handle(&mut self, lrit: &LRIT);
}
