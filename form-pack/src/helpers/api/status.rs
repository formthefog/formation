use axum::{Json, extract::Path};
use crate::types::response::PackResponse;

pub(crate) async fn get_status(
    Path(_build_id): Path<String>,
) -> Json<PackResponse> {
    Json(PackResponse::Failure)
}
