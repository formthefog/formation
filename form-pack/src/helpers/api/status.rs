use axum::{Json, extract::{Path, Extension}};
use crate::types::response::PackResponse;
use crate::auth::SignatureAuth;

pub(crate) async fn get_status(
    Path(build_id): Path<String>,
    Extension(signature_auth): Extension<Option<SignatureAuth>>,
) -> Json<PackResponse> {
    // Log authentication 
    if let Some(auth) = signature_auth {
        println!("Status request for build {} authenticated with signature from: {}", 
            build_id, auth.public_key_hex);
    } else {
        println!("Warning: Status request for build {} lacks authentication", build_id);
    }

    // TODO: Implement actual status checking
    Json(PackResponse::Failure)
}
