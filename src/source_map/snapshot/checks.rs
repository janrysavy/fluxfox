use super::*;
use serde_json::{json, Value};

fn round_trip(image: &DiskImage) -> DiskImage {
    let saved: SourceMapSnapshot =
        serde_json::from_slice(&serde_json::to_vec(&image.snapshot_source_map().unwrap()).unwrap()).unwrap();
    let candidate: DiskImage = serde_json::from_slice(&serde_json::to_vec(image).unwrap()).unwrap();
    assert!(candidate.source_map.is_none()); // independently observed native skip
    candidate.prepare_source_map_restore(&saved).unwrap()
}
fn wire(image: &DiskImage) -> Value {
    serde_json::to_value(image.snapshot_source_map().unwrap()).unwrap()
}

#[test]
fn source_map_native_tree_restores_values_duplicate_names_and_append_continuation() {
    let mut image = DiskImage::default();
    image.assign_source_map(true);
    image
        .source_map_mut()
        .add_child(
            0,
            "duplicate",
            SourceValue::u32_base(0x12345678, Repr::Hex(8), ValueState::Questionable, "native tip")
                .comment("native comment"),
        )
        .add_child("leaf", SourceValue::string("exact string"));
    image
        .source_map_mut()
        .add_child(0, "duplicate", SourceValue::u16(0x2468).bin(16).bad());
    let before = wire(&image);
    let native_tree = &before["map"]["Tree"]["map"];
    assert_eq!(native_tree["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(native_tree["name_to_index"]["duplicate"], 3);
    assert_eq!(native_tree["nodes"][1]["data"]["scalar"], json!({"U32":0x12345678}));
    assert_eq!(native_tree["nodes"][1]["data"]["repr"], json!({"Hex":8}));
    assert_eq!(native_tree["nodes"][1]["data"]["state"], "Questionable");
    assert_eq!(native_tree["nodes"][1]["data"]["tip"], "native tip");
    assert_eq!(native_tree["nodes"][1]["data"]["comment"], "native comment");
    let mut next = round_trip(&image);
    assert_eq!(wire(&next), before);
    assert_eq!(next.source_map().as_some().unwrap().children(0), &[1, 3]);
    next.source_map_mut()
        .last_node()
        .add_child("resume", SourceValue::u8(0x5a));
    assert_eq!(wire(&image), before); // the restored Box/tree is independent
    image
        .source_map_mut()
        .last_node()
        .add_child("resume", SourceValue::u8(0x5a));
    assert_eq!(wire(&next), wire(&image));
    assert_eq!(next.source_map().as_some().unwrap().node(4).0, "resume");
    println!(
        "SOURCE_MAP: native tree values/lookup preserved; independent JSON restore resumes identical cursor append"
    );
}

#[test]
fn source_map_absent_null_hidden_tree_and_native_empty_default_stay_distinct() {
    let mut absent = DiskImage::default();
    absent.source_map = None;
    assert_eq!(wire(&round_trip(&absent))["map"], "Absent");
    let mut null = DiskImage::default();
    assert!(null.source_map().as_any().is::<NullSourceMap>());
    null.source_map_mut().add_child(0, "ignored", SourceValue::u8(9));
    assert_eq!(wire(&null)["map"]["Null"]["tree"]["nodes"].as_array().unwrap().len(), 1);
    // Native null add_child is a no-op, but its returned/public cursor can add.
    null.source_map_mut()
        .last_node()
        .add_child("hidden", SourceValue::u8(7));
    let original = wire(&null);
    let mut next = round_trip(&null);
    assert!(next.source_map().as_any().is::<NullSourceMap>() && next.source_map().as_some().is_none());
    assert_eq!(wire(&next), original);
    next.source_map_mut()
        .last_node()
        .add_sibling("continued", SourceValue::u8(8));
    assert_eq!(wire(&null), original);
    null.source_map_mut()
        .last_node()
        .add_sibling("continued", SourceValue::u8(8));
    assert_eq!(wire(&next), wire(&null));
    let mut empty = DiskImage::default();
    empty.source_map = Some(Box::new(SourceMap::default()));
    assert_eq!(wire(&round_trip(&empty))["map"]["Tree"]["map"]["nodes"], json!([]));
    println!("SOURCE_MAP: None/Null/real/derived-empty owners preserved; null cursor's nonempty hidden tree retained");
}

#[test]
fn source_map_invalid_tree_and_schema_refused_without_mutating_original() {
    let mut image = DiskImage::default();
    image.assign_source_map(true);
    image
        .source_map_mut()
        .add_child(0, "child", SourceValue::u8(1))
        .add_child("grandchild", SourceValue::u8(2));
    let good = wire(&image);
    for case in 0..8 {
        let mut bad = good.clone();
        let tree = &mut bad["map"]["Tree"]["map"];
        match case {
            0 => tree["nodes"][1]["index"] = json!(99),
            1 => tree["nodes"][1]["parent"] = json!(99),
            2 => tree["nodes"][0]["children"] = json!([99]),
            3 => tree["nodes"][0]["children"] = json!([1, 1]),
            4 => tree["nodes"][2]["parent"] = json!(0),
            5 => tree["name_to_index"]["child"] = json!(2),
            6 => {
                tree["nodes"][0]["children"] = json!([]);
                tree["nodes"][1]["parent"] = json!(2);
                tree["nodes"][2]["children"] = json!([1]);
            }
            _ => bad["version"] = json!(2),
        }
        let saved: SourceMapSnapshot = serde_json::from_value(bad).unwrap();
        assert!(
            image.clone().prepare_source_map_restore(&saved).is_err(),
            "invalid tree case{case}"
        );
        assert_eq!(wire(&image), good);
    }
    for key in ["version", "map"] {
        let mut bad = good.clone();
        bad.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<SourceMapSnapshot>(bad).is_err());
    }
    let mut extra = good.clone();
    extra["extra"] = json!(1);
    assert!(serde_json::from_value::<SourceMapSnapshot>(extra).is_err());
    let mut unknown = good;
    unknown["map"] = json!({"Unknown":{}});
    assert!(serde_json::from_value::<SourceMapSnapshot>(unknown).is_err());
}

#[derive(Clone)]
struct UnknownOwner(NullSourceMap);
impl OptionalSourceMap for UnknownOwner {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_some(&self) -> Option<&SourceMap> {
        None
    }
    fn add_child(&mut self, parent: usize, name: &str, data: SourceValue) -> FoxTreeCursor<SourceValue> {
        self.0.add_child(parent, name, data)
    }
    fn debug_tree(&self) {
        self.0.debug_tree();
    }
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        OptionalSourceMap::fmt(&self.0, f)
    }
    fn last_node(&mut self) -> FoxTreeCursor<SourceValue> {
        self.0.last_node()
    }
}
#[test]
fn source_map_unknown_owner_is_refused_instead_of_normalized_to_null() {
    let mut image = DiskImage::default();
    image.source_map = Some(Box::new(UnknownOwner(NullSourceMap::new())));
    assert!(image.source_map().as_some().is_none());
    assert!(matches!(
        image.snapshot_source_map(),
        Err(DiskImageError::UnsupportedOperation(_))
    ));
    assert!(image.source_map().as_any().is::<UnknownOwner>());
}
