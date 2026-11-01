//! Single scalar fields count against a `.fig` message's value budget (upstream FigCraft a80df8ed
//! and 1a454edd, audit #1057 FMT-1): before, a message of nothing but scalar fields decoded any
//! number of them whatever its budget, so a small file could expand into a huge tree.

use super::kiwi::{Def, Field, Kind, Schema, Type, value_budget};

#[test]
fn scalar_fields_count_against_the_value_budget() {
    // A message of 1,000 scalar fields (id 1, `true`) is 1,001 values: a budget of 10 refuses it.
    let fields = vec![Field { name: "b".into(), ty: Some(Type::Bool), array: false, value: 1 }];
    let sc = Schema { defs: vec![Def { name: "Message".into(), kind: Kind::Message, fields }] };
    let mut data = [1u8, 1].repeat(1000);
    data.push(0);
    let e = sc.decode_message_within(0, &data, 10).unwrap_err();
    assert_eq!(e.0, "message: too many values");
    assert!(sc.decode_message_within(0, &data, 1001).is_ok());
}

#[test]
fn the_budget_grows_four_values_per_compressed_byte_between_its_bounds() {
    // Counting scalar fields too, dense real files reach about 2.4 values per compressed byte.
    let floor = value_budget(0);
    assert_eq!(value_budget(1), floor);
    let big = floor; // a canvas this many bytes packed gets four times as many values
    assert_eq!(value_budget(big), (big * 4).min(value_budget(usize::MAX)));
    assert!(value_budget(usize::MAX) >= value_budget(big));
}
