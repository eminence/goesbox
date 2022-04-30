use std::path::PathBuf;

use log::warn;

use crate::lrit::LRIT;

use super::Handler;
use std::io::Write;

/// Dumps LRIT headers to a file
pub struct DebugHandler {
    output_root: PathBuf,
}

impl DebugHandler {
    pub fn new() -> Self {
        DebugHandler {
            output_root: PathBuf::from("/tank/achin/tmp/goes_out2"),
        }
    }
}

impl Handler for DebugHandler {
    fn handle(&mut self, lrit: &LRIT) {
        if let Some(annotation) = &lrit.headers.annotation {
            if let Ok(mut output_file) = std::fs::File::create(
                self.output_root
                    .join(&annotation.text)
                    .with_extension("debug"),
            ) {
                writeln!(&mut output_file, "{:#?}", lrit.headers);
            }
        } else {
            warn!("missing annotation");
        }
    }
}
