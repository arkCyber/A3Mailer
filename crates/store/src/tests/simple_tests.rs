/*
 * SPDX-FileCopyrightText: 2020 Stalwart Labs Ltd <hello@stalw.art>
 *
 * SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-SEL
 */

//! # Simple Store Tests
//!
//! Basic tests for the store module functionality

use crate::{backend::memory::StaticMemoryStore, Value};
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_memory_store_creation() {
        let store = StaticMemoryStore::default();
        // StaticMemoryStore is a GlobMap, test basic creation
        let value = store.get("nonexistent");
        assert!(value.is_none());
    }

    #[test]
    fn test_static_memory_store_basic_operations() {
        let mut store = StaticMemoryStore::default();

        // Test insertion
        let key = "test_key";
        let value = Value::Text("test_value".into());
        store.insert(key, value.clone());

        // Test retrieval
        let retrieved = store.get(key);
        assert!(retrieved.is_some());

        // Test that we can retrieve the value
        if let Some(retrieved_value) = retrieved {
            match (retrieved_value, &value) {
                (Value::Text(retrieved_text), Value::Text(original_text)) => {
                    assert_eq!(retrieved_text.as_ref(), original_text.as_ref());
                }
                _ => panic!("Value types don't match"),
            }
        }
    }

    #[test]
    fn test_static_memory_store_different_value_types() {
        let mut store = StaticMemoryStore::default();

        // Test string value
        store.insert("string_key", Value::Text("hello".into()));

        // Test integer value
        store.insert("int_key", Value::Integer(42.into()));

        // Test float value
        store.insert("float_key", Value::Float(3.14.into()));

        // Verify all values can be retrieved
        assert!(store.get("string_key").is_some());
        assert!(store.get("int_key").is_some());
        assert!(store.get("float_key").is_some());

        // Test value types
        match store.get("string_key") {
            Some(Value::Text(_)) => {},
            _ => panic!("Expected Text value"),
        }

        match store.get("int_key") {
            Some(Value::Integer(_)) => {},
            _ => panic!("Expected Integer value"),
        }

        match store.get("float_key") {
            Some(Value::Float(_)) => {},
            _ => panic!("Expected Float value"),
        }
    }

    #[test]
    fn test_static_memory_store_overwrite() {
        let mut store = StaticMemoryStore::default();
        let key = "overwrite_key";

        // Insert first value
        store.insert(key, Value::Text("first".into()));

        // Verify first value
        match store.get(key) {
            Some(Value::Text(text)) => assert_eq!(text.as_ref(), "first"),
            _ => panic!("Expected first value"),
        }

        // Overwrite with second value
        store.insert(key, Value::Text("second".into()));

        // Verify the value was overwritten
        match store.get(key) {
            Some(Value::Text(text)) => assert_eq!(text.as_ref(), "second"),
            _ => panic!("Expected second value"),
        }
    }

    #[test]
    fn test_static_memory_store_nonexistent_key() {
        let store = StaticMemoryStore::default();

        // Test getting non-existent key
        let result = store.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_static_memory_store_multiple_keys() {
        let mut store = StaticMemoryStore::default();

        // Add some data
        store.insert("key1", Value::Text("value1".into()));
        store.insert("key2", Value::Text("value2".into()));
        store.insert("key3", Value::Text("value3".into()));

        // Verify all keys can be retrieved
        assert!(store.get("key1").is_some());
        assert!(store.get("key2").is_some());
        assert!(store.get("key3").is_some());

        // Verify values are correct
        match store.get("key1") {
            Some(Value::Text(text)) => assert_eq!(text.as_ref(), "value1"),
            _ => panic!("Expected value1"),
        }
    }

    #[test]
    fn test_static_memory_store_glob_patterns() {
        let mut store = StaticMemoryStore::default();

        // Test exact match
        store.insert("exact_key", Value::Text("exact_value".into()));
        assert!(store.get("exact_key").is_some());

        // Test glob pattern (StaticMemoryStore supports glob patterns)
        store.insert("prefix_*", Value::Text("glob_value".into()));

        // This should match the glob pattern
        let result = store.get("prefix_test");
        assert!(result.is_some());

        match result {
            Some(Value::Text(text)) => assert_eq!(text.as_ref(), "glob_value"),
            _ => panic!("Expected glob_value"),
        }
    }

    #[test]
    fn test_static_memory_store_performance() {
        let mut store = StaticMemoryStore::default();
        let num_operations = 100;

        // Test write performance
        let start = Instant::now();
        for i in 0..num_operations {
            let key = format!("key_{}", i);
            let value = Value::Integer(i.into());
            store.insert(&key, value);
        }
        let write_duration = start.elapsed();

        // Write performance should be reasonable (less than 100ms for 100 operations)
        assert!(write_duration < Duration::from_millis(100),
                "Write performance too slow: {:?}", write_duration);

        // Test read performance
        let start = Instant::now();
        for i in 0..num_operations {
            let key = format!("key_{}", i);
            let _value = store.get(&key);
        }
        let read_duration = start.elapsed();

        // Read performance should be reasonable (less than 50ms for 100 operations)
        assert!(read_duration < Duration::from_millis(50),
                "Read performance too slow: {:?}", read_duration);
    }
}
