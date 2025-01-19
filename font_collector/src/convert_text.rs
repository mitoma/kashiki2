use encoding_rs::{DecoderResult, MACINTOSH, UTF_16BE};
use rustybuzz::{
    ttf_parser::{
        fonts_in_collection,
        name::{Name, Names},
        PlatformId,
    },
    Face,
};

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
    match fonts_in_collection(data) {
        Some(count) => (0..count).collect(),
        None => vec![0],
    }
    .into_iter()
    .flat_map(|index| {
        Face::from_slice(data, index)
            .map(|face| get_font_name(&face.names(), NameId::FullFontName, preferred_language))
    })
    .flatten()
    .collect()
}

pub fn get_font_name(
    names: &Names,
    name_id: NameId,
    preferred_language: Option<PreferredLanguage>,
) -> Option<String> {
    let target_record = names
        .into_iter()
        .filter(|name| name.name_id == name_id.into())
        .flat_map(|name| {
            score_encoding(&name, preferred_language)
                .map(|(score, encoding)| (score, encoding, name))
        })
        .max_by(|l, r| l.0.cmp(&r.0));
    if let Some((_, encoding, record)) = target_record {
        decode_name(encoding, record.name)
    } else {
        None
    }
}

#[derive(Debug)]
enum NameEncoding {
    Utf16Be,
    AppleRoman,
}

fn score_encoding(
    name: &Name,
    preferred_language: Option<PreferredLanguage>,
) -> Option<(usize, NameEncoding)> {
    fn match_language_id(language_id: u16, preferred_language: Option<PreferredLanguage>) -> bool {
        preferred_language.is_some_and(|lang| lang.windows_lang_id() == language_id)
    }
    let platform_id = name.platform_id;
    let encoding_id = name.encoding_id;
    let language_id = name.language_id;
    match (platform_id, encoding_id, language_id) {
        // Windows; Unicode full repertoire
        (PlatformId::Windows, 10, _) => Some((1000, NameEncoding::Utf16Be)),

        // Unicode; Unicode full repertoire
        (PlatformId::Unicode, 6, 0) => Some((900, NameEncoding::Utf16Be)),

        // Unicode; Unicode 2.0 and onwards semantics, Unicode full repertoire
        (PlatformId::Unicode, 4, 0) => Some((800, NameEncoding::Utf16Be)),

        // Windows; Unicode BMP
        (PlatformId::Windows, 1, lang) if match_language_id(lang, preferred_language) => {
            Some((1000, NameEncoding::Utf16Be))
        }
        (PlatformId::Windows, 1, 0x409) => Some((750, NameEncoding::Utf16Be)),
        (PlatformId::Windows, 1, lang) if lang != 0x409 => Some((700, NameEncoding::Utf16Be)),

        // Unicode; Unicode 2.0 and onwards semantics, Unicode BMP only
        (PlatformId::Unicode, 3, 0) => Some((600, NameEncoding::Utf16Be)),

        // Unicode; ISO/IEC 10646 semantics
        (PlatformId::Unicode, 2, 0) => Some((500, NameEncoding::Utf16Be)),

        // Unicode; Unicode 1.1 semantics
        (PlatformId::Unicode, 1, 0) => Some((400, NameEncoding::Utf16Be)),

        // Unicode; Unicode 1.0 semantics
        (PlatformId::Unicode, 0, 0) => Some((300, NameEncoding::Utf16Be)),

        // Windows, Symbol
        (PlatformId::Windows, 0, _) => Some((200, NameEncoding::Utf16Be)),

        // Apple Roman
        (PlatformId::Macintosh, 0, 0) => Some((150, NameEncoding::AppleRoman)),
        (PlatformId::Macintosh, 0, lang) if lang != 0 => Some((100, NameEncoding::AppleRoman)),
        _ => None,
    }
}

fn decode_name(encoding: NameEncoding, data: &[u8]) -> Option<String> {
    //convert_u8_to_string(data);

    let mut decoder = match encoding {
        NameEncoding::Utf16Be => UTF_16BE.new_decoder(),
        NameEncoding::AppleRoman => MACINTOSH.new_decoder(),
    };
    if let Some(size) = decoder.max_utf8_buffer_length(data.len()) {
        let mut s = String::with_capacity(size);
        let (res, _read) = decoder.decode_to_string_without_replacement(data, &mut s, true);
        match res {
            DecoderResult::InputEmpty => Some(s),
            DecoderResult::OutputFull => None, // should not happen
            DecoderResult::Malformed(_, _) => None,
        }
    } else {
        None
    }
}
