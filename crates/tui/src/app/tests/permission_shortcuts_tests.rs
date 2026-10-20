use super::*;
use ruxex_arg0::Arg0DispatchPaths;
use pretty_assertions::assert_eq;

fn read_only_selection() -> PermissionProfileSelection {
    PermissionProfileSelection {
        profile_id: ":read-only".to_string(),
        approval_policy: Some(AskForApproval::OnRequest),
        approvals_reviewer: Some(ApprovalsReviewer::User),
        display_label: "Read Only".to_string(),
    }
}
