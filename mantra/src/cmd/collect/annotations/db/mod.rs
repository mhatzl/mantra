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

pub mod aggregate;
pub mod cfg;
pub mod collect;
