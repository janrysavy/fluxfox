//! Preserve the source-map owner skipped by ordinary DiskImage serde.
//! Contexts, disk bytes, weak-bit entropy and external image aliases are separate.
use super::*;
use crate::{DiskImage, DiskImageError};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceMapSnapshot {
    version: u32,
    map: SavedMap,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
enum SavedMap {
    Absent,
    Null(NullSourceMap),
    Tree(SourceMap),
}

impl SourceMapSnapshot {
    fn validate(&self) -> Result<(), DiskImageError> {
        if self.version != 1 {
            return Err(DiskImageError::ImageCorruptError("source-map snapshot version".into()));
        }
        let result = match &self.map {
            SavedMap::Absent => Ok(()),
            SavedMap::Null(map) => map.tree.validate_snapshot(false),
            SavedMap::Tree(map) => map.map.validate_snapshot(true),
        };
        result.map_err(|message| DiskImageError::ImageCorruptError(format!("source-map snapshot: {message}")))
    }
}

impl DiskImage {
    /// Capture exact None, NullSourceMap or SourceMap state without normalizing.
    /// Capture at a host call boundary: borrowed active cursors/call frames are
    /// not image-owned persistent state. Native empty Default is storage-only;
    /// its existing indexed consumers may panic before and after restoration.
    /// A null map also owns a tree accessible via last_node(): retain that tree.
    /// Unknown third-party implementations are refused, not treated as null.
    pub fn snapshot_source_map(&self) -> Result<SourceMapSnapshot, DiskImageError> {
        let map = match &self.source_map {
            None => SavedMap::Absent,
            Some(owner) => {
                if let Some(map) = owner.as_any().downcast_ref::<NullSourceMap>() {
                    SavedMap::Null(map.clone())
                } else if let Some(map) = owner.as_any().downcast_ref::<SourceMap>() {
                    SavedMap::Tree(map.clone())
                } else {
                    return Err(DiskImageError::UnsupportedOperation("unknown source-map owner".into()));
                }
            }
        };
        let saved = SourceMapSnapshot { version: 1, map };
        saved.validate()?;
        Ok(saved)
    }

    /// Install an independently cloned owner into an owned image candidate,
    /// after validating the complete tree graph. This touches no live owner and
    /// validates no other image state; pair it with image/context evidence.
    pub fn prepare_source_map_restore(mut self, saved: &SourceMapSnapshot) -> Result<Self, DiskImageError> {
        saved.validate()?;
        self.source_map = match &saved.map {
            SavedMap::Absent => None,
            SavedMap::Null(map) => Some(Box::new(map.clone())),
            SavedMap::Tree(map) => Some(Box::new(map.clone())),
        };
        Ok(self)
    }
}

#[cfg(test)]
mod checks;
