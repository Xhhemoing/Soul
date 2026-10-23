//! Offline documentation from the same DTOs the HTTP adapter serializes.
use soul_types::{MemoryDetailView, PeoplePage, PeoplePageRequest, ReadError, RuntimeStatus};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    info(title = "Soul synthetic local API", version = "1"),
    paths(status, people, memory,
        crate::openapi_writes::import_prepare, crate::openapi_writes::import_confirm,
        crate::openapi_writes::import_discard, crate::openapi_writes::import_state,
        crate::openapi_writes::scoped_draft, crate::openapi_writes::scoped_propose,
        crate::openapi_writes::scoped_confirm, crate::openapi_writes::scoped_discard,
        crate::openapi_writes::scoped_status, crate::openapi_writes::memory_edit,
        crate::openapi_writes::forget_preview, crate::openapi_writes::forget_confirm,
        crate::openapi_writes::memory_state, crate::openapi_writes::event_snapshot),
    components(schemas(RuntimeStatus, PeoplePageRequest, PeoplePage, MemoryDetailView, ReadError)),
    modifiers(&BearerSecurity),
    security(("runtime_bearer" = []))
)]
struct ApiDocument;

struct BearerSecurity;
impl Modify for BearerSecurity {
    fn modify(&self, document: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = document.components.as_mut() {
            components.add_security_scheme(
                "runtime_bearer",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .build(),
                ),
            );
        }
    }
}

// These markers describe the original read routes for the offline document;
// writes and their related queries are described in openapi_writes.
#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/status", responses(
    (status = 200, body = RuntimeStatus),
    (status = 400, body = ReadError),
    (status = 401, body = ReadError),
    (status = 403, body = ReadError),
    (status = "default", description = "Safe transport or read refusal, including 404, 405 and 503", body = ReadError)
))]
fn status() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/people", params(PeoplePageRequest), responses(
    (status = 200, body = PeoplePage),
    (status = 400, body = ReadError),
    (status = 401, body = ReadError),
    (status = 403, body = ReadError),
    (status = 413, body = ReadError),
    (status = "default", description = "Safe transport or read refusal, including 404, 405 and 503", body = ReadError)
))]
fn people() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/memories/{memory_id}",
    params(("memory_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses(
    (status = 200, body = MemoryDetailView),
    (status = 400, body = ReadError),
    (status = 401, body = ReadError),
    (status = 403, body = ReadError),
    (status = 413, body = ReadError),
    (status = "default", description = "Safe transport or read refusal, including 404, 405 and 503", body = ReadError)
))]
fn memory() {}

pub fn document() -> utoipa::openapi::OpenApi {
    ApiDocument::openapi()
}

pub fn json() -> Result<String, serde_json::Error> {
    let mut json = serde_json::to_string_pretty(&document())?;
    json.push('\n');
    Ok(json)
}
