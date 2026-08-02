use read_fonts::TableProvider;
use skrifa::FontRef;

// Platform IDs (OpenType spec values)
const PLATFORM_UNICODE: u16 = 0;
const PLATFORM_MACINTOSH: u16 = 1;
const PLATFORM_WINDOWS: u16 = 3;

#[derive(Debug, Clone, Copy)]
pub enum PreferredLanguage {
    Japanese,
    UnitedStates,
}

#[derive(Debug, Clone, Copy)]
pub enum NameId {
    CopyrightNotice,
    FontFamilyName,
    FontSubfamilyName,
    UniqueFontIdentifier,
    FullFontName,
    VersionString,
    PostscriptName,
    Trademark,
    ManufacturerName,
    Designer,
    Description,
    UrlVendor,
    UrlDesigner,
    LicenseDescription,
    LicenseInfoUrl,
    TypographicFamilyName,
    TypographicSubfamilyName,
    CompatibleFull,
    SampleText,
    PostscriptCidFindfontName,
    WwsFamilyName,
    WwsSubfamilyName,
    LightBackgroundPalette,
    DarkBackgroundPalette,
    VariationsPostscriptNamePrefix,
}

impl From<NameId> for u16 {
    fn from(value: NameId) -> Self {
        match value {
            NameId::CopyrightNotice => 0,
            NameId::FontFamilyName => 1,
            NameId::FontSubfamilyName => 2,
            NameId::UniqueFontIdentifier => 3,
            NameId::FullFontName => 4,
            NameId::VersionString => 5,
            NameId::PostscriptName => 6,
            NameId::Trademark => 7,
            NameId::ManufacturerName => 8,
            NameId::Designer => 9,
            NameId::Description => 10,
            NameId::UrlVendor => 11,
            NameId::UrlDesigner => 12,
            NameId::LicenseDescription => 13,
            NameId::LicenseInfoUrl => 14,
            /* 15 is Reserved */
            NameId::TypographicFamilyName => 16,
            NameId::TypographicSubfamilyName => 17,
            NameId::CompatibleFull => 18,
            NameId::SampleText => 19,
            NameId::PostscriptCidFindfontName => 20,
            NameId::WwsFamilyName => 21,
            NameId::WwsSubfamilyName => 22,
            NameId::LightBackgroundPalette => 23,
            NameId::DarkBackgroundPalette => 24,
            NameId::VariationsPostscriptNamePrefix => 25,
        }
    }
}

impl PreferredLanguage {
    fn windows_lang_id(&self) -> u16 {
        match self {
            PreferredLanguage::Japanese => 1041,
            PreferredLanguage::UnitedStates => 1033,
        }
    }
}

pub fn font_name(data: &[u8], preferred_language: Option<PreferredLanguage>) -> Vec<String> {
    FontRef::fonts(data)
        .flat_map(|font_result| {
            font_result
                .ok()
                .and_then(|font| get_font_name(&font, NameId::FullFontName, preferred_language))
        })
        .collect()
}

pub fn get_font_name(
    font: &FontRef,
    name_id: NameId,
    preferred_language: Option<PreferredLanguage>,
) -> Option<String> {
    let name_table = font.name().ok()?;
    let string_data = name_table.string_data();
    let target_name_id: u16 = name_id.into();

    let target_record = name_table
        .name_record()
        .iter()
        .filter(|record| record.name_id().to_u16() == target_name_id)
        .flat_map(|record| {
            score_name_record(record, preferred_language).map(|score| (score, record))
        })
        .max_by_key(|(score, _)| *score);

    target_record.and_then(|(_, record)| record.string(string_data).ok().map(|ns| ns.to_string()))
}

fn score_name_record(
    record: &read_fonts::tables::name::NameRecord,
    preferred_language: Option<PreferredLanguage>,
) -> Option<usize> {
    fn match_language_id(language_id: u16, preferred_language: Option<PreferredLanguage>) -> bool {
        preferred_language.is_some_and(|lang| lang.windows_lang_id() == language_id)
    }
    let platform_id = record.platform_id();
    let encoding_id = record.encoding_id();
    let language_id = record.language_id();
    match (platform_id, encoding_id, language_id) {
        // Windows; Unicode full repertoire
        (PLATFORM_WINDOWS, 10, _) => Some(1000),

        // Unicode; Unicode full repertoire
        (PLATFORM_UNICODE, 6, 0) => Some(900),

        // Unicode; Unicode 2.0 and onwards semantics, Unicode full repertoire
        (PLATFORM_UNICODE, 4, 0) => Some(800),

        // Windows; Unicode BMP (preferred language match)
        (PLATFORM_WINDOWS, 1, lang) if match_language_id(lang, preferred_language) => Some(1000),
        (PLATFORM_WINDOWS, 1, 0x409) => Some(750),
        (PLATFORM_WINDOWS, 1, lang) if lang != 0x409 => Some(700),

        // Unicode; Unicode 2.0 and onwards semantics, Unicode BMP only
        (PLATFORM_UNICODE, 3, 0) => Some(600),

        // Unicode; ISO/IEC 10646 semantics
        (PLATFORM_UNICODE, 2, 0) => Some(500),

        // Unicode; Unicode 1.1 semantics
        (PLATFORM_UNICODE, 1, 0) => Some(400),

        // Unicode; Unicode 1.0 semantics
        (PLATFORM_UNICODE, 0, 0) => Some(300),

        // Windows, Symbol
        (PLATFORM_WINDOWS, 0, _) => Some(200),

        // Apple Roman
        (PLATFORM_MACINTOSH, 0, 0) => Some(150),
        (PLATFORM_MACINTOSH, 0, lang) if lang != 0 => Some(100),
        _ => None,
    }
}
