use super::*;

/// The XErr codes the crate maps itself, beyond the two `minecraft-msa-auth`
/// interprets (`2148916233` no-xbox, `2148916238` add-to-family).
#[test]
fn xbox_err_codes_map_onto_user_presentable_variants() {
    assert!(matches!(AuthError::from_xerr(2148916234), AuthError::XboxLiveUnavailable));
    assert!(matches!(AuthError::from_xerr(2148916235), AuthError::RegionUnsupported));
    assert!(matches!(AuthError::from_xerr(2148916236), AuthError::AdultVerificationRequired));
    assert!(matches!(AuthError::from_xerr(2148916237), AuthError::AdultVerificationRequired));
}

#[test]
fn unknown_xerr_codes_carry_their_number() {
    let error = AuthError::from_xerr(999999999);
    assert!(matches!(error, AuthError::XboxLiveError { code: 999999999 }));
    assert_eq!(error.to_string(), "Xbox Live sign-in failed with error code 999999999");
}

#[test]
fn device_token_errors_map_onto_expiry_and_denial() {
    let expired = OAuth2DeviceTokenError::ServerResponse(oauth2::StandardErrorResponse::new(
        oauth2::DeviceCodeErrorResponseType::ExpiredToken,
        None,
        None,
    ));
    assert!(matches!(AuthError::from_device_token_error(expired), AuthError::DeviceCodeExpired));

    let denied = OAuth2DeviceTokenError::ServerResponse(oauth2::StandardErrorResponse::new(
        oauth2::DeviceCodeErrorResponseType::AccessDenied,
        None,
        None,
    ));
    assert!(matches!(AuthError::from_device_token_error(denied), AuthError::AuthorizationDenied));
}

#[test]
fn refresh_token_grant_errors_map_to_a_clean_expiry_message() {
    let invalid_grant = oauth2::RequestTokenError::ServerResponse(oauth2::StandardErrorResponse::new(
        oauth2::basic::BasicErrorResponseType::InvalidGrant,
        None,
        None,
    ));
    let error = AuthError::from_refresh_token_error(invalid_grant);
    assert_eq!(
        error.to_string(),
        "microsoft sign-in failed: your saved Microsoft sign-in has expired — sign in again"
    );
}

#[test]
fn the_microsoft_variant_carries_a_user_presentable_message() {
    let denied = OAuth2DeviceTokenError::ServerResponse(oauth2::StandardErrorResponse::new(
        oauth2::DeviceCodeErrorResponseType::AccessDenied,
        None,
        None,
    ));
    let error = AuthError::from_device_token_error(denied);
    assert_eq!(error.to_string(), "the sign-in request was denied");

    let error = AuthError::from_xerr(2148916235);
    assert_eq!(error.to_string(), "your region does not support Xbox Live sign-in");
}
