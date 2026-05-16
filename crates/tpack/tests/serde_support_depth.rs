#![cfg(feature = "serde_support")]

use std::error::Error as StdError;

use tpack::serde_support::ErrorKind;
use tpack::{Limits, Schema, TpackValue, TypeDescriptor};

fn nested_list_schema(levels: usize) -> Schema {
    let mut ty = TypeDescriptor::U8;
    for _ in 0..levels {
        ty = TypeDescriptor::List {
            element: Box::new(ty),
            max_count: None,
        };
    }
    Schema::new(ty)
}

fn nested_list_value(levels: usize) -> TpackValue<'static> {
    let mut value = TpackValue::U8(7);
    for _ in 0..levels {
        value = TpackValue::List(vec![value]);
    }
    value
}

#[test]
fn shallow_value_deserializes_within_depth_limit() {
    let schema = nested_list_schema(2);
    let value = nested_list_value(2);
    let limits = Limits {
        max_depth: 2,
        ..Limits::default()
    };

    let decoded: Vec<Vec<u8>> = tpack::serde_support::Deserializer::new()
        .limits(limits)
        .value(&schema, value)
        .unwrap();

    assert_eq!(decoded, vec![vec![7]]);
}

#[test]
fn deep_value_is_rejected_when_depth_limit_is_exceeded() {
    let schema = nested_list_schema(2);
    let value = nested_list_value(2);
    let limits = Limits {
        max_depth: 1,
        ..Limits::default()
    };

    let error = tpack::serde_support::Deserializer::new()
        .limits(limits)
        .value::<Vec<Vec<u8>>>(&schema, value)
        .unwrap_err();

    assert!(matches!(error.kind(), ErrorKind::DepthLimitExceeded));
    assert_eq!(error.path().to_string(), "/0/0");
}

#[test]
fn from_slice_wraps_core_errors_and_preserves_source_chain() {
    let error = tpack::serde_support::from_slice::<u8>(&[0x54]).unwrap_err();

    assert!(matches!(error.kind(), ErrorKind::Core));
    let source = StdError::source(&error).expect("core error source");
    assert_eq!(source.to_string(), "unexpected end of input");
}
