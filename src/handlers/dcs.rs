//! Parser for HRIT DCS ("Data Collection System") files
//!
//! Reference: HRIT_DCS_File_Format_Rev1.pdf
use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use log::{debug, info, warn};

use crate::{crc, handlers::HandlerError};

use super::Handler;

pub struct DcsHandler {}

impl DcsHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Handler for DcsHandler {
    fn handle(&mut self, lrit: &crate::lrit::LRIT) -> Result<(), HandlerError> {
        if lrit.headers.primary.filetype_code != 130 {
            return Err(super::HandlerError::Skipped);
        }

        let noaa = if let Some(noaa) = &lrit.headers.noaa {
            noaa
        } else {
            warn!("Missing NOAA header from DSC packet");
            return Err(HandlerError::MissingHeader("NOAA"));
        };

        if noaa.product_id != 8 {
            return Err(HandlerError::Skipped);
        }

        info!("DSC packet has {} bytes of data", lrit.data.len());

        let header = DcsHeader::parse(&lrit.data[..])?;
        if header.payload_type != "DCSH" {
            warn!("Expected DCSH payload type, got {:?}", header.payload_type);
            return Err(HandlerError::Parse("Expected DCSH payload type"));
        }
        info!("{:?}", header);

        assert_eq!(header.payload_len as usize, lrit.data.len());

        let blocks = DcsBlock::parse(&lrit.data[64..])?;
        info!("Found {} blocks", blocks.len());
        for block in blocks {
            // info!("{:?}", block);
        }

        Ok(())
    }
}

/// The header of a DCS packet (64 bytes)
#[derive(Debug)]
struct DcsHeader {
    name: String,
    /// Entire size of the Dcs packet (including this header)
    payload_len: u64,
    payload_source: String,
    payload_type: String,
    /// The received CRC for the header fields
    header_crc: u32,
    /// The CRC for the entire file (all header bytes and all data bytes)
    file_crc: u32,
}

impl DcsHeader {
    pub fn parse(data: &[u8]) -> Result<Self, HandlerError> {
        let mut cur = std::io::Cursor::new(data);

        // The DCS file header is 64 bytes

        // 32 bites contain the DCS file name (and trailing spaces)
        let mut name_buf = [b' '; 32];
        cur.read_exact(&mut name_buf)?;

        // 8 bytes contain length of payload, ascii encoded
        let mut payload_len_buf = [b'0'; 8];
        cur.read_exact(&mut payload_len_buf)?;

        // 4 bytes contain source of payload
        let mut source_buf = [b' '; 4];
        cur.read_exact(&mut source_buf)?;

        // 4 bytes contain type of payload
        let mut payload_type_buf = [b' '; 4];
        cur.read_exact(&mut payload_type_buf)?;

        // 12 bytes reserved for future use
        let mut reserved_buf = [0; 12];
        cur.read_exact(&mut reserved_buf)?;

        // 4 bytes contain header CRC, big endian
        let header_crc = cur.read_u32::<LittleEndian>()?;

        let computed_header_crc = crc::calc_crc32(&data[..60]);
        if computed_header_crc != header_crc {
            warn!(
                "Header CRC mismatch: {:x} != {:x}",
                computed_header_crc, header_crc
            );
        }

        let computed_file_crc = crc::calc_crc32(&data[..data.len() - 4]);

        // 4 bytes contain file CRC, which is stored as the last 4 bytes of the entire payload (big endian)
        cur.seek(SeekFrom::End(-4))?;
        let file_crc = cur.read_u32::<LittleEndian>()?;

        if computed_file_crc != file_crc {
            warn!(
                "File CRC mismatch: {:x} != {:x}",
                computed_file_crc, file_crc
            );
        }

        let name = String::from_utf8_lossy(&name_buf).trim().to_string();
        let payload_len = String::from_utf8_lossy(&payload_len_buf)
            .trim()
            .to_string()
            .parse()
            .map_err(|e| HandlerError::Parse("Failed to parse payload len in DCS header"))?;
        let payload_source = String::from_utf8_lossy(&source_buf).trim().to_string();
        let payload_type = String::from_utf8_lossy(&payload_type_buf)
            .trim()
            .to_string();

        Ok(Self {
            name,
            payload_len,
            payload_source,
            payload_type,
            header_crc,
            file_crc,
        })
    }
}

#[derive(Debug)]
enum DcsPlatform {
    CS1 = 0,
    CS2 = 1,
}

#[derive(Debug)]
enum DcsSpacescraft {
    Unknown = 0,
    GoesEast,
    GoesWest,
    GoesCentral,
    GoesTest,
    Reserved,
}

#[derive(Debug)]
enum DcsSource {
    /// NOAA WCDA E/W Prime -- Wallops Island, VA
    UP,
    /// NOAA WCDA E/W Backup – Wallops Island, VA
    UB,
    /// NOAA NSOF E/W Prime – Suitland, MD
    NP,
    /// NOAA NSOF E/W Backup – Suitland, MD
    NB,
    /// USGS EDDN East – EROS, Sioux Falls, SD
    XE,
    /// USGS EDDN West – EROS, Sioux Falls, SD
    XW,
    /// USACE MVR East – Rock Island, IL
    RE,
    /// USACE MVR West – Rock Island, IL
    RW,
    /// NIFC West Unit 1 – Boise, ID
    D1,
    /// NIFC West Unit 2 – Boise, ID
    D2,
    /// USACE LRD East – Cincinnati, OH
    LE,
    /// SFWMD East – West Palm Beach, FL
    SF,
    /// USACE NOW – Omaha, NE
    OW,
    Unknown([u8; 2]),
}

/// The main payload of a DCS file
///
/// After the 64 byte header, there will be a variable number of DcsBlock structs
#[derive(Debug)]
struct DcsBlock {
    block_id: u8,   // 3.2.1
    block_len: u16, // 3.2.2
    sequence: u32,  // 3.3.1 table

    // 3.3.1.1 message flags/baud
    // parity errors used to define 0-ASCII, 1-Pseudo-Binary
    baud_rate: u16,
    platform: DcsPlatform,
    /// Message received with parity errors
    parity_errors: bool,
    /// Not EOT received with message
    missing_eot: bool,
    // msg_flag_b6: bool,
    // msg_flag_b7: bool,

    // parse arm 3.3.1.2
    addr_corrected: bool,

    /// A bad address (not correctable)
    bad_addr: bool,
    invalid_addr: bool,
    incomplete_pdt: bool,

    /// Timing error (outsie window)
    timing_error: bool,
    unexpected_message: bool,
    wrong_channel: bool,
    // arm_flag_b7: bool,
    ///The BCH correction of the received Platform Address
    ///
    /// If the address is received without errors or is uncorrectable, this field will
    /// match the Received Address field.
    corrected_addr: u32,

    /// The time when the signal energy was first detected
    carrier_start: [u8; 7],

    /// The time when the signal energy was no logner detectable
    carrier_end: [u8; 7],

    /// Received message Signal strength in dBm
    signal_strength: f32,

    /// Frequency offset from the channel center of the received message
    freq_offset: f32,

    /// Phase noise in degrees RMS of the received mesage
    phase_noise: f32,
    // phase_mod_quality: String,
    good_phase: f32,

    space_platform: DcsSpacescraft,
    channel_number: u16,

    source_platform: DcsSource,

    data: Vec<u8>,
}

impl DcsBlock {
    /// Parse some data into a DcsBlock
    ///
    /// The data provided here should not include the DcsHeader (which is the first 64 bytes of the overall packet)
    pub fn parse(data: &[u8]) -> Result<Vec<Self>, HandlerError> {
        let mut cur = std::io::Cursor::new(data);

        let mut blocks = Vec::new();

        let mut byte_counter = 0;

        // the -4 is because the last 4 bytes of the file are the file CRC
        while byte_counter < data.len() - 4 {
            let block_start_idx = cur.position() as usize;

            // read block ID
            let block_id = cur.read_u8()?;
            let block_len = cur.read_u16::<LittleEndian>()?;
            byte_counter += block_len as usize + 1;

            if block_id != 0x01 {
                // we don't know how to parse this block, so skip forward to the next one
                // Since we've already read 3 bytes (1 for ID, 2 for len), the total bytes to skip os the block_len - 3
                // TODO handle block_id 2 (which is fully described in HRIT_DCS_File_Format_Rev1.pdf)
                warn!(
                    "Skipping unknown DSC block id {}, skipping {} bytes",
                    block_id,
                    block_len - 3
                );
                cur.seek(SeekFrom::Current(block_len as i64 - 3))?;
                continue;
            }
            // read the block message block header (36 bytes)

            // sequence number is 3 bytes
            let sequence = cur.read_u24::<LittleEndian>()?;

            let tmp = cur.read_u8()?;
            let baud_rate = match tmp & 0b111 {
                1 => 100,
                2 => 300,
                3 => 1200,
                _ => {
                    warn!("Unexpected baud rate: {}", tmp & 0b111);
                    continue;
                }
            };
            let platform = match (tmp & 0b1000) >> 3 {
                0 => DcsPlatform::CS1,
                1 => DcsPlatform::CS2,
                x => {
                    warn!("Unexpected platform: {}", x);
                    continue;
                }
            };

            let parity_errors = (tmp & 0b10000) >> 4 == 1;
            let missing_eot = (tmp & 0b100000) >> 5 == 1;

            // ARM flags (Abnormal Received Message)
            let tmp = cur.read_u8()?;
            let addr_corrected = (tmp & 0b1) == 1;
            let bad_addr = (tmp & 0b10) >> 1 == 1;
            let invalid_addr = (tmp & 0b100) >> 2 == 1;
            let incomplete_pdt = (tmp & 0b1000) >> 3 == 1;
            let timing_error = (tmp & 0b10000) >> 4 == 1;
            let unexpected_message = (tmp & 0b100000) >> 5 == 1;
            let wrong_channel = (tmp & 0b1000000) >> 6 == 1;

            // corrected address
            let corrected_addr = cur.read_u32::<LittleEndian>()?;

            // carrier start
            let mut carrier_start_buf = [0; 7];
            cur.read_exact(&mut carrier_start_buf)?;

            // carrier end
            let mut carrier_end_buf = [0; 7];
            cur.read_exact(&mut carrier_end_buf)?;

            // signal strength (10 bits)
            let signal_strength_10x = cur.read_u16::<LittleEndian>()?;
            // mask off the top 6 bytes and then divide by 10
            let signal_strength = (signal_strength_10x & 0x3ff) as f32 / 10.0;

            // freq offset (14 bite)
            let freq_offset_10x = cur.read_i16::<LittleEndian>()?;
            let freq_offset = (freq_offset_10x & 0x3fff) as f32 / 10.0;

            // phase noise (12 bits)
            let phase_noise_100x = cur.read_u16::<LittleEndian>()?;
            let phase_noise = (phase_noise_100x & 0xfff) as f32 / 100.0;

            // phase mod quality
            let good_phase_2x = cur.read_u8()?;
            let good_phase = good_phase_2x as f32 / 2.0;

            // channel/spacecraft
            let tmp = cur.read_u16::<LittleEndian>()?;
            let channel_number = tmp & 0x3ff;
            let space_platform = match (tmp & 0xf000) >> 12 {
                0 => DcsSpacescraft::Unknown,
                1 => DcsSpacescraft::GoesEast,
                2 => DcsSpacescraft::GoesWest,
                3 => DcsSpacescraft::GoesCentral,
                4 => DcsSpacescraft::GoesTest,
                x => {
                    warn!("Unexpected platform: {}", x);
                    DcsSpacescraft::Reserved
                }
            };

            // source code (2bytes)
            let mut source_code_buf = [0; 2];
            cur.read_exact(&mut source_code_buf)?;
            let source_platform = match source_code_buf {
                [b'U', b'P'] => DcsSource::UP,
                [b'U', b'B'] => DcsSource::UB,
                [b'N', b'P'] => DcsSource::NP,
                [b'N', b'B'] => DcsSource::NB,
                [b'X', b'E'] => DcsSource::XE,
                [b'X', b'W'] => DcsSource::XW,
                [b'R', b'E'] => DcsSource::RE,
                [b'R', b'W'] => DcsSource::RW,
                [b'd', b'1'] => DcsSource::D1,
                [b'd', b'2'] => DcsSource::D2,
                [b'L', b'E'] => DcsSource::LE,
                [b'S', b'F'] => DcsSource::SF,
                [b'O', b'W'] => DcsSource::OW,
                x => DcsSource::Unknown(x),
            };

            // Not currently used
            let _secondary_source = cur.read_u16::<LittleEndian>()?;

            // the data length is the total block size minus 41, calculated as:
            // * header (36 bytes)
            // * block ID (1 byte)
            // * block length (2 bytes)
            // * crc16 (2 bytes) found after the block data
            let data_len = block_len as usize - 41;
            let mut data_buf = vec![0; data_len];
            cur.read_exact(&mut data_buf)?;

            let block_end_idx = cur.position() as usize;

            // crc16
            let crc16 = cur.read_u16::<LittleEndian>()?;
            let mut crc = crc_any::CRC::crc16ccitt_false();
            crc.digest(&data[block_start_idx..block_end_idx]);

            let compuated_crc = crc.get_crc() as u16;
            if crc16 != compuated_crc {
                warn!("block CRC mismatch: {} != {}", crc16, compuated_crc);
                continue;
            }

            blocks.push(DcsBlock {
                block_id,
                block_len,
                sequence,
                baud_rate,
                platform,
                parity_errors,
                missing_eot,
                addr_corrected,
                bad_addr,
                invalid_addr,
                incomplete_pdt,
                timing_error,
                unexpected_message,
                wrong_channel,
                corrected_addr,
                carrier_start: carrier_start_buf,
                carrier_end: carrier_end_buf,
                signal_strength,
                freq_offset,
                phase_noise,
                // phase_mod_quality,
                good_phase,
                space_platform,
                channel_number,
                source_platform,
                data: data_buf,
            })
        }

        Ok(blocks)
    }
}
