//! Pattern matching module

/// Pattern matcher
pub struct PatternMatcher;

/// Threat pattern
pub struct ThreatPattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub pattern: String,
    pub description: String,
}

/// Types of patterns
#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    Regex,
    Substring,
    Hash,
    Domain,
    Ip,
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub pattern_id: String,
    pub matched_text: String,
    pub location: MatchLocation,
    pub confidence: f64,
}

/// Location of a match
#[derive(Debug, Clone)]
pub struct MatchLocation {
    pub start: usize,
    pub end: usize,
    pub field: String,
}

impl PatternMatcher {
    /// Create new pattern matcher
    pub fn new() -> Self {
        Self
    }

    /// Match patterns in text
    pub fn match_patterns(&self, text: &str, patterns: &[ThreatPattern]) -> Vec<PatternMatch> {
        let mut matches = Vec::new();

        for pattern in patterns {
            if let Some(pattern_match) = self.match_single_pattern(text, pattern) {
                matches.push(pattern_match);
            }
        }

        matches
    }

    /// Match a single pattern
    fn match_single_pattern(&self, text: &str, pattern: &ThreatPattern) -> Option<PatternMatch> {
        match pattern.pattern_type {
            PatternType::Substring => {
                if let Some(pos) = text.find(&pattern.pattern) {
                    Some(PatternMatch {
                        pattern_id: pattern.id.clone(),
                        matched_text: pattern.pattern.clone(),
                        location: MatchLocation {
                            start: pos,
                            end: pos + pattern.pattern.len(),
                            field: "text".to_string(),
                        },
                        confidence: 0.8,
                    })
                } else {
                    None
                }
            }
            _ => {
                // TODO: Implement other pattern types
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pattern() -> ThreatPattern {
        ThreatPattern {
            id: "test-pattern-1".to_string(),
            pattern_type: PatternType::Substring,
            pattern: "malicious".to_string(),
            description: "Test malicious pattern".to_string(),
        }
    }

    #[test]
    fn test_pattern_matcher_creation() {
        let matcher = PatternMatcher::new();
        // Just test that it can be created
        assert!(true);
    }

    #[test]
    fn test_threat_pattern_creation() {
        let pattern = create_test_pattern();

        assert_eq!(pattern.id, "test-pattern-1");
        assert_eq!(pattern.pattern_type, PatternType::Substring);
        assert_eq!(pattern.pattern, "malicious");
        assert_eq!(pattern.description, "Test malicious pattern");
    }

    #[test]
    fn test_pattern_type_variants() {
        let types = vec![
            PatternType::Regex,
            PatternType::Substring,
            PatternType::Hash,
            PatternType::Domain,
            PatternType::Ip,
        ];

        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_match_location_creation() {
        let location = MatchLocation {
            start: 10,
            end: 20,
            field: "subject".to_string(),
        };

        assert_eq!(location.start, 10);
        assert_eq!(location.end, 20);
        assert_eq!(location.field, "subject");
    }

    #[test]
    fn test_pattern_match_creation() {
        let pattern_match = PatternMatch {
            pattern_id: "test-pattern".to_string(),
            matched_text: "malicious".to_string(),
            location: MatchLocation {
                start: 0,
                end: 9,
                field: "body".to_string(),
            },
            confidence: 0.9,
        };

        assert_eq!(pattern_match.pattern_id, "test-pattern");
        assert_eq!(pattern_match.matched_text, "malicious");
        assert_eq!(pattern_match.confidence, 0.9);
    }

    #[test]
    fn test_substring_pattern_matching() {
        let matcher = PatternMatcher::new();
        let pattern = create_test_pattern();
        let text = "This is a malicious email";

        let matches = matcher.match_patterns(text, &[pattern]);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "malicious");
        assert_eq!(matches[0].location.start, 10);
        assert_eq!(matches[0].location.end, 19);
    }

    #[test]
    fn test_no_pattern_match() {
        let matcher = PatternMatcher::new();
        let pattern = create_test_pattern();
        let text = "This is a clean email";

        let matches = matcher.match_patterns(text, &[pattern]);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_multiple_patterns() {
        let matcher = PatternMatcher::new();

        let patterns = vec![
            ThreatPattern {
                id: "pattern-1".to_string(),
                pattern_type: PatternType::Substring,
                pattern: "malicious".to_string(),
                description: "Malicious pattern".to_string(),
            },
            ThreatPattern {
                id: "pattern-2".to_string(),
                pattern_type: PatternType::Substring,
                pattern: "suspicious".to_string(),
                description: "Suspicious pattern".to_string(),
            },
        ];

        let text = "This malicious and suspicious email";
        let matches = matcher.match_patterns(text, &patterns);

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_case_sensitive_matching() {
        let matcher = PatternMatcher::new();
        let pattern = create_test_pattern();
        let text = "This is a MALICIOUS email";

        let matches = matcher.match_patterns(text, &[pattern]);

        // Should not match due to case sensitivity
        assert_eq!(matches.len(), 0);
    }
}
