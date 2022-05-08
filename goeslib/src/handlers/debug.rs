use std::path::PathBuf;

use log::warn;

use crate::{emwin, lrit::LRIT};

use super::{Handler, HandlerError};
use std::io::Write;

/// Dumps LRIT headers to a file
pub struct DebugHandler {
    output_root: PathBuf,
}

impl DebugHandler {
    pub fn new() -> Self {
        DebugHandler {
            output_root: PathBuf::from("/tank/achin/tmp/goes_out3"),
        }
    }
}

impl Handler for DebugHandler {
    fn handle(&mut self, lrit: &LRIT) -> Result<(), HandlerError> {
        if let Some(annotation) = &lrit.headers.annotation {
            if let Ok(mut output_file) =
                std::fs::File::create(self.output_root.join(&annotation.text).with_extension("debug"))
            {
                writeln!(&mut output_file, "VCID: {}", lrit.vcid)?;
                writeln!(&mut output_file, "{:#?}", lrit.headers)?;

                // Is this a EMWIN text product?
                if lrit.vcid == 20 || lrit.vcid == 21 || lrit.vcid == 22 {
                    if annotation.text.starts_with("A_") || annotation.text.starts_with("Z_") {
                        if let Some(parsed_emwin) = emwin::ParsedEmwinName::parse(&annotation.text) {
                            writeln!(&mut output_file, "{:#?}", parsed_emwin)?;
                        }
                    }
                }
            }
        } else {
            warn!("missing annotation");
        }

        Ok(())
    }
}
