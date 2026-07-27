//! Which key a backend is allowed to be given.
//!
//! This is one rule, and it is a security rule, so it lives in one place. The CLI and
//! the Python binding both route through [`resolve_api_key`] rather than each deciding
//! for itself — they used to decide separately, and a comment in the binding admitted
//! it was "mirroring the CLI's credential rules exactly", which is the kind of promise
//! that holds right up until it doesn't.
//!
//! Reading the environment stays with the caller. That keeps this function pure and its
//! tests free of `set_var`, which is racy under a thread-parallel test runner and
//! `unsafe` as of Rust 2024.

use crate::provider::Provider;

/// Pick the API key to send, or `None` to send no `Authorization` header at all.
///
/// - `explicit` — what the user handed us for *this* call: `--api-key`, the `api_key=`
///   argument, or our own `$RAQEEM_API_KEY`. Applies to any backend, because it is
///   scoped to raqeem rather than to a vendor.
/// - `cohere_env` — `$COHERE_API_KEY`. Scoped to Cohere by its name, so it is a fallback
///   for [`Provider::Cohere`] and nothing else.
///
/// The asymmetry is the whole point: a self-hosted or third-party endpoint reached via
/// `--endpoint` must never be handed a Cohere key just because one happens to be
/// exported. That would ship the user's credential to a server Cohere doesn't control.
pub fn resolve_api_key(
    provider: Provider,
    explicit: Option<String>,
    cohere_env: Option<String>,
) -> Option<String> {
    match provider {
        Provider::Cohere => explicit.or(cohere_env),
        Provider::OpenAiCompatible => explicit,
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_api_key;
    use crate::provider::Provider;

    #[test]
    fn cohere_falls_back_to_the_cohere_scoped_env() {
        assert_eq!(
            resolve_api_key(Provider::Cohere, None, Some("cohere-key".into())),
            Some("cohere-key".into())
        );
    }

    #[test]
    fn an_explicit_key_wins_over_the_fallback() {
        assert_eq!(
            resolve_api_key(
                Provider::Cohere,
                Some("explicit".into()),
                Some("cohere-key".into())
            ),
            Some("explicit".into())
        );
    }

    /// The one that matters: a Cohere-scoped key must not leak to someone else's server.
    #[test]
    fn a_self_hosted_endpoint_never_receives_the_cohere_key() {
        assert_eq!(
            resolve_api_key(Provider::OpenAiCompatible, None, Some("cohere-key".into())),
            None
        );
    }

    #[test]
    fn a_self_hosted_endpoint_still_honors_its_own_key() {
        assert_eq!(
            resolve_api_key(
                Provider::OpenAiCompatible,
                Some("mykey".into()),
                Some("cohere-key".into())
            ),
            Some("mykey".into())
        );
    }

    #[test]
    fn no_key_anywhere_is_none_not_empty_string() {
        assert_eq!(resolve_api_key(Provider::Cohere, None, None), None);
        assert_eq!(
            resolve_api_key(Provider::OpenAiCompatible, None, None),
            None
        );
    }
}
