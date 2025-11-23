use aho_corasick::AhoCorasick;
use async_trait::async_trait;
use hodei_ports::security::SecretMasker;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MaskingConfig {
    pub enabled: bool,
    pub replacement: String,
    pub patterns: Vec<String>,
}

pub struct AhoCorasickMasker {
    config: MaskingConfig,
    ac: Option<AhoCorasick>,
}

impl AhoCorasickMasker {
    pub fn new(config: MaskingConfig) -> Self {
        let ac = if config.enabled && !config.patterns.is_empty() {
            Some(
                AhoCorasick::new(&config.patterns)
                    .unwrap_or_else(|_| AhoCorasick::new(&["secret"]).unwrap()),
            )
        } else {
            None
        };

        Self { config, ac }
    }
}

#[async_trait]
impl SecretMasker for AhoCorasickMasker {
    async fn mask_text(&self, _source: &str, text: &str) -> String {
        if !self.config.enabled || self.ac.is_none() {
            return text.to_string();
        }

        let ac = self.ac.as_ref().unwrap();
        ac.replace_all(text, &[&self.config.replacement])
    }
}
