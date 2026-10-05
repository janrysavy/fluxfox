//! Supplement image serialization with the context graph which serde skips.
//! This saves native write counters and their aliases, not complete disk/Machine
//! state. Disk bytes, source maps, weak-bit entropy and external image owners
//! remain the caller's separate responsibilities. Capture only quiesced owners.
use super::*;
use std::collections::BTreeMap;

type Context = Arc<Mutex<SharedDiskContext>>;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiskContextSnapshot {
    version: u32,
    contexts: Vec<u64>,
    images: Vec<Vec<Binding>>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    path: String,
    reference: Reference,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum Reference {
    Null,
    Shared(usize),
}

struct Slot {
    path: String,
    required: bool,
    context: Option<Context>,
}

fn slots(image: &DiskImage) -> Result<Vec<Slot>, DiskImageError> {
    let mut result = vec![Slot {
        path: "image".into(),
        required: false,
        context: image.shared.clone(),
    }];
    for (index, track) in image.track_pool.iter().enumerate() {
        let prefix = format!("track/{index}");
        if let Some(track) = track.as_bitstream_track() {
            result.push(Slot {
                path: format!("{prefix}/bitstream"),
                required: false,
                context: track.shared.clone(),
            });
        } else if let Some(track) = track.as_metasector_track() {
            result.push(Slot {
                path: format!("{prefix}/metasector"),
                required: true,
                context: Some(track.shared.clone()),
            });
        } else if let Some(track) = track.as_fluxstream_track() {
            for (path, context) in track.snapshot_context_slots() {
                result.push(Slot {
                    path: format!("{prefix}/{path}"),
                    required: false,
                    context,
                });
            }
        } else {
            return Err(DiskImageError::UnsupportedOperation(
                "unknown track context owner".into(),
            ));
        }
    }
    Ok(result)
}

fn invalid(message: &str) -> DiskImageError {
    DiskImageError::ImageCorruptError(format!("disk context snapshot: {message}"))
}

impl DiskImage {
    /// Capture all image/track context counters and sharing across these images.
    /// No addresses enter the wire format. Busy/poisoned contexts are refused;
    /// capture does not wait, modify counters or normalize missing contexts.
    /// Pair this with the corresponding image payload and authenticate both.
    pub fn snapshot_contexts(images: &[&Self]) -> Result<DiskContextSnapshot, DiskImageError> {
        let mut saved = DiskContextSnapshot {
            version: 1,
            contexts: vec![],
            images: vec![],
        };
        let mut identities = BTreeMap::new();
        let mut contexts = Vec::new();
        for image in images {
            let mut bindings = vec![];
            for slot in slots(image)? {
                let reference = if let Some(context) = slot.context {
                    let identity = Arc::as_ptr(&context) as usize;
                    let index = if let Some(index) = identities.get(&identity) {
                        *index
                    } else {
                        let index = contexts.len();
                        contexts.push(context);
                        identities.insert(identity, index);
                        index
                    };
                    Reference::Shared(index)
                } else {
                    Reference::Null
                };
                bindings.push(Binding {
                    path: slot.path,
                    reference,
                });
            }
            saved.images.push(bindings);
        }
        // Hold every distinct context together: aliases cannot be relocked or
        // changed between individual counter reads. Other image state still
        // requires the caller's quiescence protocol.
        let guards = contexts
            .iter()
            .map(|context| context.try_lock().map_err(|e| DiskImageError::SyncError(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        for guard in &guards {
            saved.contexts.push(guard.writes);
        }
        Ok(saved)
    }

    /// Rebind owned candidates to fresh contexts, preserving the saved graph.
    /// Typical candidates are newly deserialized image payloads. This never
    /// writes through their old contexts, even if a candidate was cloned from
    /// a live image. Validate the complete graph before replacing any binding.
    /// The image payload itself and its other skipped fields are NOT validated
    /// or restored by this context-only API.
    pub fn prepare_context_restore(
        mut images: Vec<Self>,
        saved: &DiskContextSnapshot,
    ) -> Result<Vec<Self>, DiskImageError> {
        if saved.version != 1 {
            return Err(invalid("unsupported version"));
        }
        if images.len() != saved.images.len() {
            return Err(invalid("image count"));
        }
        let mut used = vec![false; saved.contexts.len()];
        for (image, bindings) in images.iter().zip(&saved.images) {
            let actual = slots(image)?;
            if actual.len() != bindings.len() {
                return Err(invalid("context slot count"));
            }
            for (slot, binding) in actual.iter().zip(bindings) {
                if slot.path != binding.path {
                    return Err(invalid("context slot path"));
                }
                match binding.reference {
                    Reference::Null if slot.required => return Err(invalid("missing required context")),
                    Reference::Null => {}
                    Reference::Shared(index) => {
                        let Some(mark) = used.get_mut(index) else {
                            return Err(invalid("context index"));
                        };
                        *mark = true;
                    }
                }
            }
        }
        if used.iter().any(|used| !used) {
            return Err(invalid("unreferenced context"));
        }
        let contexts: Vec<Context> = saved
            .contexts
            .iter()
            .map(|&writes| Arc::new(Mutex::new(SharedDiskContext { writes })))
            .collect();
        for (image, bindings) in images.iter_mut().zip(&saved.images) {
            let mut values = bindings.iter().map(|binding| match binding.reference {
                Reference::Null => None,
                Reference::Shared(index) => Some(contexts[index].clone()),
            });
            image.shared = values.next().expect("preflight image slot");
            for track in &mut image.track_pool {
                if let Some(track) = track.as_bitstream_track_mut() {
                    track.shared = values.next().expect("preflight bitstream slot");
                } else if let Some(track) = track.as_metasector_track_mut() {
                    track.shared = values
                        .next()
                        .expect("preflight metasector slot")
                        .expect("preflight required context");
                } else if let Some(track) = track.as_fluxstream_track_mut() {
                    track.restore_snapshot_context_slots(&mut || values.next().expect("preflight flux slot"));
                } else {
                    unreachable!("preflight recognized track owner");
                }
            }
            debug_assert!(values.next().is_none());
        }
        Ok(images)
    }
}

#[cfg(test)]
mod tests;
