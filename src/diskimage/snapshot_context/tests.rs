use super::*;
use crate::{image_builder::ImageBuilder, prelude::*};

fn formatted() -> DiskImage {
    ImageBuilder::new()
        .with_resolution(TrackDataResolution::BitStream)
        .with_standard_format(StandardFormat::PcFloppy360)
        .with_formatted(true)
        .build()
        .unwrap()
}
fn json_images(images: &[&DiskImage]) -> Vec<DiskImage> {
    images
        .iter()
        .map(|image| serde_json::from_slice(&serde_json::to_vec(*image).unwrap()).unwrap())
        .collect()
}
fn saved(images: &[&DiskImage]) -> DiskContextSnapshot {
    serde_json::from_slice(&serde_json::to_vec(&DiskImage::snapshot_contexts(images).unwrap()).unwrap()).unwrap()
}
fn graph(images: &[&DiskImage]) -> Vec<Vec<Option<usize>>> {
    let mut identities = BTreeMap::new();
    images
        .iter()
        .map(|image| {
            slots(image)
                .unwrap()
                .iter()
                .map(|slot| {
                    slot.context.as_ref().map(|arc| {
                        let address = Arc::as_ptr(arc) as usize;
                        let next = identities.len();
                        *identities.entry(address).or_insert(next)
                    })
                })
                .collect()
        })
        .collect()
}

#[test]
fn context_json_recovers_lost_counter_and_cross_image_alias_without_live_mutation() {
    let mut first = formatted();
    let ch = DiskCh::new(0, 0);
    let id = DiskChsnQuery::new(0, 0, 1, Some(2));
    first.write_sector_basic(ch, id, None, &[0x36; 512]).unwrap();
    for _ in 0..7 {
        first.incr_writes();
    }
    let sibling = first.clone();
    let separate = formatted();
    assert_eq!((first.write_ct(), sibling.write_ct(), separate.write_ct()), (8, 8, 1));
    let original = [&first, &sibling, &separate];
    let state = saved(&original);
    // Independent schema/meaning oracle: first-seen pointers map to counters.
    let wire = serde_json::to_value(&state).unwrap();
    assert_eq!(wire["version"], 1);
    assert_eq!(wire["contexts"], serde_json::json!([8, 1]));
    for (index, row) in wire["images"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row.as_array().unwrap().len(), 81);
        for (slot, binding) in row.as_array().unwrap().iter().enumerate() {
            let expected_path = if slot == 0 {
                "image".to_string()
            } else {
                format!("track/{}/bitstream", slot - 1)
            };
            assert_eq!(binding["path"], expected_path);
            assert_eq!(
                binding["reference"],
                serde_json::json!({"Shared":if index==2 {1} else {0}})
            );
        }
    }
    let legacy = json_images(&original);
    assert_eq!(
        legacy.iter().map(DiskImage::write_ct).collect::<Vec<_>>(),
        vec![0, 0, 0]
    );
    let mut restored = DiskImage::prepare_context_restore(legacy, &state).unwrap();
    assert_eq!(
        restored.iter().map(DiskImage::write_ct).collect::<Vec<_>>(),
        vec![8, 8, 1]
    );
    assert_eq!(graph(&original), graph(&restored.iter().collect::<Vec<_>>()));
    for image in &restored {
        assert!(!Arc::ptr_eq(
            image.shared.as_ref().unwrap(),
            first.shared.as_ref().unwrap()
        ));
        for track in &image.track_pool {
            assert!(Arc::ptr_eq(
                image.shared.as_ref().unwrap(),
                track.as_bitstream_track().unwrap().shared.as_ref().unwrap()
            ));
        }
    }
    restored[0].incr_writes(); // native shared-counter consumer, not hardware I/O
    assert_eq!(
        restored.iter().map(DiskImage::write_ct).collect::<Vec<_>>(),
        vec![9, 9, 1]
    );
    assert_eq!((first.write_ct(), sibling.write_ct(), separate.write_ct()), (8, 8, 1));
    restored[0].write_sector_basic(ch, id, None, &[0x72; 512]).unwrap();
    assert_eq!(restored[0].read_sector_basic(ch, id, None).unwrap(), vec![0x72; 512]);
    assert_eq!(restored[1].read_sector_basic(ch, id, None).unwrap(), vec![0x36; 512]);
    assert_eq!(first.read_sector_basic(ch, id, None).unwrap(), vec![0x36; 512]);
    println!("CONTEXT:3 native builder images; JSON counter0 failure repaired to8/8/1; alias propagation9/9/1 without live mutation; known sector bytes preserved");
}

#[test]
fn context_seeded_sparse_flux_and_missing_optional_owners_keep_exact_paths() {
    let base = formatted();
    let bit = base.track_pool[0].as_bitstream_track().unwrap().clone();
    let mut image = DiskImage::default();
    image.shared = None;
    image
        .track_pool
        .push(Box::new(FluxStreamTrack::seeded_context_fixture(bit)));
    let state = saved(&[&image]);
    assert_eq!(
        state.images[0]
            .iter()
            .map(|binding| binding.path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "image",
            "track/0/flux",
            "track/0/flux/decoded/0",
            "track/0/flux/decoded/2",
            "track/0/flux/resolved"
        ]
    );
    let restored = DiskImage::prepare_context_restore(json_images(&[&image]), &state).unwrap();
    assert_eq!(graph(&[&image]), graph(&[&restored[0]]));
    assert_eq!(restored[0].write_ct(), 0); // native missing image context preserved
    let actual = slots(&restored[0]).unwrap();
    assert!(actual[0].context.is_none() && actual[3].context.is_none());
    assert!(Arc::ptr_eq(
        actual[1].context.as_ref().unwrap(),
        actual[2].context.as_ref().unwrap()
    ));
    assert!(Arc::ptr_eq(
        actual[1].context.as_ref().unwrap(),
        actual[4].context.as_ref().unwrap()
    ));
    assert_eq!(actual[1].context.as_ref().unwrap().lock().unwrap().writes, 1);
    // Equal slot counts do not establish equal cache positions.
    let mut moved = serde_json::to_value(&image).unwrap();
    moved["track_pool"][0]["FluxStreamTrack"]["decoded_revolutions"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    let changed: DiskImage = serde_json::from_value(moved).unwrap();
    assert!(DiskImage::prepare_context_restore(vec![changed], &state).is_err());
    println!("CONTEXT:seeded sparse flux metadata only; optional/null contexts and cached decoded/resolved paths preserved, not native flux continuation");
}

fn metadata_image() -> DiskImage {
    let mut image = DiskImage::default();
    image
        .add_track_metasector(&MetaSectorTrackParams {
            ch: DiskCh::new(0, 0),
            encoding: TrackDataEncoding::Mfm,
            data_rate: TrackDataRate::from(250_000),
        })
        .unwrap();
    image
}

#[test]
fn context_preflight_refuses_invalid_graph_before_touching_shared_native_state() {
    let mut image = metadata_image();
    // Native add_track_metasector increments the initial shared counter once.
    assert_eq!(image.write_ct(), 1);
    for _ in 0..17 {
        image.incr_writes();
    }
    let good = saved(&[&image]);
    for case in 0..7 {
        let mut bad = good.clone();
        match case {
            0 => bad.version += 1,
            1 => bad.images.clear(),
            2 => {
                bad.images[0].pop();
            }
            3 => bad.images[0][0].path = "wrong".into(),
            4 => bad.images[0][0].reference = Reference::Shared(999),
            5 => bad.images[0][1].reference = Reference::Null,
            _ => bad.contexts.push(42),
        }
        let counter = image.write_ct();
        let old = image.shared.as_ref().unwrap().clone();
        assert!(
            DiskImage::prepare_context_restore(vec![image.clone()], &bad).is_err(),
            "invalid graph case{case}"
        );
        assert_eq!(image.write_ct(), counter);
        assert!(Arc::ptr_eq(image.shared.as_ref().unwrap(), &old));
    }
    let good_next = DiskImage::prepare_context_restore(json_images(&[&image]), &good).unwrap();
    assert_eq!(good_next[0].write_ct(), 18);
    assert!(Arc::ptr_eq(
        good_next[0].shared.as_ref().unwrap(),
        &good_next[0].track_pool[0].as_metasector_track().unwrap().shared
    ));
    let empty = DiskImage::snapshot_contexts(&[]).unwrap();
    assert!(DiskImage::prepare_context_restore(vec![], &empty).unwrap().is_empty());
}

#[test]
fn context_capture_refuses_busy_and_poisoned_lock_without_resetting_it() {
    let image = DiskImage::default();
    let context = image.shared.as_ref().unwrap().clone();
    let guard = context.lock().unwrap();
    assert!(DiskImage::snapshot_contexts(&[&image]).is_err());
    drop(guard);
    assert_eq!(image.write_ct(), 0);
    let thread_context = context.clone();
    assert!(std::thread::spawn(move || {
        let _guard = thread_context.lock().unwrap();
        panic!("DELIBERATE poisoned context");
    })
    .join()
    .is_err());
    assert!(DiskImage::snapshot_contexts(&[&image]).is_err());
    assert!(context.is_poisoned());
}

#[test]
fn context_schema_requires_every_key_and_recovers_full_u64_counter() {
    let image = DiskImage::default();
    image.shared.as_ref().unwrap().lock().unwrap().writes = u64::MAX;
    let state = saved(&[&image]);
    assert_eq!(state.contexts, vec![u64::MAX]);
    let next = DiskImage::prepare_context_restore(json_images(&[&image]), &state).unwrap();
    assert_eq!(next[0].write_ct(), u64::MAX);
    let value = serde_json::to_value(&state).unwrap();
    for key in ["version", "contexts", "images"] {
        let mut bad = value.clone();
        bad.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<DiskContextSnapshot>(bad).is_err());
    }
    for key in ["path", "reference"] {
        let mut bad = value.clone();
        bad["images"][0][0].as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<DiskContextSnapshot>(bad).is_err());
    }
    for nested in [false, true] {
        let mut bad = value.clone();
        if nested {
            bad["images"][0][0]["extra"] = serde_json::json!(1);
        } else {
            bad["extra"] = serde_json::json!(1);
        }
        assert!(serde_json::from_value::<DiskContextSnapshot>(bad).is_err());
    }
}
