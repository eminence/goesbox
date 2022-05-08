//! Various ulitities for parsing EMWIN and NWS data
//!
//!
pub mod wmo;

use chrono::Utc;

/// Data parsed from an EMWIN filename
///
/// The EMWIN filename starts with 1 letter "pflag" that indicates its origin:
///
/// * A -- Standard WMO product heading follows
/// * Z -- Originating Center's local product identifier (used for images)
///
/// # References
///
/// * https://www.weather.gov/tg/awips
/// * https://www.weather.gov/tg/headef
/// * https://library.wmo.int/doc_num.php?explnum_id=10469
#[derive(Debug)]
pub struct ParsedEmwinName {
    pub pflag: PFlag,

    pub data_type_1: wmo::WMODataTypeT1,
    pub data_type_2: wmo::WMODataTypeT2,

    pub area: wmo::Area,

    /// A 2-digit numeric
    pub originator: Originator,
    pub location: Location,

    pub date: chrono::DateTime<Utc>,
    pub sequence: u32,

    pub priority: Priority,

    pub legacy_filename: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Priority {
    /// Highest priority (1)
    Highest,
    /// High priority (2)
    High,
    /// Medium priority (3)
    Medium,
    /// Low priority (4)
    Low,
}

/// The site that originated/issued the bulletin
///
/// Reference: https://www.weather.gov/tg/awips
#[derive(Debug)]
pub enum Originator {
    /// Issued from U.S. Pacific WFO
    UsPacific,
    /// Issued from Northeast US WFO
    UsNorthEast,
    UsSouthEast,
    UsNorthCentral,
    UsSouthCentral,
    UsRockyMountains,
    UsWestCoast,
    SouthEastAlaska,
    CentralAlaska,
    NorthEastAlaska,
    Unknown(u8),
}

impl Originator {
    // Reference: https://www.weather.gov/tg/awips
    pub fn from_ii(i1: u8, i2: u8) -> Originator {
        if i1 >= 4 && i1 <= 8 {
            match i2 {
                0 => Originator::UsPacific,
                1 => Originator::UsNorthEast,
                2 => Originator::UsSouthEast,
                3 => Originator::UsNorthCentral,
                4 => Originator::UsSouthCentral,
                5 => Originator::UsRockyMountains,
                6 => Originator::UsWestCoast,
                7 => Originator::SouthEastAlaska,
                8 => Originator::CentralAlaska,
                9 => Originator::NorthEastAlaska,
                _ => unreachable!(),
            }
        } else {
            Originator::Unknown(i1 * 10 + i2)
        }
    }
}

/// This is a 4-letter international location identifier
///
/// Some common locations for NWS centers are included in this enum.  Everything else in captured
/// in in "Other" variant.
///
/// Reference: https://w2.weather.gov/source/datamgmt/xr07_Center_ID_List.html
#[derive(Debug)]
pub enum Location {
    /// KKCI - Aviation Weather Center, Kansas City, MO
    KKCI,
    /// KMSC - Marshall Space Flight Center, Huntsville, AL
    KMSC,
    /// KMWI - USDA National Computer Center (NCC), Kansas City, MO [fire weather]
    KMWI,
    /// KNAS - Jet Proportion Lab (JBL) Langely, NASA
    KNAS,
    /// KNCF - AWIPS/NOAAPort Network Control Facility
    KNCF,
    /// KNEC - National Earthquake Center, Golden, CO
    KNEC,
    /// KNES - NESDIS, National Environmental Satellite, Data, and Information Service
    KNES,
    /// KNHC - Tropical Prediction Center, Miami, FL
    KNHC,
    /// KNKA - FAA Weather Message Switching Center Replacement (WMSCR) Atlanta, GA/ Salt Lake City, UT
    KNKA,
    /// KNWC - USN Fleet Numerical Meteorological and Oceanographic Center (FNMOC), Monterrey, CA
    KNWC,
    /// KWAT - National Water Center
    KWAT,
    /// KPML - NOAA's Pacific Marine Environmental Laboratory in Seattle, WA
    KPML,
    /// KWAL - Wallops Island Earth Station, Wallops Island, VA
    KWAL,
    /// KWBC - GISC/RTH Washington, DC
    KWBC,
    /// KWCO - National Water Center
    KWCO,
    /// KWIN - Emergency Managers Weather Information Network (EMWIN)
    KWIN,
    ///KWNB - National Data Buoy Center, Southern MS
    KWNB,
    /// KWNC - Climate Prediction Center, NCEP, College Park, MD
    KWNC,
    /// KWNE - Environmental Modeling Center, NCEP, College Park, MD
    KWNE,
    /// KWNH - Weather Prediction Center, NCEP, College Park, MD
    KWNH,
    /// KWNJ - Johnson Space Center, Houston, TX
    KWNJ,
    /// KWNM - Ocean Prediction Center, NCEP, College Park, MD
    KWNM,
    /// KWNO - NCEP Central Operations, NCEP, College Park, MD
    KWNO,
    /// KWNP - Space Environment Center, Boulder, CO
    KWNP,
    /// KWNS - Storm Prediction Center, Norman, OK
    KWNS,
    /// KWOH - NWS Office of Hydrology / HADS System
    KWOH,
    /// PAAQ - National Tsunami Warning Center (NTWC) Palmer, AK
    PAAQ,
    /// PAWU - Alaskan Aviation Weather Unit, AK
    PAWU,
    /// PGTW - Joint Typhoon Warning Center (JTWC)
    PGTW,
    /// PHEB - Pacific Tsunami Warning Center (PTWC)
    PHEB,

    /// KWBD -- DOWNSCALED GFS USING ETA EXTENSION (DGEX)
    KWBD,
    /// KWBE -- North American Mesoscale (NAM) Model
    KWBE,
    /// KWBF -- NGM
    KWBF,
    /// KWBG -- RUC
    KWBG,
    /// KWBH -- MRF
    KWBH,
    /// KWBI -- SST/ SEA SURFACE TEMPERATURE
    KWBI,
    /// KWBJ -- WIND-WAVE FORECAST MODELS
    KWBJ,
    /// KWBK -- GLOBAL ENSEMBLE FORECASTS
    KWBK,
    /// KWBL -- REGIONAL ENSEMBLE FORECASTS
    KWBL,
    /// KWBM -- Ocean Models/Extratropical Storm Surge Model
    KWBM,
    /// KWBN - ** NATIONAL DIGITAL FORECAST DATABASE (NDFD) PRODUCTS **
    KWBN,
    /// KWBO -- MERGE OF MODELS
    KWBO,
    /// KWBP -- AQM/AIR QUALITY MODEL
    KWBP,
    /// KWBR -- Real Time Mesoscale Analysis/ Analysis of Error
    KWBR,
    /// KWBS -- Hires Window Model (ARW, NMM); Five Domains (West/East US, AK, HI, PR)
    KWBS,
    /// KWBT -- GFS Downscale Guidance
    KWBT,
    /// KWBU -- Hurricane Wave Model
    KWBU,
    /// KWBV -- North American Ensemble Forecast System (NAEFS) - CMC
    KWBV,
    /// KWBW -- Real-Time Ocean Forecast System - RTOFS
    KWBW,
    /// KWBX -- ECMWF
    KWBX,
    /// KWBZ -- REFER TO GRIB PDS
    KWBZ,

    /// KWBQ -- MDL Gridded MOS Products
    KWBQ,
    /// KWEA -- MDL National Blend of Models products for CONUS and Oceanic regions
    KWEA,
    /// KWEB -- MDL National Blend of Models products for CONUS and Oceanic regions - additional elements
    KWEB,
    /// KWEC -- MDL National Blend of Models products for Alaska region
    KWEC,
    /// KWED -- MDL National Blend of Models products for Alaska region - additional elements
    KWED,
    /// KWEE -- MDL National Blend of Models products for Hawaii region
    KWEE,
    /// KWEF -- MDL National Blend of Models products for Hawaii region - additional elements
    KWEF,
    /// KWEG -- MDL National Blend of Models products for Puerto Rico region
    KWEG,
    /// KWEH -- MDL National Blend of Models products for Puerto Rico region - additional elements
    KWEH,
    /// KWEI -- MDL National Blend of Models products for CONUS and Oceanic regions - additional elements
    KWEI,
    /// KWEJ -- MDL National Blend of Models products for Alaska region - additional elements
    KWEJ,
    /// KWEK -- MDL National Blend of Models products for Hawaii region - additional elements
    KWEK,
    /// KWEL -- MDL National Blend of Models products for Puerto Rico region - additional elements
    KWEL,
    /// KWEM -- MDL National Blend of Models products for Guam region
    KWEM,
    /// KWEN -- MDL National Blend of Models products for Guam region - additional elements
    KWEN,
    /// KWEO -- MDL National Blend of Models products for CONUS and Oceanic regions - additional elements
    KWEO,
    /// KWEP -- MDL National Blend of Models products for Alaska region - additional elements
    KWEP,
    /// KWEQ -- MDL National Blend of Models products for Guam region - additional elements
    KWEQ,
    /// KWER -- MDL National Blend of Models products for Global region
    KWER,
    /// KWES -- MDL Probabilistic Extra-Tropical Storm Surge products for CONUS 625m
    KWES,
    /// KWET -- MDL Probabilistic Extra-Tropical Storm Surge products for CONUS 2.5km
    KWET,
    /// KWEU -- MDL Probabilistic Extra-Tropical Storm Surge products for Alaska 3km
    KWEU,

    /// Some other location indicator
    Other(String),
}

impl From<&str> for Location {
    fn from(s: &str) -> Self {
        match s {
            "KKCI" => Location::KKCI,
            "KMSC" => Location::KMSC,
            "KMWI" => Location::KMWI,
            "KNAS" => Location::KNAS,
            "KNCF" => Location::KNCF,
            "KNEC" => Location::KNEC,
            "KNES" => Location::KNES,
            "KNHC" => Location::KNHC,
            "KNKA" => Location::KNKA,
            "KNWC" => Location::KNWC,
            "KWAT" => Location::KWAT,
            "KPML" => Location::KPML,
            "KWAL" => Location::KWAL,
            "KWBC" => Location::KWBC,
            "KWCO" => Location::KWCO,
            "KWIN" => Location::KWIN,
            "KWNB" => Location::KWNB,
            "KWNC" => Location::KWNC,
            "KWNE" => Location::KWNE,
            "KWNH" => Location::KWNH,
            "KWNJ" => Location::KWNJ,
            "KWNM" => Location::KWNM,
            "KWNO" => Location::KWNO,
            "KWNP" => Location::KWNP,
            "KWNS" => Location::KWNS,
            "KWOH" => Location::KWOH,
            "PAAQ" => Location::PAAQ,
            "PAWU" => Location::PAWU,
            "PGTW" => Location::PGTW,
            "PHEB" => Location::PHEB,

            "KWBD" => Location::KWBD,
            "KWBE" => Location::KWBE,
            "KWBF" => Location::KWBF,
            "KWBG" => Location::KWBG,
            "KWBH" => Location::KWBH,
            "KWBI" => Location::KWBI,
            "KWBJ" => Location::KWBJ,
            "KWBK" => Location::KWBK,
            "KWBL" => Location::KWBL,
            "KWBM" => Location::KWBM,
            "KWBN" => Location::KWBN,
            "KWBO" => Location::KWBO,
            "KWBP" => Location::KWBP,
            "KWBR" => Location::KWBR,
            "KWBS" => Location::KWBS,
            "KWBT" => Location::KWBT,
            "KWBU" => Location::KWBU,
            "KWBV" => Location::KWBV,
            "KWBW" => Location::KWBW,
            "KWBX" => Location::KWBX,
            "KWBZ" => Location::KWBZ,

            "KWBQ" => Location::KWBQ,
            "KWEA" => Location::KWEA,
            "KWEB" => Location::KWEB,
            "KWEC" => Location::KWEC,
            "KWED" => Location::KWED,
            "KWEE" => Location::KWEE,
            "KWEF" => Location::KWEF,
            "KWEG" => Location::KWEG,
            "KWEH" => Location::KWEH,
            "KWEI" => Location::KWEI,
            "KWEJ" => Location::KWEJ,
            "KWEK" => Location::KWEK,
            "KWEL" => Location::KWEL,
            "KWEM" => Location::KWEM,
            "KWEN" => Location::KWEN,
            "KWEO" => Location::KWEO,
            "KWEP" => Location::KWEP,
            "KWEQ" => Location::KWEQ,
            "KWER" => Location::KWER,
            "KWES" => Location::KWES,
            "KWET" => Location::KWET,
            "KWEU" => Location::KWEU,

            other => Location::Other(other.to_string()),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum PFlag {
    /// Standard WMO product heading
    A,
    /// Originating Center's local product identifier (used for images)
    Z,
}

impl ParsedEmwinName {
    /// Parses an EMWIN filename (without the file extension)
    pub fn parse(filename: &str) -> Option<Self> {
        if filename.len() < 18 {
            return None;
        }
        let mut chars = filename.chars();
        let pflag = match chars.next() {
            Some('A') => PFlag::A,
            Some('Z') => PFlag::Z,
            _ => return None,
        };

        // skip underscore
        if !matches!(chars.next(), Some('_')) {
            return None;
        }

        let t1 = chars.next().unwrap();
        let t2 = chars.next().unwrap();

        let aa = &filename[4..6];
        let mut chars = chars.skip(2);

        let (t1, t2, area) = wmo::parse_wmo_abbreviated_heading(t1, t2, aa);

        // next 2 digits are the ii indicators
        let i1 = chars.next().unwrap().to_digit(10).unwrap_or_default();
        let i2 = chars.next().unwrap().to_digit(10).unwrap_or_default();

        let originator = Originator::from_ii(i1 as u8, i2 as u8);

        // next 4 chars are the 4-letter international CCCC code
        let cccc = Location::from(&filename[8..12]);

        // next char is underscore
        // then 'C' to indicate that the originator field is a standard CCCC code
        // then another underscore
        // then "KWIN" originator field

        // next 6 chars are day-of-month, hour, minute, but w e are going to ignore this because we can
        // get a better date from other fields in the filename

        // then a 14-length representing the date:  yyyyMMddhhmmss (UTC i think)
        let date = chrono::NaiveDateTime::parse_from_str(&filename[26..40], "%Y%m%d%H%M%S").ok()?;
        let date = chrono::DateTime::<chrono::Utc>::from_utc(date, chrono::Utc);

        // then underscore
        // then a 6-digit sequence number
        let sequence = (&filename[41..47]).parse::<u32>().ok()?;

        // then underscore
        // then a 1-digit priority, from 1 (highest) to 4 (lowest)
        let priority = match &filename[48..49] {
            "1" => Priority::Highest,
            "2" => Priority::High,
            "3" => Priority::Medium,
            "4" => Priority::Low,
            x => panic!("Unknown priority {}", x),
        };

        // rest of the characters (6) are the old GOES-R product name
        let legacy_filename = filename[50..].to_string();

        Some(ParsedEmwinName {
            pflag,
            data_type_1: t1,
            data_type_2: t2,
            area,
            originator,
            location: cccc,
            date,
            sequence,
            priority,
            legacy_filename,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::emwin::ParsedEmwinName;

    use super::wmo::WMODataTypeT2;

    #[test]
    fn test_parse() {
        let a = ParsedEmwinName::parse("A_ASUS41KPHI041812_C_KWIN_20220504181303_881367-3-RWRPHIPA").unwrap();
        println!("{a:?}");

        let b = ParsedEmwinName::parse("A_FTUS80KWBC040521_C_KWIN_20220504052104_839346-2-TAFALLUS").unwrap();
        println!("{b:?}");

        let c = ParsedEmwinName::parse("A_SXAK58PACR051736_C_KWIN_20220505173627_959486-2-HYDACRAK").unwrap();
        println!("{c:?}");
    }

    #[test]
    #[ignore]
    fn test_unknowns() {
        // let mut set = std::collections::HashSet::new();

        for entry in std::fs::read_dir("/tank/achin/tmp/goes_out3").unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let filename = path.file_name().unwrap().to_str().unwrap();
            if (filename.starts_with("A_") || filename.starts_with("Z_")) && filename.ends_with(".debug") {
                let a = ParsedEmwinName::parse(filename).unwrap();

                match a.data_type_2 {
                    WMODataTypeT2::UnknownAnalyses(_)
                    | WMODataTypeT2::UnknownClimate(_)
                    | WMODataTypeT2::UnknownNotice(_)
                    | WMODataTypeT2::UnknownSatellite(_)
                    | WMODataTypeT2::UnknownUpperAir(_)
                    | WMODataTypeT2::UnknownWarning(_) => {
                        println!("{} {a:?}", filename);
                    }
                    _ => {}
                }
            }
        }

        // println!("{:#?}", set);
    }
}
