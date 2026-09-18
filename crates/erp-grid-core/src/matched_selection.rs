//! Explicit and server-authorized all-matching selection.
use crate::RowId;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchingCredential {
    pub token: String,
    pub authorization_scope: String,
    pub query_fingerprint: String,
    pub snapshot: String,
    pub total: usize,
    pub expires_at: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedLoadedRows {
    scope: String,
    query: String,
    snapshot: String,
    rows: BTreeSet<RowId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedRowProof {
    row: RowId,
    scope: String,
    query: String,
    snapshot: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionError {
    InvalidCredential,
    Expired,
    ScopeMismatch,
    QueryMismatch,
    SnapshotMismatch,
    ExclusionNotLoaded,
    TooManyExplicit,
    TooManyExcluded,
    NotAllMatching,
    NotExplicit,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutableSelection {
    Explicit(Vec<RowId>),
    AllMatching {
        credential: MatchingCredential,
        excluded: Vec<RowId>,
    },
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MatchedSelection {
    explicit: BTreeSet<RowId>,
    all_matching: Option<(MatchingCredential, BTreeSet<RowId>)>,
    binding: Option<(String, String)>,
}

impl TrustedLoadedRows {
    /// The adapter must validate the response and its server snapshot first.
    /// This guards local membership only; it is not cryptographic authorization.
    pub fn from_verified_response(
        scope: String,
        query: String,
        snapshot: String,
        rows: impl IntoIterator<Item = RowId>,
    ) -> Result<Self, SelectionError> {
        if !valid_binding_text(&scope)
            || !valid_binding_text(&query)
            || !valid_binding_text(&snapshot)
        {
            return Err(SelectionError::InvalidCredential);
        }
        let mut accepted = BTreeSet::new();
        for (index, row) in rows.into_iter().enumerate() {
            if index >= 10_000 || row.0 == 0 || !accepted.insert(row) {
                return Err(SelectionError::InvalidCredential);
            }
        }
        Ok(Self {
            scope,
            query,
            snapshot,
            rows: accepted,
        })
    }
    pub fn proof(&self, row: RowId) -> Result<LoadedRowProof, SelectionError> {
        if !self.rows.contains(&row) {
            return Err(SelectionError::ExclusionNotLoaded);
        }
        Ok(LoadedRowProof {
            row,
            scope: self.scope.clone(),
            query: self.query.clone(),
            snapshot: self.snapshot.clone(),
        })
    }
}
impl MatchedSelection {
    pub const MAX_EXPLICIT: usize = 10_000;
    pub const MAX_EXCLUDED: usize = 10_000;
    pub fn explicit() -> Self {
        Self::default()
    }
    pub fn bind(&mut self, scope: &str, query: &str) -> Result<(), SelectionError> {
        if !valid_binding_text(scope) || !valid_binding_text(query) {
            return Err(SelectionError::InvalidCredential);
        }
        let next = (scope.into(), query.into());
        if self.binding.as_ref() != Some(&next) {
            self.explicit.clear();
            self.all_matching = None;
            self.binding = Some(next);
        }
        Ok(())
    }
    pub fn clear(&mut self) {
        self.explicit.clear();
        self.all_matching = None;
    }
    pub fn select(&mut self, row: RowId, selected: bool) -> Result<(), SelectionError> {
        if self.all_matching.is_some() {
            return Err(SelectionError::NotExplicit);
        }
        if selected && !self.explicit.contains(&row) && self.explicit.len() >= Self::MAX_EXPLICIT {
            return Err(SelectionError::TooManyExplicit);
        }
        if selected {
            self.explicit.insert(row);
        } else {
            self.explicit.remove(&row);
        }
        Ok(())
    }
    pub fn use_all_matching(
        &mut self,
        credential: MatchingCredential,
        now: u64,
    ) -> Result<(), SelectionError> {
        validate(&credential, now)?;
        self.check(
            &credential.authorization_scope,
            &credential.query_fingerprint,
        )?;
        self.explicit.clear();
        self.all_matching = Some((credential, BTreeSet::new()));
        Ok(())
    }
    pub fn set_excluded(
        &mut self,
        proof: LoadedRowProof,
        excluded: bool,
        now: u64,
    ) -> Result<(), SelectionError> {
        let Some((credential, ids)) = &mut self.all_matching else {
            return Err(SelectionError::NotAllMatching);
        };
        validate(credential, now)?;
        if proof.scope != credential.authorization_scope {
            return Err(SelectionError::ScopeMismatch);
        }
        if proof.query != credential.query_fingerprint {
            return Err(SelectionError::QueryMismatch);
        }
        if proof.snapshot != credential.snapshot {
            return Err(SelectionError::SnapshotMismatch);
        }
        if excluded && ids.len() >= Self::MAX_EXCLUDED && !ids.contains(&proof.row) {
            return Err(SelectionError::TooManyExcluded);
        }
        if excluded {
            if ids.len() >= credential.total && !ids.contains(&proof.row) {
                return Err(SelectionError::ExclusionNotLoaded);
            }
            ids.insert(proof.row);
        } else {
            ids.remove(&proof.row);
        }
        Ok(())
    }
    /// Apply a page's exclusion changes as one transaction.
    pub fn set_excluded_batch(
        &mut self,
        changes: impl IntoIterator<Item = (LoadedRowProof, bool)>,
        now: u64,
    ) -> Result<(), SelectionError> {
        let mut changes: Vec<_> = changes
            .into_iter()
            .take(Self::MAX_EXCLUDED * 2 + 1)
            .collect();
        if changes.len() > Self::MAX_EXCLUDED * 2 {
            return Err(SelectionError::TooManyExcluded);
        }
        // Removing first permits an atomic swap while the exclusion set is at capacity.
        changes.sort_by_key(|(_, excluded)| *excluded);
        let mut next = self.clone();
        for (proof, excluded) in changes {
            next.set_excluded(proof, excluded, now)?;
        }
        *self = next;
        Ok(())
    }
    pub fn excluded_ids(&self) -> Option<&BTreeSet<RowId>> {
        self.all_matching.as_ref().map(|(_, ids)| ids)
    }
    pub fn selected_count(
        &self,
        scope: &str,
        query: &str,
        now: u64,
    ) -> Result<usize, SelectionError> {
        match &self.all_matching {
            Some((c, ids)) => {
                validate(c, now)?;
                binding(c, scope, query)?;
                Ok(c.total - ids.len())
            }
            None => {
                self.check(scope, query)?;
                Ok(self.explicit.len())
            }
        }
    }
    pub fn executable(
        &self,
        scope: &str,
        query: &str,
        now: u64,
    ) -> Result<ExecutableSelection, SelectionError> {
        match &self.all_matching {
            Some((c, ids)) => {
                validate(c, now)?;
                binding(c, scope, query)?;
                Ok(ExecutableSelection::AllMatching {
                    credential: c.clone(),
                    excluded: ids.iter().copied().collect(),
                })
            }
            None => {
                self.check(scope, query)?;
                Ok(ExecutableSelection::Explicit(
                    self.explicit.iter().copied().collect(),
                ))
            }
        }
    }
    fn check(&self, scope: &str, query: &str) -> Result<(), SelectionError> {
        match &self.binding {
            Some((s, q)) if s == scope && q == query => Ok(()),
            Some((s, _)) if s != scope => Err(SelectionError::ScopeMismatch),
            Some(_) => Err(SelectionError::QueryMismatch),
            None => Err(SelectionError::ScopeMismatch),
        }
    }
}
fn validate(c: &MatchingCredential, now: u64) -> Result<(), SelectionError> {
    if c.token.is_empty()
        || c.token.len() > 4096
        || c.token.contains('\0')
        || !valid_binding_text(&c.authorization_scope)
        || !valid_binding_text(&c.query_fingerprint)
        || !valid_binding_text(&c.snapshot)
        || now >= c.expires_at
    {
        return Err(if now >= c.expires_at {
            SelectionError::Expired
        } else {
            SelectionError::InvalidCredential
        });
    }
    Ok(())
}
fn valid_binding_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.contains('\0')
}
fn binding(c: &MatchingCredential, scope: &str, query: &str) -> Result<(), SelectionError> {
    if c.authorization_scope != scope {
        return Err(SelectionError::ScopeMismatch);
    }
    if c.query_fingerprint != query {
        return Err(SelectionError::QueryMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn credential() -> MatchingCredential {
        MatchingCredential {
            token: "opaque-server-token".into(),
            authorization_scope: "account-a".into(),
            query_fingerprint: "query-a".into(),
            snapshot: "snapshot-a".into(),
            total: 10_000,
            expires_at: 20,
        }
    }
    #[test]
    fn all_matching_count_exclusions_restore_and_scope_changes_are_explicit() {
        let mut selection = MatchedSelection::default();
        selection.bind("account-a", "query-a").unwrap();
        selection.use_all_matching(credential(), 10).unwrap();
        let loaded = TrustedLoadedRows::from_verified_response(
            "account-a".into(),
            "query-a".into(),
            "snapshot-a".into(),
            [RowId(2), RowId(3)],
        )
        .unwrap();
        assert_eq!(
            loaded.proof(RowId(99)),
            Err(SelectionError::ExclusionNotLoaded)
        );
        selection
            .set_excluded(loaded.proof(RowId(2)).unwrap(), true, 10)
            .unwrap();
        selection
            .set_excluded(loaded.proof(RowId(3)).unwrap(), true, 10)
            .unwrap();
        assert_eq!(
            selection.selected_count("account-a", "query-a", 10),
            Ok(9998)
        );
        selection
            .set_excluded(loaded.proof(RowId(2)).unwrap(), false, 10)
            .unwrap();
        assert_eq!(
            selection.selected_count("account-a", "query-a", 10),
            Ok(9999)
        );
        assert_eq!(
            selection.executable("account-a", "query-a", 20),
            Err(SelectionError::Expired)
        );
        selection.bind("account-b", "query-a").unwrap();
        assert_eq!(selection.selected_count("account-b", "query-a", 10), Ok(0));
    }
    #[test]
    fn wrong_credential_and_excess_membership_do_not_mutate_selection() {
        let mut selection = MatchedSelection::default();
        selection.bind("account-b", "query-a").unwrap();
        selection.select(RowId(1), true).unwrap();
        let before = selection.clone();
        assert_eq!(
            selection.use_all_matching(credential(), 10),
            Err(SelectionError::ScopeMismatch)
        );
        assert_eq!(selection, before);
        let count = std::cell::Cell::new(0);
        let rows = (1..).map(|row| {
            count.set(count.get() + 1);
            RowId(row)
        });
        assert!(
            TrustedLoadedRows::from_verified_response("a".into(), "q".into(), "s".into(), rows)
                .is_err()
        );
        assert_eq!(count.get(), 10_001);
    }

    #[test]
    fn batch_changes_validate_identity_and_swap_at_capacity_atomically() {
        let mut selection = MatchedSelection::default();
        selection.bind("account-a", "query-a").unwrap();
        let mut c = credential();
        c.total = 10_000;
        selection.use_all_matching(c.clone(), 10).unwrap();
        let rows = TrustedLoadedRows::from_verified_response(
            "account-a".into(),
            "query-a".into(),
            "snapshot-a".into(),
            [RowId(1), RowId(10_001)],
        )
        .unwrap();
        let mut changes = Vec::with_capacity(MatchedSelection::MAX_EXCLUDED + 1);
        for id in 1..=MatchedSelection::MAX_EXCLUDED as u128 {
            let loaded = TrustedLoadedRows::from_verified_response(
                "account-a".into(),
                "query-a".into(),
                "snapshot-a".into(),
                [RowId(id)],
            )
            .unwrap();
            changes.push((loaded.proof(RowId(id)).unwrap(), true));
        }
        selection.set_excluded_batch(changes, 10).unwrap();
        let bad_rows = TrustedLoadedRows::from_verified_response(
            "account-a".into(),
            "query-a".into(),
            "other-snapshot".into(),
            [RowId(10_001)],
        )
        .unwrap();
        let bad = bad_rows.proof(RowId(10_001)).unwrap();
        let before = selection.clone();
        assert_eq!(
            selection.set_excluded_batch([(rows.proof(RowId(1)).unwrap(), false), (bad, true)], 10),
            Err(SelectionError::SnapshotMismatch)
        );
        assert_eq!(selection, before);
        selection
            .set_excluded_batch(
                [
                    (rows.proof(RowId(1)).unwrap(), false),
                    (rows.proof(RowId(10_001)).unwrap(), true),
                ],
                10,
            )
            .unwrap();
        assert_eq!(
            selection.excluded_ids().unwrap().len(),
            MatchedSelection::MAX_EXCLUDED
        );
    }
}
