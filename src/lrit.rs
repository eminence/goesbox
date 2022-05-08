use byteorder::{NetworkEndian, ReadBytesExt};
use log::{info, warn};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Read, Write};

use crate::crc;

// M_SDU -- Multiplexing Service Data Unit
// VCLC -- Virtual Channel Link Control
// VCA -- Virtual Channel Access
// M_PDU -- Multiplexing Protocol Data Unit

fn diff_with_wrap(low: u32, high: u32, max: u32) -> u32 {
    //let max = 1 << 24;
    if low <= high {
        high - low
    } else {
        max - low + high
    }
}

#[derive(Clone)]
pub struct LRIT {
    /// The vcid (virtual channel id) that this LRIT file came in on
    pub vcid: u8,
    pub headers: Headers,
    pub data: Vec<u8>,
}

impl Debug for LRIT {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "<LRIT headers: {:?} data.len: {}",
            self.headers,
            self.data.len()
        )
    }
}

/// Virtual Channel Data Unit
///
/// This structure has 6 bytes of header, followed by 886 bytes of data (for a total of 892 bytes).
///
/// # Header
///
/// The 6 byte header has the following fields:
///
/// * 2 bits for a version field (should always be 1)
/// * 14 bits for a VCDU-ID field (upper 8 bits is the SDID, lower 6 bits is the VCID)
/// * 24 bits for a VCDU counter
/// * 8 bits for a signaling field.  The first bit is the replay flag, and lower 7 bits are not used
///
///              1
///      0123456701234567
///     +------------------
///     |..ooooooooIIIIII
///
/// Ref: 3_LRIT_Receiver-specs.pdf page 9
pub struct VCDU<'a> {
    bytes: &'a [u8],
}

impl<'a> VCDU<'a> {
    pub fn new(bytes: &'a [u8]) -> VCDU<'a> {
        VCDU { bytes }
    }

    /// VCDU version
    ///
    /// This should always be 1
    pub fn version(&self) -> u8 {
        (self.bytes[0] & 0b11 << 6) >> 6
    }

    /// Spacecraft ID
    ///
    /// This represents the spacecraft which sent this message
    pub fn SCID(&self) -> u8 {
        (self.bytes[0] & 0x3f) << 2 | (self.bytes[1] & 0xc0) >> 6
    }

    /// Virtual Channel ID
    ///
    /// This is a 6-bit field, so the max ID is 63 (which represents a fill packet)
    pub fn VCID(&self) -> u8 {
        self.bytes[1] & 0x3f
    }

    /// A sequential counter of VCDUs on each virtual channel
    ///
    /// This counter is specified to this VCDU, and can be used to detected dropped packets
    ///
    /// This is a 24-bit field, so this counter is modulo 1<<24
    pub fn counter(&self) -> u32 {
        ((self.bytes[2] as u32) << 16) | ((self.bytes[3] as u32) << 8) | self.bytes[4] as u32
    }

    //const uint8_t* data() const {
    //    return &data_[6];
    //}

    /// The length of the data in bytes
    pub fn len(&self) -> usize {
        self.bytes.len() - 6
    }

    pub fn data(&self) -> &[u8] {
        &self.bytes[6..]
    }

    /// Fill packets are sent on VCID 63
    pub fn is_fill(&self) -> bool {
        self.VCID() == 63
    }
}

/// Ths Transport Service Protocol Data Unit
///
/// This unit stores up to 8190 bytes for a specific APID (application process identifier)
///
/// Ref: 4_LRIT_Transmitter-specs.pdf Page 16
struct TP_PDU {
    /// The header contains 6 bytes
    header: Vec<u8>,
    /// The data field is max 8190 bytes, plus 2 additional bytes for CRC
    data: Vec<u8>,
    vcid: u8,
}

impl TP_PDU {
    pub fn new(vcid: u8) -> TP_PDU {
        TP_PDU {
            header: Vec::with_capacity(6),
            data: Vec::with_capacity(8192),
            vcid,
        }
    }

    pub fn header_complete(&self) -> bool {
        assert!(self.header.len() <= 6);
        self.header.len() == 6
    }

    pub fn data_complete(&self) -> bool {
        if let Some(len) = self.packet_length() {
            assert!(self.data.len() <= len as usize);
            self.data.len() == len as usize
        } else {
            false
        }
    }

    pub fn is_crc_ok(&self) -> bool {
        if self.data_complete() {
            let len = self.data.len();
            // the CRC is over the application data file, and is stored in the last 2 bytes
            let computed = crc::calc_crc16(&self.data[..len - 2]);
            let received = (self.data[len - 2] as u16) << 8 | self.data[len - 1] as u16;
            if computed != received {
                warn!(
                    "Computed CRC {:x} does not match recieved CRC {:x}",
                    computed, received
                );
            }

            computed == received
        } else {
            warn!("Can't check CRC when data isn't complete");
            false
        }
    }

    /// The version of the TP_PDU
    ///
    /// The first 3 bits of the header, this should also be 0
    ///
    pub fn version(&self) -> Option<u8> {
        if self.header.len() > 0 {
            let ver = (self.header[0] >> 5) & 0x7;
            assert_eq!(ver, 0);
            Some(ver)
        } else {
            None
        }
    }

    /// Packet type
    ///
    /// This should always be 0
    pub fn packet_type(&self) -> Option<bool> {
        if self.header.len() > 0 {
            Some((self.header[0] & 0b00010000) > 0)
        } else {
            None
        }
    }

    pub fn secondary_flag(&self) -> Option<bool> {
        if self.header.len() > 0 {
            Some((self.header[0] & 0b00001000) > 0)
        } else {
            None
        }
    }

    /// The Application Process Identifier
    ///
    /// APIDs between 0 and 191 are GOES LRIT application data.
    /// APID 2047 is a fill packet which contains no context
    pub fn APID(&self) -> Option<u16> {
        if self.header.len() >= 2 {
            Some(((self.header[0] & 0b111) as u16) << 8 | self.header[1] as u16)
        } else {
            None
        }
    }

    /// Sequence flags
    ///
    /// This flag has the following values:
    ///
    /// * 3: The user data contains one user data file entirely
    /// * 1: The user data contains the first segment of one user file extending through subsequence packets
    /// * 0: THe user data contains a continuation segment of one user data file, still extending through subsequence packets
    /// * 2: The user data contains the last segment of one user data file beginning in an earlier packet
    pub fn flags(&self) -> Option<u8> {
        if self.header.len() >= 4 {
            Some((self.header[2] & 0xc0) >> 6)
        } else {
            None
        }
    }

    pub fn sequence_count(&self) -> Option<u16> {
        if self.header.len() >= 4 {
            Some(((self.header[2] & 0x3f) as u16) << 8 | self.header[3] as u16)
        } else {
            None
        }
    }

    /// Length of the user data field (including CRC)
    ///
    /// Returns `None` if the full header hasn't been received yet
    pub fn packet_length(&self) -> Option<u16> {
        if self.header_complete() {
            // This header field is documented as "the length of the remainder of the source packet
            // following this field minus 1".  There will always be a 2byte CRC field, so when
            // there is no application data, the packet_length field will be 1.  We'll return "2"
            // in this case.
            let len = ((self.header[4] as u16) << 8 | self.header[5] as u16) + 1;
            assert!(
                len <= 8192,
                "len {} is too long (apid {:?} vcid {})",
                len,
                self.APID(),
                self.vcid
            );
            Some(len)
        } else {
            None
        }
    }

    /// Consume as many bytes as possible to fill the user data section of this PDU
    ///
    /// Returns the total number of bytes read
    pub fn process_bytes(&mut self, bytes: &[u8]) -> usize {
        let bytes_used = if !self.header_complete() {
            // read in as many bytes as we need / as we can
            let needed_bytes = 6 - self.header.len();
            let a = std::cmp::min(needed_bytes, bytes.len());
            assert!(a > 0);
            assert!(a <= 6);
            self.header.extend_from_slice(&bytes[..a]);
            a
        } else {
            0
        };

        if let Some(packet_len) = self.packet_length() {
            // if we know how much data we have and there's more data to read, then let's read it
            // (if we can)
            let needed_bytes = packet_len as usize - self.data.len();
            assert!(needed_bytes > 0);
            let a = std::cmp::min(needed_bytes, bytes.len() - bytes_used);
            self.data
                .extend_from_slice(&bytes[bytes_used..bytes_used + a]);
            bytes_used + a // how many total bytes we used
        } else {
            bytes_used
        }
    }
}

enum DecompInfo {
    NoneNeeded,
    Needed(acres::sz::Sz),
}

/// A utility struct used to build up session layer data (an LRIT file)
///
/// An entire LRIT file will be transmitted via 1 or more TP_PDUs.  This struct
/// will collect them as they arrive, and produce a single LRIT file when complete.
struct Session {
    /// Bytes received so far
    bytes: Vec<u8>,
    /// The most recent sequence number received (from the last TP_PDU)
    last_seq: u16,
    apid: u16,
    needs_decomp: DecompInfo,
    /// The vcid (virtual channel id) of the session
    vcid: u8,
}

/// Returns true if we need to decompress
fn check_headers_for_rice_compression(bytes: &[u8]) -> DecompInfo {
    let headers = read_headers(bytes);
    if let (Some(ref ish), Some(ref rice)) = (headers.img_strucutre, headers.rice_compression) {
        return DecompInfo::Needed(acres::sz::Sz::new(
            acres::sz::Options::from_bits_truncate(rice.flags as u32),
            ish.bits_per_pixel as usize,
            rice.pixels_per_block as usize,
            ish.num_columns as usize,
        ));
    }
    DecompInfo::NoneNeeded
}

impl Session {
    /// Create a new session from the first TP_PDU of some session layer data
    pub fn new_from_pdu(pdu: TP_PDU) -> Session {
        assert!(pdu.header_complete());
        assert!(pdu.data_complete());
        assert!(pdu.is_crc_ok());
        let seq = pdu
            .sequence_count()
            .expect("pdu sequence should never be None");
        let apid = pdu.APID().expect("APID should never be None");

        let _ver = pdu.version();

        // According to a comment in goestools, the first 10 bytes of this data is garbage
        // so ignore the first 10 bytes from this first TP_PDU

        // last 2 bytes of pdu's data will be a CRC that we have already validated
        let mut bytes = pdu.data;
        bytes.truncate(bytes.len() - 2);
        bytes = bytes.split_off(10);

        // we need to check a few things here:
        // 1. is this an image file type (filetype_code == 0)
        // 2. does it have a compression flag in the image strucuture record?
        // 3. does it have a rice compression header?
        //
        // if the answer is 'yes' to all three, then we should do decompression of this tp_pdu (and
        // all other tp_pdus in this session) before adding it to self.bytes

        // see if we have enough data to extract a primary header
        let needs_decomp = if let Some(prim) = PrimaryHeader::from_bytes(&bytes) {
            if bytes.len() >= prim.total_header_length as usize {
                // we have enough data to extract all the headers

                check_headers_for_rice_compression(&bytes)
            } else {
                warn!("Not enough data in first TP_PDU to extract all the headers (need {} bytes, but only have {} bytes)", prim.total_header_length, bytes.len());
                DecompInfo::NoneNeeded
            }
        } else {
            warn!(
                "First TP_PDU didn't have enough data for a primary header (only {} bytes)",
                bytes.len()
            );
            DecompInfo::NoneNeeded
        };

        if let DecompInfo::Needed(_params) = &needs_decomp {
            //info!("tp_pdu's in session {} need rice decompression", apid);
            let headers = read_headers(&bytes);

            let data = &bytes[headers.primary.total_header_length as usize..];
            assert_eq!(
                data.len(),
                0,
                "Expected data len to be zero, but was actually {}",
                data.len()
            );
            //info!("{} bytes to decompress, pixels per scanline {}", data.len(), params.pixels_per_scanline);
        }

        // TODO
        // read enough data to extract the full set of LRIT headers
        // check for rice and image strucuture headers
        // set up

        Session {
            last_seq: seq,
            bytes,
            apid,
            needs_decomp,
            vcid: pdu.vcid,
        }
    }

    pub fn append(&mut self, mut pdu: TP_PDU, stats: &crate::stats::Stats) {
        assert!(pdu.header_complete());
        assert!(pdu.data_complete());
        if !pdu.is_crc_ok() {
            warn!(
                "Refusing to append data that failed CRC (apid {})",
                pdu.APID().unwrap()
            );
            return;
        }
        // remove the 2 CRC bytes (which we've just verified)
        pdu.data.truncate(pdu.data.len() - 2);

        let new_seq = pdu
            .sequence_count()
            .expect("pdu sequence should never be None");

        // Note: 4_LRIT_Transmitter-specs.pdf section 6.2.1 says that this sequence number is 14 bit modulo 16394
        //       but that is almost certainly a typo
        if diff_with_wrap(self.last_seq as u32, new_seq as u32, 1 << 14) > 1 {
            //if new_seq != self.last_seq + 1 {
            let skipped = new_seq as isize - self.last_seq as isize;
            warn!("VC XXX: Detected TP_PDU drop (skipped {} packet(s) on APID {}; prev: {}, packet: {})",
            skipped - 1, self.apid, self.last_seq, new_seq);
        }
        self.last_seq = new_seq;
        if let DecompInfo::Needed(ref mut params) = self.needs_decomp {
            let num_columns = params.pixels_per_scanline() as usize;
            assert!(pdu.data.len() <= num_columns, "session needs rice decomp, but bytes to decomp ({}) is greater than image cols ({})", pdu.data.len() - 2, num_columns);

            let mut out_buf = Vec::with_capacity(num_columns as usize);
            // match acres::decompress(&pdu.data, &mut out_buf, params) {
            match params.decompress(&pdu.data, &mut out_buf) {
                Ok(buf) => {
                    assert_eq!(buf.len(), num_columns, "Successfully decompressed TP_PDU, but bytes out of decompressor ({}) doesn't match num columns ({})", buf.len(), num_columns);
                    self.bytes.extend_from_slice(buf);
                }
                Err(rc) => panic!("Failed to decompress with rc {}", rc),
            }
        } else {
            // sanity check:
            assert!(
                pdu.data.len() < 1_000_000,
                "tp_pdu data length is suspicious {}",
                pdu.data.len()
            );
            self.bytes.extend(pdu.data);
        }
    }

    pub fn finish(mut self) -> LRIT {
        //let header = crate::lrit::PrimaryHeader::from_data(&self.bytes[10..]);
        //info!("primary header: {:?}", header);
        let headers = read_headers(&self.bytes);
        let data = self
            .bytes
            .split_off(headers.primary.total_header_length as usize);
        if let Some(_rice) = &headers.rice_compression {
            //let ish = headers.img_strucutre.as_ref().unwrap();
            //info!("{:?}", headers);
            //info!("ish.cols={}, datalen={}", ish.num_columns, data.len());
        }
        return LRIT { vcid: self.vcid, headers, data };
        //info!("Headers: {:?}", headers);

        //let root = std::path::Path::new("/nas/achin/devel/goes-dht/out_new");
        //if let Some(header) = &headers.primary {
        //    if header.filetype_code == 1 {
        //        // messages
        //        info!("messages Headers: {:?}", headers);
        //    }
        //    if header.filetype_code == 128 {
        //        // mmeteorological data
        //        info!("meteorological Headers: {:?}", headers);
        //    }
        //    if header.filetype_code == 0 {
        //    }

        //    if header.filetype_code == 2 {
        //    }
        //}
    }
}

/// A structure that parses LRIT data out of one specific virtual channel
///
/// This structure doesn't have a direct mapping to any of the offical LRIT structures.
///
/// Different types of data are transmitted on each virtual channel.
pub struct VirtualChannel {
    /// The virtual channel ID
    id: u8,

    /// Holds the current incomplete TP_PDU that we're working on (if any)
    current_tp_pdu: Option<TP_PDU>,

    /// A map between APID and session-layer data
    apid_map: HashMap<u16, Session>,

    last_counter: u32,
}

impl VirtualChannel {
    pub fn new(id: u8, initial_counter: u32) -> VirtualChannel {
        VirtualChannel {
            id,
            current_tp_pdu: None,
            apid_map: HashMap::new(),
            last_counter: initial_counter,
        }
    }

    /// Extract TP_PUDs from a VCDU, returning any completed LRIT files
    pub fn process_vcdu(&mut self, vcdu: VCDU, stats: &mut crate::stats::Stats) -> Vec<LRIT> {
        let data = vcdu.data();
        assert_eq!(data.len(), 886);
        assert_eq!(vcdu.VCID(), self.id);

        // check this vcdu counter against the last one received
        if diff_with_wrap(self.last_counter, vcdu.counter(), 1 << 24) > 1 {
            // we're missing some packets -- if we've got an incomplete TP_PDU,
            // we need to drop it (because we can't know if the missing packet(s)
            // started a new one or finished the current one.
            self.current_tp_pdu.take();
            info!("VC {} Dropping incomplete TP_PDU", self.id);
        }

        self.last_counter = vcdu.counter();

        let first_header = {
            // read off the first 2 bytes and extract a first header pointer

            // Ref: 3_LRIT_Receiver-specs.pdf Figure 5 M_PDU Structure
            // Ref: 5_LRIT_Mission-data.pdf Page 3
            let spare = (data[0] & 0b11111000) >> 3;
            assert_eq!(spare, 0);

            ((data[0] & 0b111) as usize) << 8 | data[1] as usize
        };

        let mut offset = 2; // + if first_header == 2047 { 0 } else { first_header };
        let mut lrits: Vec<LRIT> = Vec::new();

        // if first_header is non-zero, and we still have an open incomplete TP_PDU, read data
        // up-to first_header to complete it
        if let Some(mut tp_pdu) = self.current_tp_pdu.take() {
            assert!(!tp_pdu.data_complete());

            if let Some(total_len) = tp_pdu.packet_length() {
                let bytes_needed = total_len as usize - tp_pdu.data.len();
                if first_header != 2047 && first_header < bytes_needed {
                    // if first_header is not 2047, then it represents how many bytes to read
                    // before the header
                    // TODO debug 'needed 661 bytes to finish this TP_PDU, but first_header is only 0'
                    panic!(
                        "needed {} bytes to finish this TP_PDU, but first_header is only {}",
                        bytes_needed, first_header
                    );
                }
            }

            // we have an unfinished tp_pdu, which we may or may not be able to complete with this new data
            // (however, we do expect to always be able to complete the 6 byte header)
            offset += tp_pdu.process_bytes(&data[offset..]);
            assert!(tp_pdu.header_complete());

            if tp_pdu.data_complete() {
                lrits.extend(self.process(tp_pdu, stats));

                // at this point, if we have another packet, we should expect it to start at our current offset.
                // remember "first_header" is relative to the start of the packet zone, but "offset" is relative to the start of
                // entire data (which includes a 2 byte header).
                if first_header != 2047 {
                    assert_eq!(
                        offset - 2,
                        first_header,
                        "offset={} first_header={}",
                        offset,
                        first_header
                    );
                }
                // assert!(offset - 2 <= first_header, "offset {} is past first_header {}", offset - 2, first_header);
            } else {
                // if not complete, then we should have no more bytes to read
                if first_header != 2047 {
                    info!("XXX TP_PDU is still completed, first_header was {first_header}");
                }
                assert_eq!(offset, data.len());
                self.current_tp_pdu = Some(tp_pdu); // store it for later
                return lrits;
            }
        } else {
            // the "first_header" is the offset to the first TP_PDU that contains a header.  Any data before this
            // is going to be from some previously started TP_PDU
            offset = 2 + first_header;
        }

        // at this point we should not have any pending tp_pdus
        assert!(self.current_tp_pdu.is_none());

        if first_header == 2047 {
            return lrits; // fill packet
        }

        while offset < data.len() {
            let mut tp_pdu = TP_PDU::new(vcdu.VCID());
            offset += tp_pdu.process_bytes(&data[offset..]);
            // note that while "first_header" is documented to point to the first TP_PDU with a header, it doesn't
            // mean that the TP_PDU will have a complete header!

            if tp_pdu.header_complete() && tp_pdu.data_complete() {
                lrits.extend(self.process(tp_pdu, stats));
            } else {
                // not complete, keep it around!
                self.current_tp_pdu = Some(tp_pdu);
                assert_eq!(offset, data.len());
            }
        }

        lrits
    }

    /// Process a completed TP_PDU
    ///
    /// If this was the last TP_PDU in an LRIT file, a new LRIT file can be returned.
    /// Else, this TP_PDU is added
    fn process(&mut self, tp_pdu: TP_PDU, stats: &mut crate::stats::Stats) -> Option<LRIT> {
        let apid = tp_pdu.APID().unwrap();
        if apid == 2047 {
            return None;
        }
        stats.record(crate::stats::Stat::APID(apid));
        let flags = tp_pdu.flags().unwrap();
        assert!(flags >= 0);
        assert!(flags <= 3);

        if flags == 1 || flags == 3 {
            // x == 1 means this is the first segment of a new data file, and there will be
            // more to come.
            // x == 3 means this is the first and only segment of a new data file
            // (Ref: 4_LRIT_Transmitter-specs.pdf page 20)

            // see if there's a previous record of this apid in our map.  If so, it won't be valid.
            if let Some(_pdu) = self.apid_map.remove(&apid) {
                warn!("XXX Dropping old apid data {}", apid);
            }

            let session = Session::new_from_pdu(tp_pdu);
            if flags == 1 {
                // we'll expect to receive more data with this same APID
                self.apid_map.insert(apid, session);
            } else {
                //info!("Starting (and finishing) apid={} (total data len {})", apid, session.bytes.len());
                let lrit = session.finish();
                //info!("{:?}", lrit);
                return Some(lrit);
            }
        } else if flags == 0 {
            // we should expect that the starting packets were already received, and that we'll
            // receive some more.
            if let Some(ref mut sess) = self.apid_map.get_mut(&apid) {
                sess.append(tp_pdu, stats);
            } else {
                // ignore this
                //println!("Dropping data for unknow apid {}", apid);
                stats.record(crate::stats::Stat::DiscardedDataPacket);
            }
        } else if flags == 2 {
            // this is the final packet
            if let Some(mut sess) = self.apid_map.remove(&apid) {
                sess.append(tp_pdu, stats);
                //info!("got final TP_PDU packet for APID {} !", apid);
                //info!("this session frame has {} bytes", sess.bytes.len());
                let lrit = sess.finish();
                return Some(lrit);
            } else {
                info!(
                    "Got a final TP_PDU packet for APID {}, but we weren't tracking this one yet",
                    apid
                );
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Headers {
    pub primary: PrimaryHeader,
    pub img_strucutre: Option<ImageStructureRecord>,
    pub img_navigation: Option<ImageNavigationRecord>,
    pub img_data: Option<ImageDataFunctionRecord>,
    pub img_segment: Option<ImageSegmentIdentificationRecord>,
    pub annotation: Option<AnnotationRecord>,
    pub noaa: Option<NOAALRITHeader>,
    pub header: Option<HeaderStructureRecord>,
    pub timestamp: Option<TimeStampRecord>,
    pub text: Option<AncillaryTextRecord>,
    pub rice_compression: Option<RiceCompressionSecondaryHeader>,
}

impl Headers {
    pub fn new(primary: PrimaryHeader) -> Headers {
        Headers {
            primary,
            img_strucutre: None,
            img_navigation: None,
            img_data: None,
            img_segment: None,
            annotation: None,
            noaa: None,
            header: None,
            timestamp: None,
            text: None,
            rice_compression: None,
        }
    }
}

pub trait LRITHeader: std::fmt::Debug {
    const TYPE: u8;
}

/// Attempts to read LRIT headers
///
/// Ref: 3_LRIT_Receiver-specs.pdf
///
/// Ref: 5_LRIT_Mission-data.pdf
pub fn read_headers(data: &[u8]) -> Headers {
    // the general approach is to read 1 byte, which indicates what type of header we have, and
    // then read the full header once we know what it is and how long it is.
    //
    // There always must be a primary header at the first header, so we read that first
    let prim_header = PrimaryHeader::from_bytes(&data).expect("Missing primary header");
    assert_eq!(prim_header.header_type, 0);
    assert_eq!(prim_header.header_record_lenth, 16);
    let mut headers = Headers::new(prim_header);

    if headers.primary.total_header_length == 16 {
        // there are no more headers, so we're done
        return headers;
    }

    let prim_header = &headers.primary;

    let mut offset = prim_header.header_record_lenth as usize;

    while offset < prim_header.total_header_length as usize {
        // peek at next byte
        match &data[offset] {
            0 => panic!("Found unexpected header type 0, after already reading a primary header"),
            1 => {
                // Mandatory for image data
                let h = ImageStructureRecord::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.img_strucutre = Some(h);
            }
            2 => {
                // Optional for image data
                let h = ImageNavigationRecord::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.img_navigation = Some(h);
            }
            3 => {
                // Optional for image data
                let h = ImageDataFunctionRecord::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.img_data = Some(h);
            }
            4 => {
                // Mandatory for Image Data, Text, Meteorologic Data, and GTS Messages
                let h = AnnotationRecord::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.annotation = Some(h);
            }
            5 => {
                // Mandatory for GTS Messages, optional for image/text/meteorological data
                let h = TimeStampRecord::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.timestamp = Some(h);
            }
            6 => {
                // Optional for image/service messages/text/meteorological data
                let h = AncillaryTextRecord::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.text = Some(h);
            }
            // 7 -- encrytpion header
            // Optional for image/text/meteorological/GTS
            128 => {
                let h = ImageSegmentIdentificationRecord::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.img_segment = Some(h);
            }
            129 => {
                let h = NOAALRITHeader::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.noaa = Some(h);
            }
            130 => {
                let h = HeaderStructureRecord::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.header = Some(h);
            }
            131 => {
                // Optional for all file types
                let h = RiceCompressionSecondaryHeader::from_bytes(&data[offset..]).unwrap();
                offset += h.header_record_lenth as usize;
                headers.rice_compression = Some(h);
            }
            x => {
                panic!("Found unexpected header type {}", x);
            }
        }
    }

    headers
}

#[derive(Debug, Clone)]
pub struct PrimaryHeader {
    /// Header type, should always be 0 (zero)
    header_type: u8,

    /// Length of this header record, should always be 16
    pub header_record_lenth: u16,

    /// File type code
    ///
    /// This indicates what other headers we might expect to see
    pub filetype_code: u8,

    /// Total header length
    ///
    /// Total length of all header records (including this one), in bytes
    ///
    /// Since the primary header itself is 16 bytes, if total_header_length == 16, then there are no other headers
    pub total_header_length: u32,

    /// Data field length
    ///
    /// Total length of the data field, in bits
    pub data_field_bits: u64,
}

impl LRITHeader for PrimaryHeader {
    const TYPE: u8 = 0;
}

impl PrimaryHeader {
    pub const fn header_type() -> u8 {
        0
    }
    pub fn from_bytes(data: &[u8]) -> Option<PrimaryHeader> {
        if data.len() < 16 {
            return None;
        }

        let mut cur = std::io::Cursor::new(data);

        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();
        let code = cur.read_u8().unwrap();
        let total_header_len = cur.read_u32::<NetworkEndian>().unwrap();
        let data_len = cur.read_u64::<NetworkEndian>().unwrap();

        let header = PrimaryHeader {
            header_type: typ,
            header_record_lenth: len,
            filetype_code: code,
            total_header_length: total_header_len,
            data_field_bits: data_len,
        };

        Some(header)
    }
}

#[derive(Debug, Clone)]
pub struct ImageStructureRecord {
    /// Header type, must always be 1
    header_type: u8,

    /// Length of this header record, should always be 9
    header_record_lenth: u16,

    pub bits_per_pixel: u8,

    pub num_columns: u16,

    pub num_lines: u16,

    pub compression: u8,
}
impl LRITHeader for ImageStructureRecord {
    const TYPE: u8 = 1;
}

impl ImageStructureRecord {
    pub const fn header_type() -> u8 {
        1
    }
    pub fn from_bytes(data: &[u8]) -> Option<ImageStructureRecord> {
        if data.len() < 16 {
            return None;
        }

        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();
        let bpp = cur.read_u8().unwrap();
        let cols = cur.read_u16::<NetworkEndian>().unwrap();
        let rows = cur.read_u16::<NetworkEndian>().unwrap();
        let cflg = cur.read_u8().unwrap();

        let header = ImageStructureRecord {
            header_type: typ,
            header_record_lenth: len,
            bits_per_pixel: bpp,
            num_columns: cols,
            num_lines: rows,
            compression: cflg,
        };

        Some(header)
    }
}

#[derive(Debug, Clone)]
pub struct ImageNavigationRecord {
    /// Header type, must always be 2
    header_type: u8,

    /// Length of this header record, should always be 51
    header_record_lenth: u16,

    projection_name: String,

    column_scaling_factor: i32,
    line_scaling_factor: i32,
    column_offset: i32,
    line_offset: i32,
}

impl LRITHeader for ImageNavigationRecord {
    const TYPE: u8 = 2;
}

impl ImageNavigationRecord {
    pub const fn header_type() -> u8 {
        2
    }
    pub fn from_bytes(data: &[u8]) -> Option<ImageNavigationRecord> {
        if data.len() < 51 {
            return None;
        }

        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let mut name_buf = [' ' as u8; 32];
        cur.read_exact(&mut name_buf);
        let name = String::from_utf8_lossy(&name_buf)
            .to_owned()
            .trim()
            .to_owned();

        let col_scaling_factor = cur.read_i32::<NetworkEndian>().unwrap();
        let line_scaling_factor = cur.read_i32::<NetworkEndian>().unwrap();
        let col_offset = cur.read_i32::<NetworkEndian>().unwrap();
        let line_offset = cur.read_i32::<NetworkEndian>().unwrap();

        let header = ImageNavigationRecord {
            header_type: typ,
            header_record_lenth: len,
            projection_name: name,
            column_scaling_factor: col_scaling_factor,
            line_scaling_factor: line_scaling_factor,
            column_offset: col_offset,
            line_offset: line_offset,
        };

        Some(header)
    }
}

/// This header specifies an alphanumeric annotation for the fil
///
/// Mandatory for Image Data, Text, Meteorologic Data, and GTS Messages (4_LRIT_Transmitter-specs.pdf Table 16)
///
/// Source: 4_LRIT_Transmitter-specs.pdf Table 10 (page 13)
#[derive(Debug, Clone)]
pub struct AnnotationRecord {
    /// Header type, must always be 4
    header_type: u8,

    /// Length of this header record (variable)
    pub header_record_lenth: u16,

    pub text: String,
}

impl LRITHeader for AnnotationRecord {
    const TYPE: u8 = 4;
}

impl AnnotationRecord {
    pub const fn header_type() -> u8 {
        4
    }
    pub fn from_bytes(data: &[u8]) -> Option<AnnotationRecord> {
        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let mut buf = Vec::with_capacity(len as usize - 3);
        buf.resize(len as usize - 3, ' ' as u8);

        cur.read_exact(&mut buf);
        let text = String::from_utf8_lossy(&buf).to_owned().trim().to_owned();

        let header = AnnotationRecord {
            header_type: typ,
            header_record_lenth: len,
            text,
        };

        Some(header)
    }
}

#[derive(Debug, Clone)]
pub struct NOAALRITHeader {
    /// Header type, must always be 129
    header_type: u8,

    /// Length of this header record, must be 14
    pub header_record_lenth: u16,

    pub product_id: u16,
    pub product_subid: u16,
    pub parameter: u16,
    pub noaa_compression: u8,
}

impl LRITHeader for NOAALRITHeader {
    const TYPE: u8 = 129;
}

impl NOAALRITHeader {
    pub const fn header_type() -> u8 {
        129
    }
    pub fn from_bytes(data: &[u8]) -> Option<NOAALRITHeader> {
        if data.len() < 14 {
            return None;
        }

        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let mut buf = [' ' as u8; 4];
        cur.read_exact(&mut buf);
        let agency_sig = String::from_utf8_lossy(&buf).to_owned().trim().to_owned();

        let product_id = cur.read_u16::<NetworkEndian>().unwrap();
        let product_subid = cur.read_u16::<NetworkEndian>().unwrap();
        let parameter = cur.read_u16::<NetworkEndian>().unwrap();
        let noaa_compression = cur.read_u8().unwrap();

        let header = NOAALRITHeader {
            header_type: typ,
            header_record_lenth: len,
            product_id,
            product_subid,
            parameter,
            noaa_compression,
        };

        Some(header)
    }
}

#[derive(Debug, Clone)]
pub struct HeaderStructureRecord {
    /// Header type, must always be 130
    header_type: u8,

    /// Length of this header record (variable)
    header_record_lenth: u16,

    text: String,
}

impl LRITHeader for HeaderStructureRecord {
    const TYPE: u8 = 130;
}

impl HeaderStructureRecord {
    pub const fn header_type() -> u8 {
        130
    }
    pub fn from_bytes(data: &[u8]) -> Option<HeaderStructureRecord> {
        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let mut buf = Vec::with_capacity(len as usize - 3);
        buf.resize(len as usize - 3, ' ' as u8);

        cur.read_exact(&mut buf);
        let text = String::from_utf8_lossy(&buf).to_owned().trim().to_owned();

        let header = HeaderStructureRecord {
            header_type: typ,
            header_record_lenth: len,
            text,
        };

        Some(header)
    }
}

#[derive(Clone)]
pub struct ImageDataFunctionRecord {
    /// Header type, must always be 3
    header_type: u8,

    /// Length of this header record (variable)
    header_record_lenth: u16,

    data: Vec<u8>,
}

// A custom implementation that doesn't show all the bytes of self.data
impl std::fmt::Debug for ImageDataFunctionRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "ImageDataFunctionRecord {{ header_type: {:?}, header_record_lenth: {:?}, data: {} bytes }}", self.header_type, self.header_record_lenth, self.data.len())
    }
}

impl LRITHeader for ImageDataFunctionRecord {
    const TYPE: u8 = 3;
}

impl ImageDataFunctionRecord {
    pub const fn header_type() -> u8 {
        3
    }
    pub fn from_bytes(data: &[u8]) -> Option<ImageDataFunctionRecord> {
        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let mut buf = Vec::with_capacity(len as usize - 3);
        buf.resize(len as usize - 3, 0u8);

        cur.read_exact(&mut buf);

        let header = ImageDataFunctionRecord {
            header_type: typ,
            header_record_lenth: len,
            data: buf,
        };

        Some(header)
    }
}

#[derive(Debug, Clone)]
pub struct TimeStampRecord {
    /// Header type, must always be 5
    header_type: u8,

    /// Length of this header record, must be 10
    header_record_lenth: u16,

    /// CCSDS time
    ///
    /// CCSDS time is a 2-byte counter of the number of dates from 1 January 1958, followed by
    /// 4-byte coutner of the milliseconds of that day
    time: [u8; 7],
}

impl LRITHeader for TimeStampRecord {
    const TYPE: u8 = 5;
}

impl TimeStampRecord {
    pub fn from_bytes(data: &[u8]) -> Option<TimeStampRecord> {
        if data.len() < 14 {
            return None;
        }

        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let mut time = [0u8; 7];
        cur.read_exact(&mut time);

        let header = TimeStampRecord {
            header_type: typ,
            header_record_lenth: len,
            time,
        };

        Some(header)
    }
}

#[derive(Debug, Clone)]
pub struct AncillaryTextRecord {
    /// Header type, must always be 6
    header_type: u8,

    /// Length of this header record (variable)
    header_record_lenth: u16,

    pub text: String,
}

impl LRITHeader for AncillaryTextRecord {
    const TYPE: u8 = 6;
}

impl AncillaryTextRecord {
    pub const fn header_type() -> u8 {
        6
    }
    pub fn from_bytes(data: &[u8]) -> Option<AncillaryTextRecord> {
        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let mut buf = Vec::with_capacity(len as usize - 3);
        buf.resize(len as usize - 3, ' ' as u8);

        cur.read_exact(&mut buf);
        let text = String::from_utf8_lossy(&buf).to_owned().trim().to_owned();

        let header = AncillaryTextRecord {
            header_type: typ,
            header_record_lenth: len,
            text,
        };

        Some(header)
    }
}

#[derive(Debug, Clone)]
pub struct RiceCompressionSecondaryHeader {
    /// Header type, must always be 131
    header_type: u8,

    /// Length of this header record, must be 7
    header_record_lenth: u16,

    flags: u16,

    pixels_per_block: u8,

    scanlines_per_packet: u8,
}

impl LRITHeader for RiceCompressionSecondaryHeader {
    const TYPE: u8 = 131;
}

impl RiceCompressionSecondaryHeader {
    pub const fn header_type() -> u8 {
        131
    }
    pub fn from_bytes(data: &[u8]) -> Option<RiceCompressionSecondaryHeader> {
        if data.len() < 7 {
            return None;
        }

        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let flags = cur.read_u16::<NetworkEndian>().unwrap();
        let pixels_per_block = cur.read_u8().unwrap();
        let scanlines_per_packet = cur.read_u8().unwrap();

        let header = RiceCompressionSecondaryHeader {
            header_type: typ,
            header_record_lenth: len,
            flags,
            pixels_per_block,
            scanlines_per_packet,
        };

        Some(header)
    }
}

#[derive(Debug, Clone)]
pub struct ImageSegmentIdentificationRecord {
    /// Header type, must always be 128
    header_type: u8,

    /// Length of this header record, must be 17
    header_record_lenth: u16,

    pub image_id: u16,

    pub segment_seq: u16,

    pub start_col: u16,
    pub start_line: u16,

    pub max_segment: u16,
    pub max_column: u16,
    pub max_row: u16,
}

impl LRITHeader for ImageSegmentIdentificationRecord {
    const TYPE: u8 = 128;
}

impl ImageSegmentIdentificationRecord {
    pub const fn header_type() -> u8 {
        128
    }
    pub fn from_bytes(data: &[u8]) -> Option<ImageSegmentIdentificationRecord> {
        if data.len() < 17 {
            return None;
        }

        let mut cur = std::io::Cursor::new(data);
        let typ = cur.read_u8().unwrap();
        let len = cur.read_u16::<NetworkEndian>().unwrap();

        let image_id = cur.read_u16::<NetworkEndian>().unwrap();
        let segment_seq = cur.read_u16::<NetworkEndian>().unwrap();
        let start_col = cur.read_u16::<NetworkEndian>().unwrap();
        let start_line = cur.read_u16::<NetworkEndian>().unwrap();
        let max_segment = cur.read_u16::<NetworkEndian>().unwrap();
        let max_column = cur.read_u16::<NetworkEndian>().unwrap();
        let max_row = cur.read_u16::<NetworkEndian>().unwrap();

        let header = ImageSegmentIdentificationRecord {
            header_type: typ,
            header_record_lenth: len,
            image_id,
            segment_seq,
            start_col,
            start_line,
            max_segment,
            max_column,
            max_row,
        };

        Some(header)
    }
}
