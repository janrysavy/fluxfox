//! Version-one snapshot field policy; ordinary public serde stays unchanged.
use super::*;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Value {
    #[serde(deserialize_with = "required_option")]
    scalar: Option<Scalar>,
    repr: Repr,
    state: ValueState,
    #[serde(deserialize_with = "required_option")]
    tip: Option<String>,
    #[serde(deserialize_with = "required_option")]
    comment: Option<String>,
}

impl From<Value> for SourceValue {
    fn from(value: Value) -> Self {
        Self {
            scalar: value.scalar,
            repr: value.repr,
            state: value.state,
            tip: value.tip,
            comment: value.comment,
        }
    }
}

// Nullable fields remain required: missing is not the same as explicit null.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(deserializer)
}

fn tree<'de, D>(deserializer: D) -> Result<FoxTreeMap<SourceValue>, D::Error>
where
    D: Deserializer<'de>,
{
    crate::tree_map::deserialize_tree::<D, SourceValue, Value>(deserializer)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Null {
    #[serde(deserialize_with = "tree")]
    tree: FoxTreeMap<SourceValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Real {
    #[serde(deserialize_with = "tree")]
    map: FoxTreeMap<SourceValue>,
}

pub(super) fn null<'de, D>(deserializer: D) -> Result<NullSourceMap, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(NullSourceMap {
        tree: Null::deserialize(deserializer)?.tree,
    })
}

pub(super) fn real<'de, D>(deserializer: D) -> Result<SourceMap, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(SourceMap {
        map: Real::deserialize(deserializer)?.map,
    })
}
