use genex_config::Constrained;
use genex_config::ConstraintResult;
use genex_protocol::models::ActivePermissionProfile;
use genex_protocol::models::PermissionProfile;
use genex_protocol::models::PermissionProfileSnapshot;
use genex_protocol::models::ProfileWorkspaceRoot;
use genex_utils_path_uri::PathUri;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PermissionProfileState {
    permission_profile: Constrained<PermissionProfileSnapshot>,
}

impl PermissionProfileState {
    pub(crate) fn from_constrained_legacy(
        constrained_permission_profile: Constrained<PermissionProfile>,
    ) -> ConstraintResult<Self> {
        let snapshot =
            PermissionProfileSnapshot::legacy(constrained_permission_profile.get().clone());
        Self::from_constrained_snapshot(constrained_permission_profile, snapshot)
    }

    pub(crate) fn from_constrained_active_profile(
        constrained_permission_profile: Constrained<PermissionProfile>,
        active_permission_profile: Option<ActivePermissionProfile>,
        profile_workspace_roots: Vec<PathUri>,
    ) -> ConstraintResult<Self> {
        let permission_profile = constrained_permission_profile.get().clone();
        let snapshot = match active_permission_profile {
            Some(active_permission_profile) => {
                PermissionProfileSnapshot::active_with_profile_workspace_roots(
                    permission_profile,
                    active_permission_profile,
                    profile_workspace_roots
                        .into_iter()
                        .map(ProfileWorkspaceRoot::from)
                        .collect(),
                )
            }
            None => PermissionProfileSnapshot::legacy(permission_profile),
        };
        Self::from_constrained_snapshot(constrained_permission_profile, snapshot)
    }

    pub(crate) fn from_constrained_snapshot(
        constrained_permission_profile: Constrained<PermissionProfile>,
        permission_profile: PermissionProfileSnapshot,
    ) -> ConstraintResult<Self> {
        let permission_profile = Constrained::new(permission_profile, move |candidate| {
            constrained_permission_profile.can_set(candidate.permission_profile())
        })?;
        Ok(Self { permission_profile })
    }

    pub(crate) fn permission_profile(&self) -> &PermissionProfile {
        self.permission_profile.get().permission_profile()
    }

    pub(crate) fn snapshot(&self) -> PermissionProfileSnapshot {
        self.permission_profile.get().clone()
    }

    pub(crate) fn active_permission_profile(&self) -> Option<ActivePermissionProfile> {
        self.permission_profile.get().active_permission_profile()
    }

    pub(crate) fn profile_workspace_roots(&self) -> &[ProfileWorkspaceRoot] {
        self.permission_profile.get().profile_workspace_roots()
    }

    pub(crate) fn can_set_legacy_permission_profile(
        &self,
        permission_profile: &PermissionProfile,
    ) -> ConstraintResult<()> {
        let candidate = PermissionProfileSnapshot::legacy(permission_profile.clone());
        self.permission_profile.can_set(&candidate)
    }

    pub(crate) fn set_legacy_permission_profile(
        &mut self,
        permission_profile: PermissionProfile,
    ) -> ConstraintResult<()> {
        self.permission_profile
            .set(PermissionProfileSnapshot::legacy(permission_profile))
    }

    pub(crate) fn set_permission_profile_snapshot(
        &mut self,
        snapshot: PermissionProfileSnapshot,
    ) -> ConstraintResult<()> {
        self.permission_profile.set(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genex_config::Constrained;
    use genex_protocol::models::ActivePermissionProfile;
    use genex_utils_absolute_path::AbsolutePathBuf;

    fn constrained(profile: PermissionProfile) -> Constrained<PermissionProfile> {
        Constrained::allow_any(profile)
    }

    #[test]
    fn tracks_legacy_profile_and_allows_compatible_updates() {
        let profile = PermissionProfile::read_only();
        let mut state =
            PermissionProfileState::from_constrained_legacy(constrained(profile.clone()))
                .expect("legacy profile should construct");

        assert_eq!(state.permission_profile(), &profile);
        assert!(state.active_permission_profile().is_none());
        assert!(state.profile_workspace_roots().is_empty());
        assert!(
            state
                .can_set_legacy_permission_profile(&PermissionProfile::Disabled)
                .is_ok()
        );
        state
            .set_legacy_permission_profile(PermissionProfile::Disabled)
            .expect("unconstrained state should accept update");
        assert_eq!(state.permission_profile(), &PermissionProfile::Disabled);
    }

    #[test]
    fn preserves_active_profile_identity_and_workspace_roots() {
        let path = AbsolutePathBuf::try_from("/workspace/project").expect("absolute path");
        let active = ActivePermissionProfile::new("project");
        let state = PermissionProfileState::from_constrained_active_profile(
            constrained(PermissionProfile::workspace_write()),
            Some(active.clone()),
            vec![PathUri::from_abs_path(&path)],
        )
        .expect("active profile should construct");

        assert_eq!(state.active_permission_profile(), Some(active));
        assert_eq!(state.profile_workspace_roots().len(), 1);
        assert_eq!(state.snapshot().profile_workspace_roots().len(), 1);
    }
}
