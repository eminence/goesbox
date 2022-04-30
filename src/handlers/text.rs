use std::{io::Write, path::PathBuf};

use log::{info, warn};

use crate::lrit::LRIT;

use super::Handler;

pub struct TextHandler {
    output_root: PathBuf,
}

impl TextHandler {
    pub fn new() -> TextHandler {
        TextHandler {
            output_root: PathBuf::from("/tank/achin/tmp/goes_out2"), // hell yes hardcoded output paths
        }
    }
}

impl Handler for TextHandler {
    fn handle(&mut self, lrit: &LRIT) {
        if lrit.headers.primary.filetype_code != 2 {
            return;
        }
        // before trying to print this message, see if it's compressed by looking

        let compressed = if let Some(noaa) = &lrit.headers.noaa {
            noaa.noaa_compression != 0
        } else {
            false
        };

        if compressed {
            let mut cur = std::io::Cursor::new(&lrit.data);
            let result = zip::read::ZipArchive::new(&mut cur);
            match result {
                Ok(mut archive) => {
                    //info!("zip read: Ok(archive) {}", archive.len());
                    for idx in 0..archive.len() {
                        if let Ok(mut file) = archive.by_index(idx) {
                            //info!("Zip archive file {}", file.name());
                            if let Ok(mut output_file) =
                                std::fs::File::create(self.output_root.join(file.sanitized_name()))
                            {
                                std::io::copy(&mut file, &mut output_file);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("zip read: err {}", e);
                }
            }
        } else {
            // try to print data
            //let s = String::from_utf8_lossy(&self.bytes[offset as usize..]);
            if let Some(annotation) = &lrit.headers.annotation {
                if let Ok(mut output_file) =
                    std::fs::File::create(self.output_root.join(&annotation.text))
                {
                    output_file.write_all(&lrit.data);
                }
            }
            //info!("uncompressed string data: {}", s);
        }

        if let Some(ann) = &lrit.headers.annotation {
            info!("Wrote {}", ann.text);
        }
    }
}
