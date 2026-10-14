//! Shared SDK CLI argument helpers.

use vapor_sdk_core as core;
pub(super) use vapor_sdk_core::{ContentSource, ContentType};

pub(super) fn child(content_type: ContentType, content_id: String) -> core::ChildContentRef {
    core::ChildContentRef {
        content_type,
        content_id,
    }
}
