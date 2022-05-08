//! Data structures for parsing WMO data, in particular data from attachment II-5 of WMO manual 386
//!

/// Parse a WMO abbreviated heading
///
/// Within the WMO literature, these are 6 character abbreviations that are often referened
/// with the following fields:
///
///     TTAAii
///
/// # References:
///
/// * https://library.wmo.int/doc_num.php?explnum_id=10469
pub fn parse_wmo_abbreviated_heading(t1: char, t2: char, aa: &str) -> (WMODataTypeT1, WMODataTypeT2, Area) {
    // The first character (T1) indicates data type
    let data_type = WMODataTypeT1::from(t1);

    // The next two characters (T2) depend on the data type
    let data_type_2 = match data_type {
        WMODataTypeT1::Analyses
        | WMODataTypeT1::ClimaticData
        | WMODataTypeT1::Forecasts
        | WMODataTypeT1::Notices
        | WMODataTypeT1::SurfaceData
        | WMODataTypeT1::SatalliteData
        | WMODataTypeT1::UpperAirData
        | WMODataTypeT1::Warnings => {
            // table B1 is used to look up the next data type
            lookup_table_b1(data_type, t2)
        }
        WMODataTypeT1::Pictoral | WMODataTypeT1::PictoralRegional => lookup_table_b6(t2),
        WMODataTypeT1::SatelliteImg => lookup_table_b5(t2),
        x => panic!("Unknow {:?}", x),
    };

    // next is A1 and A2.  This is nominally an area designator, but T1 can adjust
    // this meaning slightly documented in table C1 of WMO manual 386

    let area = match data_type {
        WMODataTypeT1::Analyses
        | WMODataTypeT1::ClimaticData
        | WMODataTypeT1::SatelliteImg
        | WMODataTypeT1::Forecasts
        | WMODataTypeT1::Notices
        | WMODataTypeT1::Warnings => {
            // these types ues table c1 to look up area designator
            let a = AreaDesignator::from_c1(aa).unwrap_or_else(|| {
                panic!(
                    "Unknown area designator: {} for {}{}: {:?} {:?}",
                    aa, t1, t2, data_type, data_type_2
                )
            });
            Area::Area(a)
        }
        WMODataTypeT1::SurfaceData | WMODataTypeT1::UpperAirData => {
            let mut c = aa.chars();
            let a1 = c.next().unwrap();
            let a2 = c.next().unwrap();

            if let Some((a, b)) = lookup_nature_and_area(a1, a2) {
                Area::ReportArea(a, b)
            } else {
                // fall back to table c1
                let a = AreaDesignator::from_c1(aa).unwrap_or_else(|| {
                    panic!(
                        "Unknown area designator: {} for {}{}: {:?} {:?}",
                        aa, t1, t2, data_type, data_type_2
                    )
                });
                Area::Area(a)
            }
        }
        WMODataTypeT1::PictoralRegional | WMODataTypeT1::SatalliteData => {
            let mut c = aa.chars();
            let a1 = c.next().unwrap();
            let a2 = c.next().unwrap();
            let a = GeographicalAreaDesignator::from_c3(a1).unwrap();

            let t = if data_type == WMODataTypeT1::SatalliteData {
                TimeDesignator::from_c4(a2).unwrap()
            } else {
                TimeDesignator::from_c5(a2).unwrap()
            };

            Area::GeoArea(a, t)
        }
        _ => panic!("Unable to lookup area designator for {:?} {}", data_type, t2),
    };

    (data_type, data_type_2, area)
}

fn lookup_nature_and_area(a1: char, a2: char) -> Option<(ReportAreaDesignator, ReportNature)> {
    let nature = match a1 {
        'W' => ReportNature::OceanWeatherStation,
        'V' => ReportNature::MobileShipOrStation,
        'F' => ReportNature::Floats,
        _ => return None,
    };

    let area = match a2 {
        'A' => ReportAreaDesignator::A,
        'B' => ReportAreaDesignator::B,
        'C' => ReportAreaDesignator::C,
        'D' => ReportAreaDesignator::D,
        'E' => ReportAreaDesignator::E,
        'F' => ReportAreaDesignator::F,
        'J' => ReportAreaDesignator::J,
        'X' => ReportAreaDesignator::X,
        _ => return None,
    };

    Some((area, nature))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum WMODataTypeT1 {
    /// Analyses
    ///
    /// T1 Code A
    Analyses,
    /// Addressed message
    ///
    /// T1 Code B
    AddressedMessage,
    /// Climatic data
    ///
    /// T1 Code C
    ClimaticData,
    /// Grid point information (GRID)
    ///
    /// T1 Code D
    GridD,

    /// T1 Satellite imagery
    ///
    /// T1 Code E
    SatelliteImg,

    /// Forecasts
    ///
    /// T1 Code F
    Forecasts,

    /// Notices
    ///
    /// T1 Code N
    Notices,

    /// Pictoral information
    ///
    /// T1 code P
    Pictoral,
    /// Picture information regional
    ///
    /// T1 code Q
    PictoralRegional,

    /// Surface data
    ///
    /// T1 Code S
    SurfaceData,

    /// Satellite data
    ///
    /// T1 Code T
    SatalliteData,

    /// Upper-air data
    ///
    /// T1 Code U
    UpperAirData,
    /// Warning
    ///
    /// T1 Code W
    Warnings,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
/// WMODataTypeT2
///
/// Reference: Table B2
pub enum WMODataTypeT2 {
    // Start of T1=A analysis reference
    /// Code B
    ///
    /// This is not documented by WMO, but NWS uses this for its TPT products
    TemperaturePrecipitationTable,

    // Code C
    CycloneAnalysis,

    /// Air Quality Alert
    ///
    /// Code E
    ///
    /// This is not documented by WMO, but NWS uses this for its AQA products
    AirQualityAlert,

    /// Hydrological/marine
    // Code G
    HydrologicalMarineAnalysis,

    /// Thickness
    // Code H
    Thickness,

    /// Ice
    // Code I
    Ice,

    /// Ozone layer
    // Code O
    Ozone,

    /// Radar
    // Code R
    Radar,

    /// Surface
    // Code S
    SurfaceAnalysis,

    /// Upper air
    // Code U
    UpperAirAnalysis,

    /// Weather summary
    // COde W
    WeatherSummary,

    /// Miscellaneous
    // Code X
    MiscellaneousAnalysis,

    // Start of T1=C reference
    /// Climate anomalies
    // Code A
    ClimateAnomalies,

    /// Climatological Report (Daily)
    ///
    /// Code D
    ///
    /// Not documented by WMO, but NWS uses this
    ClimatologicalReportDaily,

    /// A non-daily climatological report
    /// Code X
    ///
    /// Not documented by WMO, but NWS uses this
    ///
    /// See also: https://www.weather.gov/gyx/f6-key.html
    ClimatologicalReport,

    /// Monthly means (upper air)
    // Code E
    MonthlyMeansUpperAir,

    /// Monthly means (surface)
    // Code H
    MonthlyMeansSurface,

    /// Monthly means (ocean areas)
    // Code O
    MonthlyMeansOceanAreas,

    /// Monthly means (surface)
    // Code S
    MonthlyMeansSurface2,

    // ======= Start of T1=F reference =======
    /// Aviation Area/GAMET/advisories
    // Code A
    AviationAreaAdvisories,

    /// Upper winds and temperatures
    // Code B
    UpperWindsAndTemperatures,

    /// Aerodrome (VT < 12 hours)
    /// Code C
    Aerodrome,

    /// Radiological Trajectory Dose
    /// Code D
    RadiologicalTrajectoryDose,

    /// Extended
    /// Code E
    Extended,

    /// Shipping
    /// Code F
    Shipping,

    /// Hydrological
    /// Code G
    Hydrological,

    /// Upper air thickness
    /// Code H
    UpperAirThickness,

    /// Iceberg
    /// Code I
    Iceberg,

    /// Radio warning service (including IUWDS data)
    /// Code J
    RadioWarningService,

    /// Tropical cylone advisories
    /// Code K
    TropicalCycloneAdvisories,

    /// Local/area
    /// Code L
    LocalArea,

    /// Temperature extremes
    /// Code M
    TemperatureExtremes,

    /// Space weather advisories
    /// Code N
    SpaceWeatherAdvisories,

    /// Guidance
    /// Code O
    Guidance,

    /// Public
    /// Code P
    Public,

    /// Other shipping
    /// Code Q
    OtherShipping,

    /// Aviation route
    /// Code R
    AviationRoute,

    /// Surface
    /// Code S
    SurfaceForecast,

    /// Aerodrome (VT >= 12 hours)
    /// Code T
    Aerodrome12,

    /// Upper air
    /// Code U
    UpperAirForecast,

    /// Volcanic ash advisories
    /// Code V
    VolcanicAshAdvisories,

    /// Winter sports
    /// Code W
    WinterSports,

    /// Miscellaneous
    /// Code X
    MiscellaneousForecast,

    /// Shipping area
    /// Code Z
    ShippingArea,

    // Start of T=N notice reference
    /// Hydrological
    /// Code G
    HydrologicalNotice,

    /// Marine
    /// Code H
    MarineNotice,

    /// Nuclear emergency response
    /// Code N
    NuclearEmergencyResponse,

    /// METNO/WIFMA
    /// Code O
    METNOWIFMANotice,

    /// Product Generation delay
    /// Code P
    ProductGenerationDelay,

    /// Test Msg
    /// Code T
    TestMsg,

    /// Warning related and/or cancellation
    /// Code W
    WarningRelatedCancellation,

    /// Regional Weather Roundup
    ///
    /// Code Z
    ///
    /// This is not documented by WMO, but NWS uses this for its RWR (and, strangly, RWT) products
    RegionalWeatherRoundup,

    // ====== Start of T=S surface data reference
    /// Aviation routine reports
    /// Code A
    AviationRoutineReports,

    /// Radar reports (part A)
    /// Code B
    RadarReportsPartA,

    /// Radar reports (part B)
    /// Code C
    RadarReportsPartB,

    /// Radar reports (parts A & B)
    /// Code D
    RadarReportsPartsAB,

    /// Seismic data
    /// Code E
    SeismicData,

    /// Atmospherics reports
    /// Code F
    AtmosphericsReports,

    /// Radiological data report
    /// Code G
    RadiologicalDataReport,

    /// Reports from DCP stations
    /// Code H
    ReportsFromDCPStations,

    /// Intermediate synoptic hour
    /// Code I
    IntermediateSynopticHour,

    /// (not used)
    /// Code L
    NotUsed,

    /// Main synoptic hour
    /// Code M
    MainSynopticHour,

    /// Non-standard synoptic hour
    /// Code N
    NonStandardSynopticHour,

    /// Oceanographic data
    /// Code O
    OceanographicData,

    /// Special aviation weather reports
    /// Code P
    SpecialAviationWeatherReports,

    /// Hydrological (river) reports
    /// Code R
    HydrologicalRiverReports,

    /// Drifting bouy reports
    /// Code S
    DriftingBouyReports,

    /// Sea ice
    /// Code T
    SeaIce,

    /// Snow depth
    /// Code U
    SnowDepth,

    /// Lake ice
    /// Code V
    LakeIce,

    /// Wave information
    /// Code W
    WaveInformation,

    /// Miscellaneous
    /// Code X
    MiscellaneousSurface,

    /// Seismic waveform data
    /// Code Y
    SeismicWaveformData,

    /// Sea-level data and deep-ocean tsunami data
    /// Code Z
    TsunamiData,

    // ====== Start of T=T satellite data reference
    /// Satellite orbit parameters
    /// Code B
    SatelliteOrbitParameters,

    /// Satellite cloud interpretations
    /// Code C
    SatelliteCloudInterpretations,

    /// Satellite remote upper-air soundings
    /// Code H
    SatelliteRemoteUpperAirSounding,

    /// Clear radiance observations
    /// Code R
    ClearRadianceObservations,

    /// Sea surface temperatures
    /// Code T
    SeaSurfaceTemperatures,

    /// Winds and cloud temperatures
    /// Code W
    WindsAndCloudTemperatures,

    /// Miscellaneous
    /// Code X
    MiscellaneousSatellite,

    // ======= Start of T=U upper-air data reference =======
    /// Aircraft reports (FM 41)
    /// Code A
    AircraftReports41,

    /// Aircraft reports (FM 42)
    /// Code D
    AircraftReports42,

    /// Upper-level pressure, temperature, humidity and wind (Part D)
    /// Code E
    UpperLevelPressureTemperatureHumidityWindPartD,

    /// Upper-level pressure, temperature, humidity and wind (Part C and D)
    ///
    /// National and bilateral option
    ///
    /// Code F
    UpperLevelPressureTemperatureHumidityWindPartCD,

    /// Upper wind (part B)
    /// Code G
    UpperWindPartB,

    /// Upper wind (part C)
    /// Code H
    UpperWindPartC,

    /// Upper wind (parts A and B)
    ///
    /// National and bilateral option
    ///
    /// Code I
    UpperWindPartsAB,

    /// Upper-level pressure, temperature, humidity and wind (Part B)
    /// Code K
    UpperLevelPressureTemperatureHumidityWindPartB,

    /// Upper-level pressure, temperature, humidity and wind (part C)
    /// Code L
    UpperLevelPressureTemperatureHumidityWindPartC,

    /// Upper-level pressure, temperature, humidity and wind (parts A and B)
    ///
    /// National and bilateral option
    ///
    /// Code M
    UpperLevelPressureTemperatureHumidityWindPartsAB,

    /// Rocketsonde reports
    /// Code N
    RocketsondeReports,

    /// Upper wind (part A)
    /// Code P
    UpperWindPartA,

    /// Upper wind (part D)
    /// Code Q
    UpperWindPartD,

    /// Aircraft report
    /// Code R
    AircraftReport,

    /// Upper-level pressure, temperature, humidity and wind (part A)
    /// Code S
    UpperLevelPressureTemperatureHumidityWindPartA,

    /// Aircraft report
    /// Code T
    AircraftReport2,

    /// Miscellaneous
    /// Code X
    MiscellaneousUpperAir,

    /// Upper wind (parts C and D)
    ///
    /// National and bilateral option
    ///
    /// Code Y
    UpperWindPartsCD,

    /// Upper-level pressure, temperature, humidity, and wind from a sonde
    /// released by carrier balloon or aircraft
    ///
    /// (Parts A, B, C and D)
    PTHWFromSonde,

    // ======== Start of T=W warning reference ========
    /// AIRMET
    /// Code A
    AIRMET,

    /// Tropical cyclone (SIGMET)
    /// Code C
    TropicalCyclone,

    /// Tsunami
    /// Code E
    Tsunami,

    /// Tornado
    /// Code F
    Tornado,

    /// Hydrological/river floor
    /// Code G
    HydrologicalRiverFloor,

    /// Marine/coastal flood
    /// Code H
    MarineCoastalFlood,

    /// Other
    /// Code O
    OtherWarning,

    /// Humanitarian activities
    /// Code R
    HumanitarianActivities,

    /// SIGMET
    /// Code S
    SIGMET,

    /// Troical Cyclone (typhoon/hurricane)
    /// Code T
    TropicalCyclone2,

    /// Severe thunderstorm
    /// Code U
    SevereThunderstorm,

    /// Volcanic ash clouds (SIGMET)
    /// Code V
    VolcanicAshClouds,

    /// Warnings and weather summary
    /// Code W
    WarningsAndWeatherSummary,

    // ======= Start of T=P and T=Q reference (table B6) =======
    /// Radar data
    /// Code A
    RadarDataImg,

    /// Cloud
    /// Code B
    CloudImg,

    /// Clear air turbulence
    /// Code C
    ClearAirTurbulenceImg,

    /// Thickness
    /// Code D
    ThicknessImg,

    /// Precipitation
    /// Code E
    PrecipitationImg,

    /// Aerological diagrams (ash cloud)
    /// Code F
    AerologicalDiagramsImg,

    /// Significan weather
    /// Code G
    SignificantWeatherImg,

    /// Height
    /// Code H
    HeightImg,

    /// Ice flow
    /// Code I
    IceFlowImg,

    /// Wave height + combinations
    /// Code J
    WaveHeightCombinationsImg,

    /// Swell height + combinations
    /// Code K
    SwellHeightCombinationsImg,

    /// Plain langauge
    /// Code L
    PlainLanguageImg,

    /// FOr national use
    /// Code M
    NationalUseImg,

    /// Radiation
    /// Code N
    RadiationImg,

    /// Vertical velocity
    /// Code O
    VerticalVelocityImg,

    /// Pressure
    /// Code P
    PressureImg,

    /// Wet bulb potential temperature
    /// Code Q
    WetBulbPotentialTemperatureImg,

    /// Relative humidity
    /// Code R
    RelativeHumidityImg,

    /// SNow cover
    /// Code S
    SnowCoverImg,

    /// Temperature
    /// Code T
    TemperatureImg,

    /// Eastward wind component
    /// Code U
    EastwardWindComponentImg,

    /// Northward wind component
    /// Code V
    NorthwardWindComponentImg,

    /// Wind
    /// Code W
    WindImg,

    /// Lifted index
    /// Code X
    LiftedIndexImg,

    /// Observational plotted chart
    /// Code Y
    ObservationalPlottedChartImg,

    /// Not assigned
    /// Code Z
    NotAssignedImg,

    // ======= Start of T=E reference (table B5) =======
    /// CloudTopTemperature
    /// Code C
    CloudTopTemperatureSatImg,

    /// Fog
    /// Code F
    FogSatImg,

    /// Infrared
    /// Code I
    InfraredSatImg,

    /// Surface temperature
    /// Code S
    SurfaceTemperatureSatImg,

    /// Visible
    /// Code V
    VisibleSatImg,

    /// Water vapour
    /// Code W
    WaterVaporSatImg,

    /// User specified
    /// Code Y
    UserSpecifiedSatImg,

    /// Unspecified
    /// Code Z
    UnspecifiedSatImg,

    // ## Start of unknown T2 codes ##
    // These codes seem to be used by NWS, but are not assigned in WMO manual 386
    UnknownAnalyses(char),
    UnknownClimate(char),
    UnknownNotice(char),
    UnknownUpperAir(char),
    UnknownWarning(char),
    UnknownSatellite(char),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AreaDesignator {
    Albania,
    Argentina,
    Afghanistan,
    AscensionIsland,
    Azerbaijan,
    Alaska,
    Algeria,
    Angola,
    AntiguaAndBarbuda,
    Australia,
    Armenia,
    Azores,

    Bahamas,
    Botswana,
    BruneiDarussalam,
    Bermuda,
    Belize,
    Burundi,
    Benin,
    BanksIslands,
    Myanmar,
    Bahrain,
    Bolivia,
    Barbados,
    Bhutan,
    Bulgaria,
    BouvetIsland,
    Bangladesh,
    Belgium,
    Belarus,
    Brazil,

    Chad,
    CentralAfricanRepublic,
    Congo,
    Chili,
    China,
    Cameroon,
    Canada,
    Columbia,
    CanaryIslands,
    CostaRica,
    CantonIsland,
    Cuba,
    CaboVerde,
    Cyprus,
    Czechia,

    Egypt,
    Eritrea,
    Estonia,
    Ecuador,
    UnitedArabEmirates,
    ElSalvador,
    Ethiopia,

    FaroeIslands,
    FrenchGuiana,
    Finland,
    Fiji,
    FalklandIslands,
    FederatedStatesOfMicronesia,
    SaintPierre,
    France,
    WallisAndFutuna,

    Gambia,
    CaymanIslands,
    Grenada,
    GoughIsland,
    Georgia,
    Ghana,
    Gibraltar,
    Greenland,
    Guam,
    Guinea,
    Gabon,
    EquatorialGuinea,
    Greece,
    Guatemala,
    GuineaBissau,
    Guyana,

    Haiti,
    SaintHelena,
    HongKong,
    Honduras,
    Hungary,
    BurkinaFaso,
    HawaiianIslands,

    CarolineIslands,
    Kiribati,
    ChristmasIsland,
    CocosIslands,
    Kenya,
    /// South Korea
    RepublicOfKorea,
    Cambodia,
    /// North Korea
    DemocraticPeoplesRepublicOfKorea,
    CookIslands,
    Kuwait,
    Kyrgyzstan,
    Kazakhstan,

    Mauritius,
    MarionIsland,
    Morocco,
    Madeira,
    SaintMartin,
    Madagascar,
    MarshallIslands,
    Mali,
    Macedonia,
    Montenegro,
    Malta,
    StMaarten,
    Mongolia,
    Martinique,
    Malaysia,
    Mauritania,
    MacaoChina,
    Maldives,
    Malawi,
    Mexico,
    MarinaIslands,
    Mozambique,

    NewCaledonia,
    Niue,
    PapuaNewGuinea,
    Nigeria,
    Nicaragua,
    Netherlands,
    Namibia,
    Norway,
    Nepal,
    Niger,
    CuraacoAndAruba,
    Vanuatu,
    Nauru,
    NewZealand,

    FrenchPolynesia,
    Philippines,
    PhoenixIslands,
    Pakistan,
    Poland,
    Panama,
    Portugal,
    Palau,
    Peru,
    Pitcairn,
    PuertoRico,
    Paraguay,

    SriLanka,
    Seychelles,
    SaudiArabia,
    Senegal,
    Somalia,
    Sarawak,
    SierraLeone,
    Suriname,
    Sweden,
    SolomonIslands,
    Spain,
    Slovakia,
    Singapore,
    Sudan,
    Swaziland,
    Switzerland,
    SantaCruzIslands,
    SyrianArabRepublic,
    SpitzbergenIslands,

    Tajikistan,
    TristanDaCunha,
    TrinidadAndTobago,
    Togo,
    Thailand,
    TurksAndCaicosIslands,
    Tokelau,
    TimorLeste,
    UnitedRepublicOfTanzania,
    Tonga,
    SaoTomeAndPrincipe,
    Turkmenistan,
    Tunisia,
    Turkey,
    Tuvalu,

    Uganda,
    UnitedKingdom,
    Ukraine,
    UnitedStates,
    Uruguay,
    Uzbekistan,

    Yemen,
    Servia,

    SouthAfrica,
    Zambia,
    Samoa,
    DemocraticRepublicOfTheCongo,
    SouthSudan,
    Zimbabwe,

    // ### Part 2
    AntarcticArea,
    ArcticArea,
    SouthEastAsiaArea,
    AfricaArea,
    CentralAfricaArea,
    WestAfricaArea,
    SouthernAfricaArea,
    AsiaArea,
    NearEastArea,
    ArabianSeaArea,

    CaribbeanAndCentralAmerica,

    EastAfricaArea,
    EastChinaSeaArea,
    EasternEuropeArea,
    MiddleEuropArea,
    NorthernEuropeArea,
    EuropeArea,
    WesternEuropeArea,

    FarEastArea,

    GulfOfAlaskaArea,
    GulfOfMexicoArea,

    IndianOceanArea,

    NorthAmericaArea,
    NorthAtlanticArea,

    PacificArea,
    PersianGulfArea,
    NorthPacificArea,
    WesternNorthPacificArea,
    SouthPacificArea,
    WesternPacificArea,
    EasternPacificArea,

    SouthAmericaArea,
    SouthernOceanArea,
    SeaOfJapanArea,
    SouthChinaseaArea,
    SouthAtlanticArea,

    // unknown areas that are not in the WMO manual but are common in NWS data
    HNUnknown,

    /// An area without a designator
    Unknown,
}

impl AreaDesignator {
    // Table C1 of WMO.386
    pub fn from_c1(s: &str) -> Option<AreaDesignator> {
        Some(match s {
            "AB" => AreaDesignator::Albania,
            "AG" => AreaDesignator::Argentina,
            "AH" => AreaDesignator::Afghanistan,
            "AI" => AreaDesignator::AscensionIsland,
            "AJ" => AreaDesignator::Azerbaijan,
            "AK" => AreaDesignator::Alaska,
            "AL" => AreaDesignator::Algeria,
            "AN" => AreaDesignator::Angola,
            "AT" => AreaDesignator::AntiguaAndBarbuda,
            "AU" => AreaDesignator::Australia,
            "AY" => AreaDesignator::Armenia,
            "AZ" => AreaDesignator::Azores,

            "BA" => AreaDesignator::Bahamas,
            "BC" => AreaDesignator::Botswana,
            "BD" => AreaDesignator::BruneiDarussalam,
            "BE" => AreaDesignator::Bermuda,
            "BH" => AreaDesignator::Belize,
            "BI" => AreaDesignator::Burundi,
            "BJ" => AreaDesignator::Benin,
            "BK" => AreaDesignator::BanksIslands,
            "BM" => AreaDesignator::Myanmar,
            "BN" => AreaDesignator::Bahrain,
            "BO" => AreaDesignator::Bolivia,
            "BR" => AreaDesignator::Barbados,
            "BT" => AreaDesignator::Bhutan,
            "BU" => AreaDesignator::Bulgaria,
            "BV" => AreaDesignator::BouvetIsland,
            "BW" => AreaDesignator::Bangladesh,
            "BX" => AreaDesignator::Belgium,
            "BY" => AreaDesignator::Belarus,
            "BZ" => AreaDesignator::Brazil,

            "CD" => AreaDesignator::Chad,
            "CE" => AreaDesignator::CentralAfricanRepublic,
            "CG" => AreaDesignator::Congo,
            "CH" => AreaDesignator::Chili,
            "CI" => AreaDesignator::China,
            "CM" => AreaDesignator::Cameroon,
            "CN" => AreaDesignator::Canada,
            "CO" => AreaDesignator::Columbia,
            "CR" => AreaDesignator::CanaryIslands,
            "CS" => AreaDesignator::CostaRica,
            "CT" => AreaDesignator::CantonIsland,
            "CU" => AreaDesignator::Cuba,
            "CV" => AreaDesignator::CaboVerde,
            "CY" => AreaDesignator::Cyprus,
            "CZ" => AreaDesignator::Czechia,

            "EG" => AreaDesignator::Egypt,
            "EI" => AreaDesignator::Eritrea,
            "EO" => AreaDesignator::Estonia,
            "EQ" => AreaDesignator::Ecuador,
            "ER" => AreaDesignator::UnitedArabEmirates,
            "ES" => AreaDesignator::ElSalvador,
            "ET" => AreaDesignator::Ethiopia,

            "FA" => AreaDesignator::FaroeIslands,
            "FG" => AreaDesignator::FrenchGuiana,
            "FI" => AreaDesignator::Finland,
            "FJ" => AreaDesignator::Fiji,
            "FK" => AreaDesignator::FalklandIslands,
            "FM" => AreaDesignator::FederatedStatesOfMicronesia,
            "FP" => AreaDesignator::SaintPierre,
            "FR" => AreaDesignator::France,
            "FW" => AreaDesignator::WallisAndFutuna,

            "GB" => AreaDesignator::Gambia,
            "GC" => AreaDesignator::CaymanIslands,
            "GD" => AreaDesignator::Grenada,
            "GE" => AreaDesignator::GoughIsland,
            "GG" => AreaDesignator::Georgia,
            "GH" => AreaDesignator::Ghana,
            "GI" => AreaDesignator::Gibraltar,
            "GL" => AreaDesignator::Greenland,
            "GM" => AreaDesignator::Guam,
            "GN" => AreaDesignator::Guinea,
            "GO" => AreaDesignator::Gabon,
            "GQ" => AreaDesignator::EquatorialGuinea,
            "GR" => AreaDesignator::Greece,
            "GU" => AreaDesignator::Guatemala,
            "GW" => AreaDesignator::GuineaBissau,
            "GY" => AreaDesignator::Guyana,

            "HA" => AreaDesignator::Haiti,
            "HE" => AreaDesignator::SaintHelena,
            "HK" => AreaDesignator::HongKong,
            "HO" => AreaDesignator::Honduras,
            "HU" => AreaDesignator::Hungary,
            "HV" => AreaDesignator::BurkinaFaso,
            "HW" => AreaDesignator::HawaiianIslands,

            "KA" => AreaDesignator::CarolineIslands,
            "KB" => AreaDesignator::Kiribati,
            "KI" => AreaDesignator::ChristmasIsland,
            "KK" => AreaDesignator::CocosIslands,
            "KN" => AreaDesignator::Kenya,
            "KO" => AreaDesignator::RepublicOfKorea,
            "KP" => AreaDesignator::Cambodia,
            "KR" => AreaDesignator::DemocraticPeoplesRepublicOfKorea,
            "KU" => AreaDesignator::CookIslands,
            "KW" => AreaDesignator::Kuwait,
            "KY" => AreaDesignator::Kyrgyzstan,
            "KZ" => AreaDesignator::Kazakhstan,

            "MA" => AreaDesignator::Mauritius,
            "MB" => AreaDesignator::MarionIsland,
            "MC" => AreaDesignator::Morocco,
            "MD" => AreaDesignator::Madeira,
            "MF" => AreaDesignator::SaintMartin,
            "MG" => AreaDesignator::Madagascar,
            "MH" => AreaDesignator::MarshallIslands,
            "MI" => AreaDesignator::Mali,
            "MJ" => AreaDesignator::Macedonia,
            "MK" => AreaDesignator::Montenegro,
            "ML" => AreaDesignator::Malta,
            "MN" => AreaDesignator::StMaarten,
            "MO" => AreaDesignator::Mongolia,
            "MR" => AreaDesignator::Martinique,
            "MS" => AreaDesignator::Malaysia,
            "MT" => AreaDesignator::Mauritania,
            "MU" => AreaDesignator::MacaoChina,
            "MV" => AreaDesignator::Maldives,
            "MW" => AreaDesignator::Malawi,
            "MX" => AreaDesignator::Mexico,
            "MY" => AreaDesignator::MarinaIslands,
            "MZ" => AreaDesignator::Mozambique,

            "NC" => AreaDesignator::NewCaledonia,
            "NE" => AreaDesignator::Niue,
            "NG" => AreaDesignator::PapuaNewGuinea,
            "NI" => AreaDesignator::Nigeria,
            "NK" => AreaDesignator::Nicaragua,
            "NL" => AreaDesignator::Netherlands,
            "NM" => AreaDesignator::Namibia,
            "NO" => AreaDesignator::Norway,
            "NP" => AreaDesignator::Nepal,
            "NR" => AreaDesignator::Niger,
            "NU" => AreaDesignator::CuraacoAndAruba,
            "NV" => AreaDesignator::Vanuatu,
            "NW" => AreaDesignator::Nauru,
            "NZ" => AreaDesignator::NewZealand,

            "PF" => AreaDesignator::FrenchPolynesia,
            "PH" => AreaDesignator::Philippines,
            "PI" => AreaDesignator::PhoenixIslands,
            "PK" => AreaDesignator::Pakistan,
            "PL" => AreaDesignator::Poland,
            "PM" => AreaDesignator::Panama,
            "PO" => AreaDesignator::Portugal,
            "PP" => AreaDesignator::Palau,
            "PR" => AreaDesignator::Peru,
            "PT" => AreaDesignator::Pitcairn,
            "PU" => AreaDesignator::PuertoRico,
            "PY" => AreaDesignator::Paraguay,

            "SB" => AreaDesignator::SriLanka,
            "SC" => AreaDesignator::Seychelles,
            "SD" => AreaDesignator::SaudiArabia,
            "SG" => AreaDesignator::Senegal,
            "SI" => AreaDesignator::Somalia,
            "SK" => AreaDesignator::Sarawak,
            "SL" => AreaDesignator::SierraLeone,
            "SM" => AreaDesignator::Suriname,
            "SN" => AreaDesignator::Sweden,
            "SO" => AreaDesignator::SolomonIslands,
            "SP" => AreaDesignator::Spain,
            "SQ" => AreaDesignator::Slovakia,
            "SR" => AreaDesignator::Singapore,
            "SU" => AreaDesignator::Sudan,
            "SV" => AreaDesignator::Swaziland,
            "SW" => AreaDesignator::Switzerland,
            "SX" => AreaDesignator::SantaCruzIslands,
            "SY" => AreaDesignator::SyrianArabRepublic,
            "SZ" => AreaDesignator::SpitzbergenIslands,

            "TA" => AreaDesignator::Tajikistan,
            "TC" => AreaDesignator::TristanDaCunha,
            "TD" => AreaDesignator::TrinidadAndTobago,
            "TG" => AreaDesignator::Togo,
            "TH" => AreaDesignator::Thailand,
            "TI" => AreaDesignator::TurksAndCaicosIslands,
            "TK" => AreaDesignator::Tokelau,
            "TM" => AreaDesignator::TimorLeste,
            "TN" => AreaDesignator::UnitedRepublicOfTanzania,
            "TO" => AreaDesignator::Tonga,
            "TP" => AreaDesignator::SaoTomeAndPrincipe,
            "TR" => AreaDesignator::Turkmenistan,
            "TS" => AreaDesignator::Tunisia,
            "TU" => AreaDesignator::Turkey,
            "TV" => AreaDesignator::Tuvalu,

            "UG" => AreaDesignator::Uganda,
            "UK" => AreaDesignator::UnitedKingdom,
            "UR" => AreaDesignator::Ukraine,
            "US" => AreaDesignator::UnitedStates,
            "UY" => AreaDesignator::Uruguay,
            "UZ" => AreaDesignator::Uzbekistan,

            "ZA" => AreaDesignator::SouthAfrica,
            "ZB" => AreaDesignator::Zambia,
            "ZM" => AreaDesignator::Samoa,
            "ZR" => AreaDesignator::DemocraticRepublicOfTheCongo,
            "ZS" => AreaDesignator::SouthSudan,
            "ZW" => AreaDesignator::Zimbabwe,

            // ### Part 2
            "AA" => AreaDesignator::AntarcticArea,
            "AC" => AreaDesignator::ArcticArea,
            "AE" => AreaDesignator::SouthEastAsiaArea,
            "AF" => AreaDesignator::AfricaArea,
            "AM" => AreaDesignator::CentralAfricaArea,
            "AO" => AreaDesignator::WestAfricaArea,
            "AP" => AreaDesignator::SouthernAfricaArea,
            "AS" => AreaDesignator::AsiaArea,
            "AW" => AreaDesignator::NearEastArea,
            "AX" => AreaDesignator::ArabianSeaArea,

            "CA" => AreaDesignator::CaribbeanAndCentralAmerica,

            "EA" => AreaDesignator::EastAfricaArea,
            "EC" => AreaDesignator::EastChinaSeaArea,
            "EE" => AreaDesignator::EasternEuropeArea,
            "EM" => AreaDesignator::MiddleEuropArea,
            "EN" => AreaDesignator::NorthernEuropeArea,
            "EU" => AreaDesignator::EuropeArea,
            "EW" => AreaDesignator::WesternEuropeArea,

            "FE" => AreaDesignator::FarEastArea,

            "GA" => AreaDesignator::GulfOfAlaskaArea,
            "GX" => AreaDesignator::GulfOfMexicoArea,

            "IO" => AreaDesignator::IndianOceanArea,

            "NA" => AreaDesignator::NorthAmericaArea,
            "NT" => AreaDesignator::NorthAtlanticArea,

            "PA" => AreaDesignator::PacificArea,
            "PE" => AreaDesignator::PersianGulfArea,
            "PN" => AreaDesignator::NorthPacificArea,
            "PQ" => AreaDesignator::WesternNorthPacificArea,
            "PS" => AreaDesignator::SouthPacificArea,
            "PW" => AreaDesignator::WesternPacificArea,
            "PZ" => AreaDesignator::EasternPacificArea,

            "SA" => AreaDesignator::SouthAmericaArea,
            "SE" => AreaDesignator::SouthernOceanArea,
            "SJ" => AreaDesignator::SeaOfJapanArea,
            "SS" => AreaDesignator::SouthChinaseaArea,
            "ST" => AreaDesignator::SouthAtlanticArea,

            "HN" => AreaDesignator::HNUnknown,

            "XX" => AreaDesignator::Unknown,

            x => {
                println!("Unknown area designator: {}", x);
                return None;
            }
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ReportNature {
    OceanWeatherStation,
    MobileShipOrStation,
    Floats,
}

/// WMO.385 table C2
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReportAreaDesignator {
    /// Area between 30°N–60°S, 35°W–70°E
    A,
    /// Area between 90°N–05°N, 70°E–180°E
    B,
    /// Area between 05°N–60°S, 120°W–35°W
    C,
    /// Area between 90°N–05°N, 180°W–35°W
    D,
    /// Area between 05°N–60°S, 70°E–120°W
    E,
    /// Area between 90°N–30°N, 35°W–70°E
    F,
    /// Area south of 60°S
    J,
    /// More than one area
    X,
}

// Table C3 of WMO.386
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum GeographicalAreaDesignator {
    /// Nothern hemisphere, 0 to 90 degrees West
    NorthernHemisphere_0_90W,

    /// Northern hemisphere, 90 to 180 degrees West
    NorthernHemisphere_90_180W,

    /// Northern hemisphere, 180 to 90 degrees East
    NorthernHemisphere_180_90E,

    /// Northern hemisphere, 90 to 0 degrees East
    NorthernHemisphere_90E_0,

    /// Southern hemisphere, 0 to 90 degrees West
    SouthernHemisphere_0_90W,

    /// Southern hemisphere, 90 to 180 degrees West
    SouthernHemisphere_90_180W,

    /// Southern hemisphere, 180 to 90 degrees East
    SouthernHemisphere_180_90E,

    /// Southern hemisphere, 90 to 0 degrees East
    SouthernHemisphere_90E_E,

    /// Tropical belt, 0 to 90 degrees West
    TropicalBelt_0_90W,

    /// Tropical belt, 90 to 180 degrees West
    TropicalBelt_90_180W,

    /// Tropical belt, 180 to 90 degrees East
    TropicalBelt_180_90E,

    /// Tropical belt, 90 to 0 degrees East
    TropicalBelt_90E_0,

    /// Northern hemispher
    NorthernHemisphere,

    /// Southern hemisphere
    SouthernHemisphere,

    /// Northern hemisphere, 45W to 180
    NorthernHemisphere_45W_180,

    /// An unknown area using code U
    UnknownU,

    /// An unknown area using code P
    UnknownP,

    /// Global area
    GlobalArea,
}

impl GeographicalAreaDesignator {
    pub fn from_c3(c: char) -> Option<GeographicalAreaDesignator> {
        match c {
            'A' => Some(GeographicalAreaDesignator::NorthernHemisphere_0_90W),
            'B' => Some(GeographicalAreaDesignator::NorthernHemisphere_90_180W),
            'C' => Some(GeographicalAreaDesignator::NorthernHemisphere_180_90E),
            'D' => Some(GeographicalAreaDesignator::NorthernHemisphere_90E_0),
            'E' => Some(GeographicalAreaDesignator::TropicalBelt_0_90W),
            'F' => Some(GeographicalAreaDesignator::TropicalBelt_90_180W),
            'G' => Some(GeographicalAreaDesignator::TropicalBelt_180_90E),
            'H' => Some(GeographicalAreaDesignator::TropicalBelt_90E_0),
            'I' => Some(GeographicalAreaDesignator::SouthernHemisphere_0_90W),
            'J' => Some(GeographicalAreaDesignator::SouthernHemisphere_90_180W),
            'K' => Some(GeographicalAreaDesignator::SouthernHemisphere_180_90E),
            'L' => Some(GeographicalAreaDesignator::SouthernHemisphere_90E_E),
            'N' => Some(GeographicalAreaDesignator::NorthernHemisphere),
            'S' => Some(GeographicalAreaDesignator::SouthernHemisphere),
            'T' => Some(GeographicalAreaDesignator::NorthernHemisphere_45W_180),
            'X' => Some(GeographicalAreaDesignator::GlobalArea),
            'U' => Some(GeographicalAreaDesignator::UnknownU),
            'P' => Some(GeographicalAreaDesignator::UnknownP),
            x => panic!("unknown c3: {}", x),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum TimeDesignator {
    Analysis,
    Forecast3Hours,
    Forecast6Hours,
    Forecast9Hours,
    Forecast12Hours,
    Forecast15Hours,
    Forecast18Hours,
    Forecast21Hours,
    Forecast24Hours,
    Forecast27Hours,
    Forecast30Hours,
    Forecast33Hours,
    Forecast36Hours,
    Forecast39Hours,
    Forecast42Hours,
    Forecast45Hours,
    Forecast48Hours,
    Forecast60Hours,
    Forecast72Hours,
    Forecast84Hours,
    Forecast96Hours,
    Forecast108Hours,
    Forecast120Hours,
    Forecast132Hours,
    Forecast144Hours,
    Forecast156Hours,
    Forecast168Hours,
    Forecast10Days,
    Forecast15Days,
    Forecast30Days,
}

impl TimeDesignator {
    pub fn from_c4(c: char) -> Option<TimeDesignator> {
        match c {
            'A' => Some(TimeDesignator::Analysis),
            'B' => Some(TimeDesignator::Forecast6Hours),
            'C' => Some(TimeDesignator::Forecast12Hours),
            'D' => Some(TimeDesignator::Forecast18Hours),
            'E' => Some(TimeDesignator::Forecast24Hours),
            'F' => Some(TimeDesignator::Forecast30Hours),
            'G' => Some(TimeDesignator::Forecast36Hours),
            'H' => Some(TimeDesignator::Forecast42Hours),
            'I' => Some(TimeDesignator::Forecast48Hours),
            'J' => Some(TimeDesignator::Forecast60Hours),
            'K' => Some(TimeDesignator::Forecast72Hours),
            'L' => Some(TimeDesignator::Forecast84Hours),
            'M' => Some(TimeDesignator::Forecast96Hours),
            'N' => Some(TimeDesignator::Forecast108Hours),
            'O' => Some(TimeDesignator::Forecast120Hours),
            'P' => Some(TimeDesignator::Forecast132Hours),
            'Q' => Some(TimeDesignator::Forecast144Hours),
            'R' => Some(TimeDesignator::Forecast156Hours),
            'S' => Some(TimeDesignator::Forecast168Hours),
            'T' => Some(TimeDesignator::Forecast10Days),
            'U' => Some(TimeDesignator::Forecast15Days),
            'V' => Some(TimeDesignator::Forecast30Days),
            x => panic!("unknown C4 time designator: {}", x),
        }
    }
    pub fn from_c5(c: char) -> Option<TimeDesignator> {
        match c {
            'A' => Some(TimeDesignator::Analysis),
            'B' => Some(TimeDesignator::Forecast3Hours),
            'C' => Some(TimeDesignator::Forecast6Hours),
            'D' => Some(TimeDesignator::Forecast9Hours),
            'E' => Some(TimeDesignator::Forecast12Hours),
            'F' => Some(TimeDesignator::Forecast15Hours),
            'G' => Some(TimeDesignator::Forecast18Hours),
            'H' => Some(TimeDesignator::Forecast21Hours),
            'I' => Some(TimeDesignator::Forecast24Hours),
            'J' => Some(TimeDesignator::Forecast27Hours),
            'K' => Some(TimeDesignator::Forecast30Hours),
            'L' => Some(TimeDesignator::Forecast33Hours),
            'M' => Some(TimeDesignator::Forecast36Hours),
            'N' => Some(TimeDesignator::Forecast39Hours),
            'O' => Some(TimeDesignator::Forecast42Hours),
            'P' => Some(TimeDesignator::Forecast45Hours),
            'Q' => Some(TimeDesignator::Forecast48Hours),
            x => panic!("unknown C5 time designator: {}", x),
        }
    }
}

pub fn lookup_table_b1(dt: WMODataTypeT1, t2: char) -> WMODataTypeT2 {
    match dt {
        WMODataTypeT1::Analyses => match t2 {
            'B' => WMODataTypeT2::TemperaturePrecipitationTable,
            'C' => WMODataTypeT2::CycloneAnalysis,
            'E' => WMODataTypeT2::AirQualityAlert,
            'G' => WMODataTypeT2::HydrologicalMarineAnalysis,
            'H' => WMODataTypeT2::Thickness,
            'I' => WMODataTypeT2::Ice,
            'O' => WMODataTypeT2::Ozone,
            'R' => WMODataTypeT2::Radar,
            'S' => WMODataTypeT2::SurfaceAnalysis,
            'U' => WMODataTypeT2::UpperAirAnalysis,
            'W' => WMODataTypeT2::WeatherSummary,
            'X' => WMODataTypeT2::MiscellaneousAnalysis,
            x => WMODataTypeT2::UnknownAnalyses(x),
        },
        WMODataTypeT1::ClimaticData => match t2 {
            'A' => WMODataTypeT2::ClimateAnomalies,
            'D' => WMODataTypeT2::ClimatologicalReportDaily,
            'X' => WMODataTypeT2::ClimatologicalReport,
            'E' => WMODataTypeT2::MonthlyMeansUpperAir,
            'H' => WMODataTypeT2::MonthlyMeansSurface,
            'O' => WMODataTypeT2::MonthlyMeansOceanAreas,
            'S' => WMODataTypeT2::MonthlyMeansSurface2,
            x => WMODataTypeT2::UnknownClimate(x),
        },
        WMODataTypeT1::Forecasts => match t2 {
            'A' => WMODataTypeT2::AviationAreaAdvisories,
            'B' => WMODataTypeT2::UpperWindsAndTemperatures,
            'C' => WMODataTypeT2::Aerodrome,
            'D' => WMODataTypeT2::RadiologicalTrajectoryDose,
            'E' => WMODataTypeT2::Extended,
            'F' => WMODataTypeT2::Shipping,
            'G' => WMODataTypeT2::Hydrological,
            'H' => WMODataTypeT2::UpperAirThickness,
            'I' => WMODataTypeT2::Iceberg,
            'J' => WMODataTypeT2::RadioWarningService,
            'K' => WMODataTypeT2::TropicalCycloneAdvisories,
            'L' => WMODataTypeT2::LocalArea,
            'M' => WMODataTypeT2::TemperatureExtremes,
            'N' => WMODataTypeT2::SpaceWeatherAdvisories,
            'O' => WMODataTypeT2::Guidance,
            'P' => WMODataTypeT2::Public,
            'Q' => WMODataTypeT2::OtherShipping,
            'R' => WMODataTypeT2::AviationRoute,
            'S' => WMODataTypeT2::SurfaceForecast,
            'T' => WMODataTypeT2::Aerodrome12,
            'U' => WMODataTypeT2::UpperAirForecast,
            'V' => WMODataTypeT2::VolcanicAshAdvisories,
            'W' => WMODataTypeT2::WinterSports,
            'X' => WMODataTypeT2::MiscellaneousForecast,
            'Z' => WMODataTypeT2::ShippingArea,
            _ => panic!("Unknown t2 data type {} for t1 {:?}", t2, dt),
        },
        WMODataTypeT1::Notices => match t2 {
            'G' => WMODataTypeT2::Hydrological,
            'H' => WMODataTypeT2::MarineNotice,
            'N' => WMODataTypeT2::NuclearEmergencyResponse,
            'O' => WMODataTypeT2::METNOWIFMANotice,
            'P' => WMODataTypeT2::ProductGenerationDelay,
            'T' => WMODataTypeT2::TestMsg,
            'W' => WMODataTypeT2::WarningRelatedCancellation,
            'Z' => WMODataTypeT2::RegionalWeatherRoundup,
            x => WMODataTypeT2::UnknownNotice(x),
            // _ => panic!("Unknown t2 data type {} for t1 {:?}", t2, dt)
        },
        WMODataTypeT1::SurfaceData => match t2 {
            'A' => WMODataTypeT2::AviationRoutineReports,
            'B' => WMODataTypeT2::RadarReportsPartA,
            'C' => WMODataTypeT2::RadarReportsPartB,
            'D' => WMODataTypeT2::RadarReportsPartsAB,
            'E' => WMODataTypeT2::SeismicData,
            'F' => WMODataTypeT2::AtmosphericsReports,
            'G' => WMODataTypeT2::RadiologicalDataReport,
            'H' => WMODataTypeT2::ReportsFromDCPStations,
            'I' => WMODataTypeT2::IntermediateSynopticHour,
            'L' => WMODataTypeT2::NotUsed,
            'M' => WMODataTypeT2::MainSynopticHour,
            'N' => WMODataTypeT2::NonStandardSynopticHour,
            'O' => WMODataTypeT2::OceanographicData,
            'P' => WMODataTypeT2::SpecialAviationWeatherReports,
            'R' => WMODataTypeT2::HydrologicalRiverReports,
            'S' => WMODataTypeT2::DriftingBouyReports,
            'T' => WMODataTypeT2::SeaIce,
            'U' => WMODataTypeT2::SnowDepth,
            'V' => WMODataTypeT2::LakeIce,
            'W' => WMODataTypeT2::WaveInformation,
            'X' => WMODataTypeT2::MiscellaneousSurface,
            'Y' => WMODataTypeT2::SeismicWaveformData,
            'Z' => WMODataTypeT2::TsunamiData,
            _ => panic!("Unknown t2 data type {} for t1 {:?}", t2, dt),
        },
        WMODataTypeT1::SatalliteData => match t2 {
            'B' => WMODataTypeT2::SatelliteOrbitParameters,
            'C' => WMODataTypeT2::SatelliteCloudInterpretations,
            'H' => WMODataTypeT2::SatelliteRemoteUpperAirSounding,
            'R' => WMODataTypeT2::ClearRadianceObservations,
            'T' => WMODataTypeT2::SeaSurfaceTemperatures,
            'W' => WMODataTypeT2::WindsAndCloudTemperatures,
            'X' => WMODataTypeT2::MiscellaneousSatellite,
            x => WMODataTypeT2::UnknownSatellite(x),
        },
        WMODataTypeT1::UpperAirData => match t2 {
            'A' => WMODataTypeT2::AircraftReports41,
            'D' => WMODataTypeT2::AircraftReports42,
            'E' => WMODataTypeT2::UpperLevelPressureTemperatureHumidityWindPartD,
            'F' => WMODataTypeT2::UpperLevelPressureTemperatureHumidityWindPartCD,
            'G' => WMODataTypeT2::UpperWindPartB,
            'H' => WMODataTypeT2::UpperWindPartC,
            'I' => WMODataTypeT2::UpperWindPartsAB,
            'K' => WMODataTypeT2::UpperLevelPressureTemperatureHumidityWindPartB,
            'L' => WMODataTypeT2::UpperLevelPressureTemperatureHumidityWindPartC,
            'M' => WMODataTypeT2::UpperLevelPressureTemperatureHumidityWindPartsAB,
            'N' => WMODataTypeT2::RocketsondeReports,
            'P' => WMODataTypeT2::UpperWindPartA,
            'Q' => WMODataTypeT2::UpperWindPartD,
            'R' => WMODataTypeT2::AircraftReport,
            'S' => WMODataTypeT2::UpperLevelPressureTemperatureHumidityWindPartA,
            'T' => WMODataTypeT2::AircraftReport2,
            'X' => WMODataTypeT2::MiscellaneousUpperAir,
            'Y' => WMODataTypeT2::UpperWindPartsCD,
            'Z' => WMODataTypeT2::PTHWFromSonde,
            x => WMODataTypeT2::UnknownUpperAir(x),
        },
        WMODataTypeT1::Warnings => match t2 {
            'A' => WMODataTypeT2::AIRMET,
            'C' => WMODataTypeT2::TropicalCyclone,
            'E' => WMODataTypeT2::Tsunami,
            'F' => WMODataTypeT2::Tornado,
            'G' => WMODataTypeT2::HydrologicalRiverFloor,
            'H' => WMODataTypeT2::MarineCoastalFlood,
            'O' => WMODataTypeT2::OtherWarning,
            'R' => WMODataTypeT2::HumanitarianActivities,
            'S' => WMODataTypeT2::SIGMET,
            'T' => WMODataTypeT2::TropicalCyclone2,
            'U' => WMODataTypeT2::SevereThunderstorm,
            'V' => WMODataTypeT2::VolcanicAshClouds,
            'W' => WMODataTypeT2::WarningRelatedCancellation,
            _ => panic!("Unknown t2 data type {} for t1 {:?}", t2, dt),
        },
        _ => panic!("Unknown t2 data type {} for t1 {:?}", t2, dt),
    }
}

pub fn lookup_table_b6(t2: char) -> WMODataTypeT2 {
    match t2 {
        'A' => WMODataTypeT2::RadarDataImg,
        'B' => WMODataTypeT2::CloudImg,
        'C' => WMODataTypeT2::ClearAirTurbulenceImg,
        'D' => WMODataTypeT2::ThicknessImg,
        'E' => WMODataTypeT2::PrecipitationImg,
        'F' => WMODataTypeT2::AerologicalDiagramsImg,
        'G' => WMODataTypeT2::SignificantWeatherImg,
        'H' => WMODataTypeT2::HeightImg,
        'I' => WMODataTypeT2::IceFlowImg,
        'J' => WMODataTypeT2::WaveHeightCombinationsImg,
        'K' => WMODataTypeT2::SwellHeightCombinationsImg,
        'L' => WMODataTypeT2::PlainLanguageImg,
        'M' => WMODataTypeT2::NationalUseImg,
        'N' => WMODataTypeT2::RadarDataImg,
        'O' => WMODataTypeT2::VerticalVelocityImg,
        'P' => WMODataTypeT2::PressureImg,
        'Q' => WMODataTypeT2::WetBulbPotentialTemperatureImg,
        'R' => WMODataTypeT2::RelativeHumidityImg,
        'S' => WMODataTypeT2::SnowCoverImg,
        'T' => WMODataTypeT2::TemperatureImg,
        'U' => WMODataTypeT2::EastwardWindComponentImg,
        'V' => WMODataTypeT2::NorthwardWindComponentImg,
        'W' => WMODataTypeT2::WindImg,
        'X' => WMODataTypeT2::LiftedIndexImg,
        'Y' => WMODataTypeT2::ObservationalPlottedChartImg,
        'Z' => WMODataTypeT2::NotAssignedImg,
        _ => panic!("Unknown table b5 t2 data type {}", t2),
    }
}

pub fn lookup_table_b5(t2: char) -> WMODataTypeT2 {
    match t2 {
        'C' => WMODataTypeT2::CloudTopTemperatureSatImg,
        'F' => WMODataTypeT2::FogSatImg,
        'I' => WMODataTypeT2::InfraredSatImg,
        'S' => WMODataTypeT2::SurfaceTemperatureSatImg,
        'V' => WMODataTypeT2::VisibleSatImg,
        'W' => WMODataTypeT2::WaterVaporSatImg,
        'Y' => WMODataTypeT2::UserSpecifiedSatImg,
        'Z' => WMODataTypeT2::UnspecifiedSatImg,
        _ => panic!("Unknown table b5 t2 data type {}", t2),
    }
}

impl From<char> for WMODataTypeT1 {
    fn from(c: char) -> Self {
        match c {
            'A' => WMODataTypeT1::Analyses,
            'B' => WMODataTypeT1::AddressedMessage,
            'C' => WMODataTypeT1::ClimaticData,
            'D' => WMODataTypeT1::GridD,
            'F' => WMODataTypeT1::Forecasts,
            'N' => WMODataTypeT1::Notices,
            'S' => WMODataTypeT1::SurfaceData,
            'T' => WMODataTypeT1::SatalliteData,
            'U' => WMODataTypeT1::UpperAirData,
            'W' => WMODataTypeT1::Warnings,
            'P' => WMODataTypeT1::Pictoral,
            'Q' => WMODataTypeT1::PictoralRegional,
            'E' => WMODataTypeT1::SatelliteImg,
            x => panic!("Unknown WMO data type {}", x),
        }
    }
}

#[derive(Debug)]
pub enum Area {
    Area(AreaDesignator),
    GeoArea(GeographicalAreaDesignator, TimeDesignator),
    /// The area from which a report was generated
    ///
    /// Used for bulletins containing ship's weather reports and oceanographic data including reports from
    /// automatic marine stations.
    ReportArea(ReportAreaDesignator, ReportNature),
}
