use std::{collections::HashMap, io::Write, path::PathBuf};

use log::info;

use crate::lrit::LRIT;

use super::Handler;

pub struct ImageHandler {
    output_root: PathBuf,

    /// holds the last few image segments
    ///
    /// While the image segments will arrive out-of-order, in theory the image segments should not
    /// be interleaved with segments from other images.  In practice, I've seen this a few times,
    /// and so this cache will keep track of segments for the 3 most recent images (indexed by a
    /// u16 image identifier)
    segments: lru_cache::LruCache<u16, Vec<LRIT>>, //files: Vec<_>
}

impl ImageHandler {
    pub fn new() -> ImageHandler {
        ImageHandler {
            output_root: PathBuf::from("/tank/achin/tmp/goes_out2"),
            segments: lru_cache::LruCache::new(3),
        }
    }
}

impl Handler for ImageHandler {
    fn handle(&mut self, lrit: &LRIT) {
        if lrit.headers.primary.filetype_code != 0 {
            return;
        }

        // these headers are mandatory for image data:
        let ihs = lrit
            .headers
            .img_strucutre
            .as_ref()
            .expect("image structure header");
        let annotation = lrit.headers.annotation.as_ref().expect("Annotation header");

        // images
        //info!("image Headers: {:?}", headers);

        let segmented = if let Some(text) = &lrit.headers.text {
            let mut map = HashMap::new();
            for pair in text.text.split(';') {
                let mut s = pair.splitn(2, '=');
                let key = s.next().expect("splitn").trim().to_owned();
                let val = s.next().expect("splitn").trim().to_owned();
                map.insert(key, val);
            }
            match map.get("Segmented") {
                Some(s) if s == "yes" => true,
                _ => false,
            }
        } else {
            false
        };

        //info!("segmented: {}", segmented);
        if !segmented {
            // write out image immeditally
            //info!("headers: {:?}", lrit.headers);
            assert_eq!(
                ihs.bits_per_pixel, 8,
                "Found non grayscale image: {:?}",
                ihs
            );

            if let Some(noaa) = &lrit.headers.noaa {
                if noaa.noaa_compression == 5 {
                    // gif image can be written directly to disk
                    if let Ok(mut file) = std::fs::File::create(
                        self.output_root
                            .join(&annotation.text)
                            .with_extension("gif"),
                    ) {
                        file.write_all(&lrit.data).expect("write_all");
                        return;
                    }
                }
            }

            // sometimes the data seems to be not quite long enough to contain the entire image, so
            // extend it if necessary
            let mut data = lrit.data.clone();
            data.resize(ihs.num_columns as usize * ihs.num_lines as usize, 0);
            // save raw pixel data
            let img: image::GrayImage =
                image::GrayImage::from_raw(ihs.num_columns as u32, ihs.num_lines as u32, data)
                    .unwrap_or_else(|| {
                        panic!(
                            "Failed to create img for {}:\n{:?}",
                            &annotation.text, lrit.headers
                        );
                    });
            let out_name = self
                .output_root
                .join(&annotation.text)
                .with_extension("jpg");
            info!("{}", out_name.display());

            img.save(out_name);

            return;
        }

        let seg = lrit
            .headers
            .img_segment
            .as_ref()
            .expect("image segment header");

        // have we seen segments with this image id before?
        if let Some(mut seg_vec) = self.segments.remove(&seg.image_id) {
            seg_vec.push(lrit.clone());

            if seg_vec.len() == seg.max_segment as usize {
                self.write_image_from_segments(seg_vec);
            } else {
                // put the list back in the LRU cache
                self.segments.insert(seg.image_id, seg_vec);
            }
        } else {
            // if adding this entry would evict an old entry... we don't really care
            self.segments.insert(seg.image_id, vec![lrit.clone()]);
        }
    }
}

impl ImageHandler {
    fn write_image_from_segments(&self, mut segments: Vec<LRIT>) {
        if segments.len() == 0 {
            return;
        }

        // these 3 headers are required for image data, but might be missing nonetheless
        // general structure info will be the same in all LRIT files, so just take the first
        let ihs = segments
            .first()
            .unwrap()
            .headers
            .img_strucutre
            .as_ref()
            .expect("img_structure header")
            .clone();
        assert_eq!(
            ihs.bits_per_pixel, 8,
            "Found non grayscale image: {:?}",
            ihs
        );
        let seg = segments
            .first()
            .unwrap()
            .headers
            .img_segment
            .as_ref()
            .expect("img_segment header")
            .clone();
        let ann = segments
            .first()
            .unwrap()
            .headers
            .annotation
            .as_ref()
            .expect("annotation header")
            .clone();

        let num_segments = segments.len();

        //assert_eq!(ihs.num_lines * seg.max_segment, seg.max_column, "segment max_col doesn't match num_lines*max_segment");
        assert!(
            self.segments.len() <= seg.max_segment as usize,
            "too many segments: {} <= {}",
            self.segments.len(),
            seg.max_segment
        );

        // list of segments, in order (with possible gaps)
        let mut new_segments = Vec::with_capacity(seg.max_segment as usize);
        new_segments.resize(seg.max_segment as usize, None);

        for lrit in segments.drain(..) {
            let seg = lrit.headers.img_segment.as_ref().unwrap();
            let id = seg.segment_seq;
            //info!("{:?}", seg);
            new_segments[id as usize] = Some(lrit);
        }

        let segments = new_segments;

        let mut pixels: Vec<u8> =
            Vec::with_capacity(ihs.num_columns as usize * seg.max_row as usize);
        pixels.resize(seg.max_row as usize * seg.max_column as usize, 0u8);

        for lrit in segments {
            if let Some(lrit) = lrit {
                let seg = lrit
                    .headers
                    .img_segment
                    .as_ref()
                    .expect("img_segment header");
                let ihs = lrit
                    .headers
                    .img_strucutre
                    .as_ref()
                    .expect("img_structure header");

                let start = seg.max_column as usize * seg.start_line as usize;
                //let end = start + (ihs.num_lines  as usize * seg.max_column as usize);
                let end = start + lrit.data.len();
                &pixels[start..end].copy_from_slice(&lrit.data);
                //pixels.extend(lrit.data);
                //
                //} else {
                //pixels.extend(std::iter::repeat(0u8).take(ihs.num_columns as usize * ihs.num_lines as usize));
            }
        }

        let pixlen = pixels.len();
        match image::GrayImage::from_raw(ihs.num_columns as u32, seg.max_row as u32, pixels) {
            Some(img) => {
                let out_name = self.output_root.join(&ann.text).with_extension("jpg");

                info!(
                    "segmented ({} of {}), {}",
                    num_segments,
                    seg.max_segment,
                    out_name.display()
                );
                img.save(out_name);
            }
            None => {
                /*
                ImageStructureRecord { header_type: 1, header_record_lenth: 9, bits_per_pixel: 8, num_columns: 5424, num_lines: 339, compression: 1 }
                ImageSegmentIdentificationRecord { header_type: 128, header_record_lenth: 17, image_id: 58004, segment_seq: 1, start_col: 0, start_line: 339, max_segment: 16, max_column: 5424, max_row: 5424 }
                                   */
                info!(
                    "failed to create image, pixlen={} {:?} {:?}",
                    pixlen, ihs, seg
                );
            }
        }
    }
}
