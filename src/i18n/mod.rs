mod en_us;
mod zh_cn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    ZhCn,
    EnUs,
}

impl Locale {
    pub fn from_language_code(code: &str) -> Self {
        match code {
            "en-US" => Self::EnUs,
            _ => Self::ZhCn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct I18n {
    locale: Locale,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    pub fn tr(&self, key: &'static str) -> &'static str {
        match self.locale {
            Locale::ZhCn => zh_cn::tr(key),
            Locale::EnUs => en_us::tr(key),
        }
    }

    pub fn tr_fmt(&self, key: &'static str, args: &[(&str, &str)]) -> String {
        let mut out = self.tr(key).to_string();
        for (k, v) in args {
            let pattern = format!("{{{}}}", k);
            out = out.replace(&pattern, v);
        }
        out
    }
}
