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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_enabled_config() -> MaskingConfig {
        MaskingConfig {
            enabled: true,
            replacement: "***REDACTED***".to_string(),
            patterns: vec![
                "password".to_string(),
                "secret".to_string(),
                "token".to_string(),
                "api_key".to_string(),
            ],
        }
    }

    fn create_disabled_config() -> MaskingConfig {
        MaskingConfig {
            enabled: false,
            replacement: "***REDACTED***".to_string(),
            patterns: vec![],
        }
    }

    fn create_empty_patterns_config() -> MaskingConfig {
        MaskingConfig {
            enabled: true,
            replacement: "***".to_string(),
            patterns: vec![],
        }
    }

    #[tokio::test]
    async fn test_masker_creation() {
        let config = create_enabled_config();
        let masker = AhoCorasickMasker::new(config);

        assert!(masker.config.enabled);
        assert!(masker.ac.is_some());
    }

    #[tokio::test]
    async fn test_masker_creation_disabled() {
        let config = create_disabled_config();
        let masker = AhoCorasickMasker::new(config);

        assert!(!masker.config.enabled);
        assert!(masker.ac.is_none());
    }

    #[tokio::test]
    async fn test_masker_creation_empty_patterns() {
        let config = create_empty_patterns_config();
        let masker = AhoCorasickMasker::new(config);

        assert!(masker.config.enabled);
        assert!(masker.ac.is_none());
    }

    #[tokio::test]
    async fn test_mask_text_with_password() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "User password is: mysecret123";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
        assert!(!masked.contains("mysecret123"));
    }

    #[tokio::test]
    async fn test_mask_text_with_secret() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "API secret: sk_test_abcdef123456789";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
        assert!(!masked.contains("sk_test_"));
    }

    #[tokio::test]
    async fn test_mask_text_with_token() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "Bearer token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
    }

    #[tokio::test]
    async fn test_mask_text_with_api_key() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "API key: abc123xyz789";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
    }

    #[tokio::test]
    async fn test_mask_text_multiple_patterns() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "password: pass123 and secret: secret456 and token: token789";
        let masked = masker.mask_text("source", input).await;

        let count = masked.matches("***REDACTED***").count();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_mask_text_case_sensitive() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "Password is: PASS123 and SECRET is: SECRET456";
        let masked = masker.mask_text("source", input).await;

        // AhoCorasick is case-sensitive by default
        // Only exact matches should be masked
        assert!(masked.contains("***REDACTED***"));
    }

    #[tokio::test]
    async fn test_mask_text_no_matches() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "This is normal text without any secrets";
        let masked = masker.mask_text("source", input).await;

        assert_eq!(masked, input);
    }

    #[tokio::test]
    async fn test_mask_text_empty_input() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "";
        let masked = masker.mask_text("source", input).await;

        assert_eq!(masked, "");
    }

    #[tokio::test]
    async fn test_mask_text_disabled_config() {
        let masker = AhoCorasickMasker::new(create_disabled_config());

        let input = "password: mypassword123";
        let masked = masker.mask_text("source", input).await;

        assert_eq!(masked, input);
        assert!(masked.contains("password"));
        assert!(masked.contains("mypassword123"));
    }

    #[tokio::test]
    async fn test_mask_text_empty_patterns() {
        let masker = AhoCorasickMasker::new(create_empty_patterns_config());

        let input = "password: secret123";
        let masked = masker.mask_text("source", input).await;

        // Should return unchanged when patterns are empty
        assert_eq!(masked, input);
    }

    #[tokio::test]
    async fn test_mask_text_with_overlapping_patterns() {
        let config = MaskingConfig {
            enabled: true,
            replacement: "[MASKED]".to_string(),
            patterns: vec!["secret".to_string(), "secret_key".to_string()],
        };
        let masker = AhoCorasickMasker::new(config);

        let input = "The secret_key is secret";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("[MASKED]"));
    }

    #[tokio::test]
    async fn test_mask_text_custom_replacement() {
        let config = MaskingConfig {
            enabled: true,
            replacement: "[HIDDEN]".to_string(),
            patterns: vec!["password".to_string()],
        };
        let masker = AhoCorasickMasker::new(config);

        let input = "password: mypass";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("[HIDDEN]"));
        assert!(!masked.contains("***REDACTED***"));
    }

    #[tokio::test]
    async fn test_mask_text_json_like() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = r#"{"password": "secret123", "token": "abc456", "user": "john"}"#;
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
        assert!(!masked.contains("secret123"));
        assert!(!masked.contains("abc456"));
        // User field should remain
        assert!(masked.contains("user"));
        assert!(masked.contains("john"));
    }

    #[tokio::test]
    async fn test_mask_text_url_like() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "https://api.example.com/auth?token=abc123&api_key=xyz789";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
        assert!(!masked.contains("abc123"));
        assert!(!masked.contains("xyz789"));
    }

    #[tokio::test]
    async fn test_mask_text_log_format() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "2025-11-24 10:30:45 INFO User authenticated password=secret123 token=token456";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
        assert!(!masked.contains("secret123"));
        assert!(!masked.contains("token456"));
    }

    #[tokio::test]
    async fn test_mask_text_with_unicode() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "密码: password123 密码是secret";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
    }

    #[tokio::test]
    async fn test_mask_text_very_long_input() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let long_input = "a".repeat(10000) + "password" + &"b".repeat(10000);
        let masked = masker.mask_text("source", &long_input).await;

        assert!(masked.len() >= 20000);
        assert!(masked.contains("***REDACTED***"));
    }

    #[tokio::test]
    async fn test_mask_text_special_characters() {
        let masker = AhoCorasickMasker::new(create_enabled_config());

        let input = "password: p@ssw0rd!#$, secret: s3cr3t@2025!";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("***REDACTED***"));
    }

    #[test]
    fn test_masking_config_structure() {
        let config = create_enabled_config();

        assert!(config.enabled);
        assert!(!config.patterns.is_empty());
        assert_eq!(config.replacement, "***REDACTED***");
    }

    #[test]
    fn test_masking_config_with_single_pattern() {
        let config = MaskingConfig {
            enabled: true,
            replacement: "[REDACTED]".to_string(),
            patterns: vec!["api_key".to_string()],
        };

        assert!(config.enabled);
        assert_eq!(config.patterns.len(), 1);
        assert_eq!(config.patterns[0], "api_key");
    }

    #[test]
    fn test_pattern_creation() {
        let patterns = vec![
            "password".to_string(),
            "secret".to_string(),
            "token".to_string(),
        ];

        let ac = AhoCorasick::new(&patterns);
        assert!(ac.is_ok());
    }

    #[tokio::test]
    async fn test_mask_text_with_numbers_in_pattern() {
        let config = MaskingConfig {
            enabled: true,
            replacement: "[REDACTED]".to_string(),
            patterns: vec!["password123".to_string()],
        };
        let masker = AhoCorasickMasker::new(config);

        let input = "The password123 is: mypass";
        let masked = masker.mask_text("source", input).await;

        assert!(masked.contains("[REDACTED]"));
    }
}
