//! Anomaly detection module

use chrono::{DateTime, Utc};

/// Anomaly detector
pub struct AnomalyDetector;

/// Anomaly score
pub type AnomalyScore = f64;

/// Anomaly detection result
#[derive(Debug, Clone)]
pub struct AnomalyResult {
    pub anomalies: Vec<DetectedAnomaly>,
    pub overall_score: AnomalyScore,
    pub is_anomalous: bool,
}

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct DetectedAnomaly {
    pub anomaly_type: AnomalyType,
    pub score: AnomalyScore,
    pub description: String,
    pub timestamp: DateTime<Utc>,
}

/// Types of anomalies
#[derive(Debug, Clone, PartialEq)]
pub enum AnomalyType {
    VolumeAnomaly,
    TimingAnomaly,
    BehaviorAnomaly,
    ContentAnomaly,
}

impl AnomalyDetector {
    /// Create new anomaly detector
    pub fn new() -> Self {
        Self
    }

    /// Detect anomalies in data
    pub fn detect_anomalies(&self, data: &[f64]) -> AnomalyResult {
        let mut anomalies = Vec::new();
        let mut total_score = 0.0;

        // Simple anomaly detection based on standard deviation
        if data.len() > 1 {
            let mean = data.iter().sum::<f64>() / data.len() as f64;
            let variance = data.iter()
                .map(|x| (x - mean).powi(2))
                .sum::<f64>() / data.len() as f64;
            let std_dev = variance.sqrt();

            // Only proceed if standard deviation is not zero
            if std_dev > 0.0 {
                for (_i, &value) in data.iter().enumerate() {
                    let z_score = (value - mean).abs() / std_dev;
                    if z_score > 1.5 { // Threshold for anomaly (lowered for better detection)
                        let anomaly = DetectedAnomaly {
                            anomaly_type: AnomalyType::VolumeAnomaly,
                            score: (z_score / 3.0).min(1.0), // Normalize to 0-1 range and cap at 1.0
                            description: format!("Value {} deviates significantly from mean", value),
                            timestamp: chrono::Utc::now(),
                        };
                        total_score += anomaly.score;
                        anomalies.push(anomaly);
                    }
                }
            }
        }

        let overall_score = if anomalies.is_empty() {
            0.0
        } else {
            total_score / anomalies.len() as f64
        };

        AnomalyResult {
            anomalies,
            overall_score,
            is_anomalous: overall_score > 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anomaly_detector_creation() {
        let detector = AnomalyDetector::new();
        // Just test that it can be created
        assert!(true);
    }

    #[test]
    fn test_anomaly_result_creation() {
        let result = AnomalyResult {
            anomalies: vec![],
            overall_score: 0.5,
            is_anomalous: false,
        };

        assert_eq!(result.anomalies.len(), 0);
        assert_eq!(result.overall_score, 0.5);
        assert!(!result.is_anomalous);
    }

    #[test]
    fn test_detected_anomaly_creation() {
        let anomaly = DetectedAnomaly {
            anomaly_type: AnomalyType::VolumeAnomaly,
            score: 0.8,
            description: "Test anomaly".to_string(),
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(anomaly.anomaly_type, AnomalyType::VolumeAnomaly);
        assert_eq!(anomaly.score, 0.8);
        assert_eq!(anomaly.description, "Test anomaly");
    }

    #[test]
    fn test_anomaly_type_variants() {
        let types = vec![
            AnomalyType::VolumeAnomaly,
            AnomalyType::TimingAnomaly,
            AnomalyType::BehaviorAnomaly,
            AnomalyType::ContentAnomaly,
        ];

        assert_eq!(types.len(), 4);
    }

    #[test]
    fn test_no_anomalies_in_normal_data() {
        let detector = AnomalyDetector::new();
        let data = vec![1.0, 1.1, 0.9, 1.05, 0.95]; // Normal data

        let result = detector.detect_anomalies(&data);

        assert_eq!(result.anomalies.len(), 0);
        assert!(!result.is_anomalous);
        assert_eq!(result.overall_score, 0.0);
    }

    #[test]
    fn test_anomaly_detection_with_outliers() {
        let detector = AnomalyDetector::new();
        let data = vec![1.0, 1.1, 0.9, 1.05, 10.0]; // One outlier

        let result = detector.detect_anomalies(&data);

        assert!(result.anomalies.len() > 0);
        assert!(result.is_anomalous);
        assert!(result.overall_score > 0.0);
    }

    #[test]
    fn test_empty_data() {
        let detector = AnomalyDetector::new();
        let data = vec![];

        let result = detector.detect_anomalies(&data);

        assert_eq!(result.anomalies.len(), 0);
        assert!(!result.is_anomalous);
        assert_eq!(result.overall_score, 0.0);
    }

    #[test]
    fn test_single_data_point() {
        let detector = AnomalyDetector::new();
        let data = vec![5.0];

        let result = detector.detect_anomalies(&data);

        // Cannot detect anomalies with single data point
        assert_eq!(result.anomalies.len(), 0);
        assert!(!result.is_anomalous);
    }

    #[test]
    fn test_multiple_outliers() {
        let detector = AnomalyDetector::new();
        let data = vec![1.0, 1.1, 10.0, 1.05, 15.0]; // Multiple outliers

        let result = detector.detect_anomalies(&data);

        assert!(result.anomalies.len() >= 2);
        assert!(result.is_anomalous);
        assert!(result.overall_score > 0.5);
    }

    #[test]
    fn test_anomaly_score_range() {
        let detector = AnomalyDetector::new();
        let data = vec![1.0, 1.1, 0.9, 1.05, 10.0];

        let result = detector.detect_anomalies(&data);

        for anomaly in &result.anomalies {
            assert!(anomaly.score >= 0.0 && anomaly.score <= 1.0);
        }

        assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0);
    }
}
