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
    let (explicit, cohere_env) = (present(explicit), present(cohere_env));
    match provider {
        Provider::Cohere => explicit.or(cohere_env),
        Provider::OpenAiCompatible => explicit,
    }
}

/// A blank value is not a key.
///
/// `std::env::var` hands back `Ok("")` for a variable that is set but empty, which is an
/// everyday state: a CI secret that didn't resolve, `export COHERE_API_KEY=` in a shell
/// script, a `.env` line with nothing after the `=`. Treating that as a credential meant
/// sending `Authorization: Bearer ` and getting an opaque 401 back from the server, in
/// place of the clear local error one line further on.
fn present(key: Option<String>) -> Option<String> {
    key.filter(|k| !k.trim().is_empty())
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
    fn a_blank_key_is_not_a_key() {
        for blank in ["", "   ", "\t", "\n"] {
            assert_eq!(
                resolve_api_key(Provider::Cohere, Some(blank.into()), None),
                None,
                "blank explicit key {blank:?} should not count"
            );
            assert_eq!(
                resolve_api_key(Provider::Cohere, None, Some(blank.into())),
                None,
                "blank $COHERE_API_KEY {blank:?} should not count"
            );
        }
    }

    /// The case that actually bites: an empty `--api-key` or `$RAQEEM_API_KEY` must fall
    /// through to a real `$COHERE_API_KEY` rather than shadowing it with nothing.
    #[test]
    fn a_blank_explicit_key_falls_through_to_the_env() {
        assert_eq!(
            resolve_api_key(Provider::Cohere, Some("".into()), Some("real-key".into())),
            Some("real-key".into())
        );
    }

    /// ...but it must not resurrect the Cohere key for a self-hosted endpoint.
    #[test]
    fn a_blank_explicit_key_does_not_leak_the_cohere_key_to_openai() {
        assert_eq!(
            resolve_api_key(
                Provider::OpenAiCompatible,
                Some("".into()),
                Some("cohere-key".into())
            ),
            None
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
