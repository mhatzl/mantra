use std::ops::Deref;

use anyhow::{Context, bail};
use mantra_schema::{
    FmtHash, Properties,
    annotations::{
        AnnotationSchema, CoverageExclude, CoverageExcludeKind, Element, FileAnnotations, Trace,
        TraceRelatedCodeVariant,
    },
    path::RelativePath,
};

use crate::cmd::collect::{Collection, merge_local_and_base_properties};

pub mod aggregate;
pub mod collect;
