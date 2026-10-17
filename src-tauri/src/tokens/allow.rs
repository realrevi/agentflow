//! "Allow once": a single-use token that lets one API chat send through a hard budget stop.
//! Tokens are random, bound to one provider and expire after a few minutes.

use std::collections::hash_map::RandomState;
use std::hash::BuildHasher;

use serde::Serialize;

const TTL_MS: u64 = 10 * 60_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowOnceGrant {
    pub token: String,
    pub provider_id: String,
    /// ms since the Unix epoch.
    pub expires_at: u64,
}

#[derive(Debug, Default)]
pub struct AllowOnce {
    grants: Vec<AllowOnceGrant>,
    issued: u64,
}

impl AllowOnce {
    pub fn grant(&mut self, provider_id: &str, now: u64) -> AllowOnceGrant {
        self.grants.retain(|g| g.expires_at > now);
        self.issued += 1;
        let random = RandomState::new().hash_one((now, self.issued));
        let grant = AllowOnceGrant {
            token: format!("{random:016x}{:x}", self.issued),
            provider_id: provider_id.to_owned(),
            expires_at: now + TTL_MS,
        };
        self.grants.push(grant.clone());
        grant
    }

    /// Uses up `token` if it is valid for `provider_id`.
    pub fn consume(&mut self, token: &str, provider_id: &str, now: u64) -> bool {
        self.grants.retain(|g| g.expires_at > now);
        match self
            .grants
            .iter()
            .position(|g| g.token == token && g.provider_id == provider_id)
        {
            Some(index) => {
                self.grants.remove(index);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_works_once_for_its_provider_until_it_expires() {
        let mut allow = AllowOnce::default();
        let grant = allow.grant("p1", 1_000);
        assert!(
            !allow.consume(&grant.token, "p2", 1_001),
            "another provider"
        );
        assert!(allow.consume(&grant.token, "p1", 1_002));
        assert!(!allow.consume(&grant.token, "p1", 1_003), "single use");

        let late = allow.grant("p1", 2_000);
        assert_ne!(late.token, grant.token);
        assert!(!allow.consume(&late.token, "p1", 2_000 + TTL_MS), "expired");
    }
}
