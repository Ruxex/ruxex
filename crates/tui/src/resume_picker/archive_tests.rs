use super::super::PickerLoadRequest;
use super::super::PickerLoader;
use super::super::PickerState;
use super::super::Row;
use super::super::SessionPickerAction;
use super::super::SessionPickerLaunchContext;
use super::super::SessionSelection;
use super::super::SessionStatus;
use super::super::SessionTarget;
use super::ArchiveState;
use crate::key_hint::KeyBinding;
use crate::tui::FrameRequester;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use pretty_assertions::assert_eq;
use ruxex_app_server_client::TypedRequestError;
use ruxex_app_server_protocol::JSONRPCErrorError;
use ruxex_app_server_protocol::ThreadSortKey;
use ruxex_protocol::ThreadId;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

fn archive_picker_state() -> (PickerState, Arc<Mutex<Vec<ThreadId>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let request_sink = Arc::clone(&requests);
    let picker_loader: PickerLoader = Arc::new(move |request| {
        if let PickerLoadRequest::Archive { thread_id }
        | PickerLoadRequest::Unarchive { thread_id } = request
        {
            request_sink
                .lock()
                .expect("archive request sink")
                .push(thread_id);
        }
    });
    let state = PickerState::new(
        FrameRequester::test_dummy(),
        picker_loader,
        /*show_all*/ true,
        /*filter_cwd*/ None,
        SessionPickerAction::Resume,
    );
    (state, requests)
}

fn set_selected_session(state: &mut PickerState, thread_id: ThreadId) {
    state.all_rows = vec![Row {
        path: None,
        preview: String::from("Selected session"),
        thread_id: Some(thread_id),
        thread_name: None,
        created_at: None,
        updated_at: None,
        cwd: None,
        git_branch: None,
    }];
    state.apply_filter();
}

#[tokio::test]
async fn archive_shortcut_archives_selected_session_once() {
    let (mut state, requests) = archive_picker_state();
    let thread_id = ThreadId::new();
    set_selected_session(&mut state, thread_id);
    let shortcut = KeyEvent::new(KeyCode::Char('\u{0001}'), KeyModifiers::NONE);

    assert!(state.handle_key(shortcut).await.unwrap().is_none());
    assert!(state.handle_key(shortcut).await.unwrap().is_none());
    assert_eq!(state.archive_state, ArchiveState::Pending { thread_id });
    assert_eq!(*requests.lock().unwrap(), vec![thread_id]);
    assert!(
        state
            .handle_key(KeyEvent::from(KeyCode::Enter))
            .await
            .unwrap()
            .is_none()
    );

    state.handle_archive_result(thread_id, Ok(()));

    assert_eq!(state.archive_state, ArchiveState::Idle);
    assert!(state.filtered_rows.is_empty());
}

#[test]
fn archive_request_rejects_current_session() {
    let (mut state, requests) = archive_picker_state();
    let thread_id = ThreadId::new();
    state.launch_context = SessionPickerLaunchContext::ExistingSession {
        current_thread_id: Some(thread_id),
    };
    set_selected_session(&mut state, thread_id);

    state.request_archive_for_selected_session();

    assert_eq!(state.archive_state, ArchiveState::Idle);
    assert_eq!(
        state.inline_error.as_deref(),
        Some("Use /archive to archive the current session and exit.")
    );
    assert!(requests.lock().unwrap().is_empty());
}

#[test]
fn archive_failure_preserves_session_and_reports_server_error() {
    let (mut state, requests) = archive_picker_state();
    let thread_id = ThreadId::new();
    set_selected_session(&mut state, thread_id);
    state.request_archive_for_selected_session();
    let error = TypedRequestError::Server {
        method: String::from("thread/archive"),
        source: JSONRPCErrorError {
            code: -32600,
            message: String::from("thread already has an active writer"),
            data: None,
        },
    };

    state.handle_archive_result(thread_id, Err(std::io::Error::other(error)));

    assert_eq!(state.archive_state, ArchiveState::Idle);
    assert_eq!(
        state.inline_error.as_deref(),
        Some(
            "Failed to archive session: thread/archive failed: thread already has an active writer (code -32600)"
        )
    );
    assert_eq!(state.filtered_rows.len(), 1);

    state.request_archive_for_selected_session();

    assert_eq!(*requests.lock().unwrap(), vec![thread_id, thread_id]);
}

#[test]
fn archive_shortcut_preserves_configured_list_binding() {
    let (mut state, _requests) = archive_picker_state();
    state.list_keymap.move_up.push(KeyBinding::new(
        KeyCode::Char('\u{0001}'),
        KeyModifiers::NONE,
    ));

    assert!(!state.archive_shortcut_available());
}

#[tokio::test]
async fn archived_session_restore_resumes_after_completion() {
    let (mut state, requests) = archive_picker_state();
    let thread_id = ThreadId::new();
    state.status = SessionStatus::Archived;
    set_selected_session(&mut state, thread_id);

    assert!(
        state
            .handle_key(KeyEvent::from(KeyCode::Enter))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(state.archive_state, ArchiveState::Restoring { thread_id });
    assert_eq!(*requests.lock().unwrap(), vec![thread_id]);

    assert!(
        state
            .handle_key(KeyEvent::from(KeyCode::Enter))
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(*requests.lock().unwrap(), vec![thread_id]);

    let selection = state.handle_unarchive_result(
        thread_id,
        Ok(SessionTarget {
            path: None,
            thread_id,
            cwd: None,
            history_mode: None,
        }),
    );

    assert!(matches!(
        selection,
        Some(SessionSelection::Resume(SessionTarget {
            path: None,
            thread_id: resumed_thread_id,
            cwd: None,
            history_mode: None,
        })) if resumed_thread_id == thread_id
    ));
    assert_eq!(state.archive_state, ArchiveState::Idle);
}

#[test]
fn archived_session_restore_failure_preserves_session_and_allows_retry() {
    let (mut state, requests) = archive_picker_state();
    let thread_id = ThreadId::new();
    state.status = SessionStatus::Archived;
    set_selected_session(&mut state, thread_id);
    state.request_unarchive(thread_id);

    let selection =
        state.handle_unarchive_result(thread_id, Err(std::io::Error::other("restore denied")));

    assert!(selection.is_none());
    assert_eq!(state.archive_state, ArchiveState::Idle);
    assert_eq!(
        state.inline_error.as_deref(),
        Some("Failed to restore archived session: restore denied")
    );
    assert_eq!(state.filtered_rows.len(), 1);

    state.request_unarchive(thread_id);

    assert_eq!(*requests.lock().unwrap(), vec![thread_id, thread_id]);
}
