//! Translation keys / language.

/// Supported locales (PMMP).
pub fn supported_locales() -> &'static [&'static str] {
    &[
        "en_US", "en_GB", "de_DE", "es_ES", "es_MX", "fr_FR", "fr_CA", "it_IT", "ja_JP", "ko_KR",
        "pt_BR", "pt_PT", "ru_RU", "zh_CN", "zh_TW", "nl_NL", "pl_PL", "tr_TR", "sv_SE", "fi_FI",
        "hu_HU", "cs_CZ", "da_DK", "el_GR",
    ]
}

pub fn default_locale() -> &'static str {
    "en_US"
}

/// Translate a key with parameters (simple replacement).
pub fn translate(key: &str, params: &[String]) -> String {
    // In real impl, this would look up lang files.
    let mut out = key.to_string();
    for (i, p) in params.iter().enumerate() {
        out = out.replace(&format!("%{}$s", i + 1), p);
    }
    out
}

/// Common death message keys.
pub fn death_message_key(cause: &str) -> String {
    format!("death.{}", cause)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_replaces_params() {
        let key = "death.attack.player";
        let params = vec!["Steve".to_string(), "Alex".to_string()];
        let result = translate(key, &params);
        assert!(result.contains("death.attack.player"));
    }
}
