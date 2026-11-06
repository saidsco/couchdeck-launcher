//! Small Fluent-based localization layer.
//!
//! German and English are bundled into the binary. English is the fallback for
//! missing German messages. Automatic language detection reads the usual
//! POSIX/Linux locale environment variables.

use std::env;
use std::fmt;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

const DE_FTL: &str = include_str!("../assets/i18n/de.ftl");
const EN_FTL: &str = include_str!("../assets/i18n/en.ftl");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LanguagePreference {
    #[default]
    Auto,
    De,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    De,
    En,
}

impl Language {
    pub fn id(self) -> &'static str {
        match self {
            Self::De => "de",
            Self::En => "en",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::De => "Deutsch",
            Self::En => "English",
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

#[derive(Debug, Clone)]
pub enum ArgValue {
    Text(String),
    Int(i64),
    UInt(u64),
}

impl From<&str> for ArgValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for ArgValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<i64> for ArgValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<u64> for ArgValue {
    fn from(value: u64) -> Self {
        Self::UInt(value)
    }
}

impl From<usize> for ArgValue {
    fn from(value: usize) -> Self {
        Self::UInt(value as u64)
    }
}

impl From<u32> for ArgValue {
    fn from(value: u32) -> Self {
        Self::UInt(u64::from(value))
    }
}

impl From<i32> for ArgValue {
    fn from(value: i32) -> Self {
        Self::Int(i64::from(value))
    }
}

pub struct I18n {
    preference: LanguagePreference,
    language: Language,
    bundle: FluentBundle<FluentResource>,
    fallback: Option<FluentBundle<FluentResource>>,
}

impl I18n {
    pub fn new(preference: LanguagePreference) -> Self {
        let language = resolve_language(preference, detected_locale().as_deref());
        let (bundle, fallback) = build_bundles(language);
        Self {
            preference,
            language,
            bundle,
            fallback,
        }
    }

    pub fn preference(&self) -> LanguagePreference {
        self.preference
    }

    pub fn language(&self) -> Language {
        self.language
    }

    pub fn set_preference(&mut self, preference: LanguagePreference) {
        self.preference = preference;
        let language = resolve_language(preference, detected_locale().as_deref());
        let (bundle, fallback) = build_bundles(language);
        self.language = language;
        self.bundle = bundle;
        self.fallback = fallback;
    }

    pub fn text(&self, key: &str) -> String {
        self.format(key, &[])
    }

    pub fn format(&self, key: &str, values: &[(&str, ArgValue)]) -> String {
        let mut args = FluentArgs::new();
        for (name, value) in values {
            let value = match value {
                ArgValue::Text(text) => FluentValue::from(text.clone()),
                ArgValue::Int(number) => FluentValue::from(*number),
                ArgValue::UInt(number) => FluentValue::from(*number as i64),
            };
            args.set((*name).to_owned(), value);
        }
        self.format_in(&self.bundle, key, &args)
            .or_else(|| {
                self.fallback
                    .as_ref()
                    .and_then(|bundle| self.format_in(bundle, key, &args))
            })
            .unwrap_or_else(|| key.to_owned())
    }

    fn format_in(
        &self,
        bundle: &FluentBundle<FluentResource>,
        key: &str,
        args: &FluentArgs,
    ) -> Option<String> {
        let message = bundle.get_message(key)?;
        let pattern = message.value()?;
        let mut errors = Vec::new();
        let text = bundle
            .format_pattern(pattern, Some(args), &mut errors)
            .to_string();
        // A partially rendered message is still better than a panic; missing
        // variables are visible in the text and are caught by development.
        Some(text)
    }
}

fn build_bundles(
    language: Language,
) -> (
    FluentBundle<FluentResource>,
    Option<FluentBundle<FluentResource>>,
) {
    let main = build_bundle(language, language_resource(language));
    let fallback = if language == Language::De {
        Some(build_bundle(Language::En, EN_FTL))
    } else {
        None
    };
    (main, fallback)
}

fn language_resource(language: Language) -> &'static str {
    match language {
        Language::De => DE_FTL,
        Language::En => EN_FTL,
    }
}

fn build_bundle(language: Language, source: &str) -> FluentBundle<FluentResource> {
    let locale: LanguageIdentifier = language.id().parse().expect("valid bundled locale");
    let resource = FluentResource::try_new(source.to_owned()).unwrap_or_else(|errors| {
        panic!(
            "embedded Fluent resource for {} is invalid: {errors:?}",
            language.id()
        )
    });
    let mut bundle = FluentBundle::new(vec![locale]);
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .unwrap_or_else(|errors| panic!("cannot add Fluent resource: {errors:?}"));
    bundle
}

fn detected_locale() -> Option<String> {
    ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|key| env::var(key).ok().filter(|value| !value.is_empty()))
}

fn resolve_language(preference: LanguagePreference, locale: Option<&str>) -> Language {
    match preference {
        LanguagePreference::De => Language::De,
        LanguagePreference::En => Language::En,
        LanguagePreference::Auto => {
            let locale = locale.unwrap_or_default().to_ascii_lowercase();
            if locale
                .split([':', '.', '@'])
                .next()
                .unwrap_or("")
                .starts_with("de")
            {
                Language::De
            } else {
                Language::En
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_german_and_english_with_english_fallback() {
        assert_eq!(
            resolve_language(LanguagePreference::Auto, Some("de_DE.UTF-8")),
            Language::De
        );
        assert_eq!(
            resolve_language(LanguagePreference::Auto, Some("de:en")),
            Language::De
        );
        assert_eq!(
            resolve_language(LanguagePreference::Auto, Some("en_US.UTF-8")),
            Language::En
        );
        assert_eq!(
            resolve_language(LanguagePreference::Auto, Some("fr_FR.UTF-8")),
            Language::En
        );
        assert_eq!(
            resolve_language(LanguagePreference::De, Some("en_US")),
            Language::De
        );
        assert_eq!(
            resolve_language(LanguagePreference::En, Some("de_DE")),
            Language::En
        );
    }

    #[test]
    fn formats_both_languages_and_switches_without_restart() {
        let mut i18n = I18n::new(LanguagePreference::De);
        assert_eq!(i18n.language(), Language::De);
        let german = i18n.text("common-back");
        assert!(!german.is_empty());
        i18n.set_preference(LanguagePreference::En);
        assert_eq!(i18n.language(), Language::En);
        let english = i18n.text("common-back");
        assert!(!english.is_empty());
        assert_ne!(german, english);
        let formatted = i18n.format("time-remaining", &[("minutes", 12u64.into())]);
        assert!(formatted.contains("12"));
    }

    #[test]
    fn missing_german_message_falls_back_to_english() {
        // Deliberately present only in en.ftl.
        let i18n = I18n::new(LanguagePreference::De);
        assert_eq!(i18n.text("zz-fallback-test"), "English fallback works");
    }
}
