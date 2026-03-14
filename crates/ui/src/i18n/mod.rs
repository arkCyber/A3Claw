pub mod strings;

mod en;
mod zh;
mod tw;
mod ja;
mod ko;
mod es;
mod fr;
mod de;
mod pt;
mod ru;
mod ar;
mod hi;
mod it;
mod nl;
mod tr;
mod pl;

use crate::theme::Language;
use strings::Strings;

/// Returns the `Strings` instance for the given language.
pub fn strings_for(lang: Language) -> &'static Strings {
    match lang {
        Language::En   => &en::EN,
        Language::ZhCn => &zh::ZH,
        Language::ZhTw => &tw::TW,
        Language::Ja   => &ja::JA,
        Language::Ko   => &ko::KO,
        Language::Es   => &es::ES,
        Language::Fr   => &fr::FR,
        Language::De   => &de::DE,
        Language::Pt   => &pt::PT,
        Language::Ru   => &ru::RU,
        Language::Ar   => &ar::AR,
        Language::Hi   => &hi::HI,
        Language::It   => &it::IT,
        Language::Nl   => &nl::NL,
        Language::Tr   => &tr::TR,
        Language::Pl   => &pl::PL,
    }
}
