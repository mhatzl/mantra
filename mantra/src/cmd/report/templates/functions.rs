use std::cell::LazyCell;

use mantra_schema::{
    jiff::{Zoned, tz::TimeZone},
    media_type::MediaType,
    mime_guess::Mime,
    product::ProductId,
    report::{
        formattable_content::FormattableContent, requirement::RequirementReference,
        review::ReviewReference, test_case::TestCaseReference, test_run::TestRunReference,
    },
};
use minijinja::value::ViaDeserialize;
use serde::de::Error;

pub(super) fn product_url_path(
    product_id: ViaDeserialize<ProductId>,
) -> Result<String, minijinja::Error> {
    Ok(product_id.url_path().into_string())
}

pub(super) fn requirement_url_path(
    req: ViaDeserialize<RequirementReference>,
) -> Result<String, minijinja::Error> {
    Ok(req.url_path().into_string())
}

pub(super) fn review_url_path(
    review: ViaDeserialize<ReviewReference>,
) -> Result<String, minijinja::Error> {
    Ok(review.url_path().into_string())
}

pub(super) fn test_run_url_path(
    test_run: ViaDeserialize<TestRunReference>,
) -> Result<String, minijinja::Error> {
    Ok(test_run.url_path().into_string())
}

pub(super) fn test_case_url_path(
    test_case: ViaDeserialize<TestCaseReference>,
) -> Result<String, minijinja::Error> {
    Ok(test_case.url_path().into_string())
}

thread_local! {
    static MARKDOWN_MIME: LazyCell<Option<Mime>> = LazyCell::new(||
        MediaType::from_ext("markdown").into_mimes().and_then(|mimes| mimes.into_iter().next())
    );
    static HTML_MIME: LazyCell<Option<Mime>> = LazyCell::new(||
        MediaType::from_ext("html").into_mimes().and_then(|mimes| mimes.into_iter().next())
    );
}

pub(super) fn formatted_datetime(datetime: ViaDeserialize<Zoned>) -> String {
    datetime.to_string()
}

pub(super) fn formatted_datetime_in_system_timezone(datetime: ViaDeserialize<Zoned>) -> String {
    datetime.with_time_zone(TimeZone::system()).to_string()
}

pub(super) fn format_html(
    content: ViaDeserialize<FormattableContent>,
) -> Result<Option<String>, minijinja::Error> {
    if let Some(media_type) = &content.media_type
        && let Some(mimes) = media_type.mimes()
    {
        if MARKDOWN_MIME.with(|markdown_mime| {
            markdown_mime
                .as_ref()
                .map(|m| mimes.contains(m))
                .unwrap_or(false)
        }) {
            Ok(Some(comrak::markdown_to_html(
                &content.content,
                &comrak::Options::default(),
            )))
        } else if HTML_MIME.with(|markdown_mime| {
            markdown_mime
                .as_ref()
                .map(|m| mimes.contains(m))
                .unwrap_or(false)
        }) {
            Ok(Some(content.content.clone()))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
