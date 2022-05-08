/// Calculates a CRC-16
///
/// This CRC has a generator polynominal x^16 + x^12 + x^5 + 1 and is also known as "CCITT"
/// Initial value is 0xFFFF
///
/// Described in 5_LRIT_Mission-data.pdf
pub fn calc_crc16(data: &[u8]) -> u16 {
    let mut crc = crc_any::CRC::crc16ccitt_false();
    crc.digest(data);

    crc.get_crc() as u16
}

/// Calculates as CRC-32
///
/// This CRC is the ISO 3309 CRC
pub fn calc_crc32(data: &[u8]) -> u32 {
    let mut crc = crc_any::CRC::crc32();
    crc.digest(data);
    crc.get_crc() as u32
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_crc16() {
        assert_eq!(crate::crc::calc_crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn test_crc32() {
        let crc = crate::crc::calc_crc32(b"123456789");
        assert_eq!(crc, 0xcbf43926, "crc32: {:x}", crc);
    }
}
