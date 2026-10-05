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
    // Independent public-native baseline: unchanged pinned dependency, not
    // this snapshot module. Full raw receipt lives in the parent's evidence.
    // Pin the complete fixture (canonical LF) and the original probe product.
    // The parent's SHA-256-pinned native receipt independently binds both trees
    // and this product, rather than trusting the fixture's own metadata.
    let fixture = include_str!("native_baseline.json").replace("\r\n", "\n");
    assert_eq!(
        sha1_smol::Sha1::from(fixture.as_bytes()).digest().to_string(),
        "678c80f6464f3410c818f1b34450e7a98750b708"
    );
    let baseline: Value = serde_json::from_str(&fixture).unwrap();
    assert_eq!(
        baseline["native_product_sha256"],
        "aea2daab87c13b31b6a9e562fc2225c05a8d5ac468ca5a83667c7e00308a496e"
    );
    assert_eq!(
        baseline["dependency_commit"],
        "5a1fb83656c3b6f933d9f91198dcca90658bfd26"
    );
    assert_eq!(before["map"]["Tree"], baseline["source_map"]["real_before"]);
    let native_tree = &before["map"]["Tree"]["map"];
    assert_eq!(native_tree["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(native_tree["name_to_index"]["duplicate"], 3);
    assert_eq!(native_tree["nodes"][1]["data"]["scalar"], json!({"U32":0x12345678}));
    assert_eq!(native_tree["nodes"][1]["data"]["repr"], json!({"Hex":8}));
    assert_eq!(native_tree["nodes"][1]["data"]["state"], "Questionable");
    assert_eq!(native_tree["nodes"][1]["data"]["tip"], "native tip");
    assert_eq!(native_tree["nodes"][1]["data"]["comment"], "native comment");
    let saved: SourceMapSnapshot = serde_json::from_value(before.clone()).unwrap();
    let mut next = round_trip(&image);
    let mut peer = image.clone().prepare_source_map_restore(&saved).unwrap();
    assert_eq!(wire(&next), before);
    assert!(image.source_map().as_any().is::<SourceMap>());
    assert!(next.source_map().as_any().is::<SourceMap>());
    assert!(peer.source_map().as_any().is::<SourceMap>());
    assert_eq!(wire(&peer), before);
    assert_eq!(next.source_map().as_some().unwrap().children(0), &[1, 3]);
    fn continue_native(image: &mut DiskImage) {
        image
            .source_map_mut()
            .last_node()
            .add_child("resume", SourceValue::u8(0x5a))
            .add_child("grandchild", SourceValue::u32(42))
            .up()
            .add_sibling("sibling", SourceValue::u16(0x6b));
    }
    continue_native(&mut next);
    assert_eq!(wire(&image), before); // restored Box/tree is independent
    assert_eq!(wire(&peer), before); // two candidates from the same saved state
    continue_native(&mut image);
    assert_eq!(wire(&next), wire(&image));
    continue_native(&mut peer);
    assert_eq!(wire(&peer), wire(&image));
    assert_eq!(next.source_map().as_some().unwrap().node(4).0, "resume");
    assert_eq!(next.source_map().as_some().unwrap().node(5).0, "grandchild");
    assert_eq!(next.source_map().as_some().unwrap().node(6).0, "sibling");
    assert_eq!(next.source_map().as_some().unwrap().children(3), &[4, 6]);
    assert_eq!(wire(&next)["map"]["Tree"], baseline["source_map"]["real_after"]);
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
    assert_eq!(null.source_map_mut().last_node().index(), 0); // pinned native null cursor reacquires root
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
    let mut restored_empty = round_trip(&empty);
    assert_eq!(wire(&restored_empty)["map"]["Tree"]["map"]["nodes"], json!([]));
    // Storage-only native Default is not promoted to a valid indexed tree.
    // DELIBERATE native failures establish unchanged before/after behavior.
    println!("SOURCE_MAP: DELIBERATE native empty-tree last_node panic, original and restored");
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        empty.source_map_mut().last_node();
    }))
    .is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        restored_empty.source_map_mut().last_node();
    }))
    .is_err());
    assert_eq!(wire(&restored_empty), wire(&empty));
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

#[test]
fn source_map_impossible_empty_null_owner_refused_before_installation() {
    let live = DiskImage::default();
    let before = wire(&live);
    let mut impossible = before.clone();
    impossible["map"]["Null"]["tree"]["nodes"] = json!([]);
    impossible["map"]["Null"]["tree"]["name_to_index"] = json!({});
    let saved: SourceMapSnapshot = serde_json::from_value(impossible).unwrap();
    assert!(live.clone().prepare_source_map_restore(&saved).is_err());
    assert_eq!(wire(&live), before);
    assert!(live.source_map().as_any().is::<NullSourceMap>());
    let mut invalid_capture = live.clone();
    invalid_capture.source_map = Some(Box::new(NullSourceMap {
        tree: FoxTreeMap::default(),
    }));
    assert!(invalid_capture.snapshot_source_map().is_err());
    println!("SOURCE_MAP: impossible empty null owner refused on capture and restore; live owner unchanged");
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

#[test]
fn source_map_nested_schema_rejects_unknown_and_missing_fields_with_explicit_nulls() {
    let mut real = DiskImage::default();
    real.assign_source_map(true);
    real.source_map_mut().add_child(0, "child", SourceValue::u8(3));
    let null = DiskImage::default();
    for (image, pointers) in [
        (
            &real,
            vec![
                "/map/Tree",
                "/map/Tree/map",
                "/map/Tree/map/nodes/0",
                "/map/Tree/map/nodes/0/data",
            ],
        ),
        (
            &null,
            vec![
                "/map/Null",
                "/map/Null/tree",
                "/map/Null/tree/nodes/0",
                "/map/Null/tree/nodes/0/data",
            ],
        ),
    ] {
        let good = wire(image);
        let _: SourceMapSnapshot = serde_json::from_value(good.clone()).unwrap();
        for pointer in pointers {
            let mut extra = good.clone();
            extra
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unknown_field".into(), json!(1));
            assert!(
                serde_json::from_value::<SourceMapSnapshot>(extra).is_err(),
                "unknown at {pointer}"
            );
            let fields: Vec<_> = good
                .pointer(pointer)
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            for field in fields {
                let mut missing = good.clone();
                missing
                    .pointer_mut(pointer)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(&field);
                assert!(
                    serde_json::from_value::<SourceMapSnapshot>(missing).is_err(),
                    "missing {pointer}/{field}"
                );
            }
        }
    }
    let root = wire(&real);
    for field in ["scalar", "tip", "comment"] {
        assert!(root["map"]["Tree"]["map"]["nodes"][0]["data"][field].is_null());
    }
    println!("SOURCE_MAP: nested native owner/tree/node/value schema refuses unknown and omitted fields; explicit nulls pass");
}

#[test]
fn source_map_ordinary_serde_retains_pinned_native_field_tolerance() {
    // Unchanged 5a1fb836 independently accepted these four unknown locations
    // and all three omitted nullable keys; snapshot strictness is separate.
    let good = serde_json::to_value(SourceMap::new()).unwrap();
    for pointer in ["", "/map", "/map/nodes/0", "/map/nodes/0/data"] {
        let mut extra = good.clone();
        extra
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("future_field".into(), json!(7));
        let read: SourceMap = serde_json::from_value(extra).unwrap();
        assert_eq!(serde_json::to_value(read).unwrap(), good);
    }
    for field in ["scalar", "tip", "comment"] {
        let mut missing = good.clone();
        missing
            .pointer_mut("/map/nodes/0/data")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        let read: SourceMap = serde_json::from_value(missing).unwrap();
        assert_eq!(serde_json::to_value(read).unwrap(), good);
    }
    println!("SOURCE_MAP: ordinary serde retains four native unknown-field and three nullable omission behaviors; snapshot remains strict");
}

#[test]
fn source_map_native_serde_renamed_root_preserved_but_impossible_null_root_refused() {
    // Original public SourceMap Deserialize accepts a renamed root and its
    // native add_child consumer works: measured on unchanged5a1fb836. Root is
    // index0, not a reserved lookup key; a child may also be named "root".
    let mut renamed = serde_json::to_value(SourceMap::new()).unwrap();
    renamed["map"]["nodes"][0]["name"] = json!("renamed-root");
    renamed["map"]["name_to_index"] = json!({"renamed-root":0});
    let mut real = DiskImage::default();
    real.source_map = Some(Box::new(serde_json::from_value::<SourceMap>(renamed).unwrap()));
    let before = wire(&real);
    let mut next = round_trip(&real);
    assert_eq!(wire(&next), before);
    assert_eq!(next.source_map().as_some().unwrap().node(0).0, "renamed-root");
    real.source_map_mut().add_child(0, "root", SourceValue::u8(5));
    next.source_map_mut().add_child(0, "root", SourceValue::u8(5));
    assert_eq!(wire(&next), wire(&real));
    assert_eq!(wire(&next)["map"]["Tree"]["map"]["name_to_index"]["root"], 1);
    // Original NullSourceMap has no public Deserialize and its constructor/API
    // cannot rename its root. Refuse this impossible null payload only.
    let live = DiskImage::default();
    let original = wire(&live);
    let mut impossible = original.clone();
    impossible["map"]["Null"]["tree"]["nodes"][0]["name"] = json!("renamed-root");
    impossible["map"]["Null"]["tree"]["name_to_index"] = json!({"renamed-root":0});
    let saved: SourceMapSnapshot = serde_json::from_value(impossible).unwrap();
    assert!(live.clone().prepare_source_map_restore(&saved).is_err());
    assert_eq!(wire(&live), original);
    println!("SOURCE_MAP: measured native serde renamed real root preserved; impossible null root refused; child named root allowed");
}
