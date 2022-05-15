use std::{
    io::Write,
    path::{Path, PathBuf},
};

use log::info;

use crate::{emwin, lrit::LRIT};

use super::{Handler, HandlerError};

pub struct TextHandler {
    output_root: PathBuf,
}

impl TextHandler {
    pub fn new(root: impl AsRef<Path>) -> TextHandler {
        TextHandler {
            output_root: root.as_ref().to_path_buf(),
        }
    }
}

impl Handler for TextHandler {
    fn handle(&mut self, lrit: &LRIT) -> Result<(), HandlerError> {
        if lrit.headers.primary.filetype_code != 2 {
            return Err(HandlerError::Skipped);
        }
        // before trying to print this message, see if it's compressed by looking

        let compressed = if let Some(noaa) = &lrit.headers.noaa {
            noaa.noaa_compression != 0
        } else {
            false
        };

        if compressed {
            let mut cur = std::io::Cursor::new(&lrit.data);
            let mut archive = zip::read::ZipArchive::new(&mut cur)?;

            //info!("zip read: Ok(archive) {}", archive.len());
            for idx in 0..archive.len() {
                if let Ok(mut file) = archive.by_index(idx) {
                    //info!("Zip archive file {}", file.name());
                    let output_path = self.output_root.join(file.mangled_name());
                    let filename = file.mangled_name();
                    let filename = filename.to_string_lossy();
                    let mut output_file = std::fs::File::create(&output_path)?;
                    std::io::copy(&mut file, &mut output_file)?;

                    if lrit.vcid == 20 || lrit.vcid == 21 || lrit.vcid == 22 {
                        if filename.starts_with("A_") || filename.starts_with("Z_") {
                            if let Some(parsed_emwin) = emwin::ParsedEmwinName::parse(&filename) {
                                let latest_symlink = self
                                    .output_root
                                    .join(format!("latest-{}", parsed_emwin.legacy_filename));
                                if latest_symlink.exists() {
                                    std::fs::remove_file(&latest_symlink)?;
                                }
                                std::os::unix::fs::symlink(&output_path, latest_symlink)?;
                            }
                        }
                    }
                }
            }
        } else {
            // try to print data
            //let s = String::from_utf8_lossy(&self.bytes[offset as usize..]);
            if let Some(annotation) = &lrit.headers.annotation {
                let output_path = self.output_root.join(&annotation.text);
                if let Ok(mut output_file) = std::fs::File::create(&output_path) {
                    output_file.write_all(&lrit.data)?;
                }

                // Is this a EMWIN product?
                if lrit.vcid == 20 || lrit.vcid == 21 || lrit.vcid == 22 {
                    if annotation.text.starts_with("A_") || annotation.text.starts_with("Z_") {
                        if let Some(parsed_emwin) = emwin::ParsedEmwinName::parse(&annotation.text) {
                            let latest_symlink = self
                                .output_root
                                .join(format!("latest-{}", parsed_emwin.legacy_filename));
                            if latest_symlink.exists() {
                                std::fs::remove_file(&latest_symlink)?;
                            }
                            std::os::unix::fs::symlink(&output_path, latest_symlink)?;
                        }
                    }
                }
            }
            //info!("uncompressed string data: {}", s);
        }

        if let Some(ann) = &lrit.headers.annotation {
            info!("Wrote {}", ann.text);
        }
        Ok(())
    }
}
