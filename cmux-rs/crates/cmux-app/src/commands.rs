use tauri::{AppHandle, State};
use uuid::Uuid;

use cmux_core::bonsplit::SplitOrientation;
use cmux_core::workspace::Workspace;

use crate::state::{
    mutate_workspace_by_id, pane_id_from, tab_id_from, workspace_id_from, AppSnapshot, AppState,
};

fn parse_orientation(raw: &str) -> Result<SplitOrientation, String> {
    match raw.to_ascii_lowercase().as_str() {
        "horizontal" => Ok(SplitOrientation::Horizontal),
        "vertical" => Ok(SplitOrientation::Vertical),
        other => Err(format!("invalid split orientation '{other}'")),
    }
}

#[tauri::command]
pub fn workspace_state(state: State<'_, AppState>) -> AppSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn workspace_create(
    app: AppHandle,
    state: State<'_, AppState>,
    title: Option<String>,
) -> Result<AppSnapshot, String> {
    let title = title.unwrap_or_else(|| "Workspace".to_string());
    state
        .mutate(&app, |inner| {
            let cwd = std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                .to_string_lossy()
                .to_string();
            let workspace = Workspace::with_starting_tab(title, cwd, "Welcome");
            inner.tab_manager.push_workspace(workspace);
            true
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn workspace_select(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    state
        .mutate(&app, |inner| {
            if inner.tab_manager.workspaces.iter().any(|workspace| workspace.id == id) {
                inner.tab_manager.select_workspace(id);
                true
            } else {
                false
            }
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn workspace_close(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    state
        .mutate(&app, |inner| inner.tab_manager.close_workspace(id))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn workspace_reorder(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    target_index: usize,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    state
        .mutate(&app, |inner| inner.tab_manager.reorder_workspace(id, target_index))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn workspace_add_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    pane_id: String,
    title: Option<String>,
    kind: Option<String>,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    let pane = pane_id_from(&pane_id)?;
    let title = title.unwrap_or_else(|| "Untitled".to_string());
    mutate_workspace_by_id(state, app, id, |workspace| {
        workspace
            .add_tab_to_pane_with_kind(pane, title, kind.clone())
            .is_some()
    })
}

#[tauri::command]
pub fn workspace_split_pane(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    pane_id: String,
    orientation: String,
    title: Option<String>,
    kind: Option<String>,
    insert_first: Option<bool>,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    let pane = pane_id_from(&pane_id)?;
    let orientation = parse_orientation(&orientation)?;
    let insert_first = insert_first.unwrap_or(false);
    mutate_workspace_by_id(state, app, id, |workspace| {
        workspace
            .split_pane_with_kind(pane, orientation, title.clone(), kind.clone(), insert_first)
            .is_some()
    })
}

#[tauri::command]
pub fn workspace_close_pane(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    pane_id: String,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    let pane = pane_id_from(&pane_id)?;
    mutate_workspace_by_id(state, app, id, |workspace| workspace.close_pane(pane))
}

#[tauri::command]
pub fn workspace_close_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    pane_id: String,
    tab_id: String,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    let pane = pane_id_from(&pane_id)?;
    let tab = tab_id_from(&tab_id)?;
    mutate_workspace_by_id(state, app, id, |workspace| workspace.close_tab_in_pane(tab, pane))
}

#[tauri::command]
pub fn workspace_move_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    tab_id: String,
    source_pane_id: String,
    target_pane_id: String,
    index: Option<usize>,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    let tab = tab_id_from(&tab_id)?;
    let source = pane_id_from(&source_pane_id)?;
    let target = pane_id_from(&target_pane_id)?;
    mutate_workspace_by_id(state, app, id, |workspace| {
        workspace.move_tab(tab, source, target, index)
    })
}

#[tauri::command]
pub fn workspace_select_pane(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    pane_id: String,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    let pane = pane_id_from(&pane_id)?;
    mutate_workspace_by_id(state, app, id, |workspace| {
        if workspace.bonsplit.root().find_pane(pane).is_some() {
            workspace.bonsplit.focus_pane(pane);
            true
        } else {
            false
        }
    })
}

#[tauri::command]
pub fn workspace_select_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    pane_id: String,
    tab_id: String,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    let pane = pane_id_from(&pane_id)?;
    let tab = tab_id_from(&tab_id)?;
    mutate_workspace_by_id(state, app, id, |workspace| workspace.bonsplit.select_tab(pane, tab))
}

#[tauri::command]
pub fn workspace_set_divider_position(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    split_id: String,
    position: f64,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    let split = Uuid::parse_str(&split_id).map_err(|err| format!("invalid uuid '{split_id}': {err}"))?;
    mutate_workspace_by_id(state, app, id, |workspace| workspace.set_divider_position(split, position))
}

#[tauri::command]
pub fn workspace_select_next_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    mutate_workspace_by_id(state, app, id, |workspace| {
        workspace.bonsplit.select_next_tab();
        true
    })
}

#[tauri::command]
pub fn workspace_select_previous_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<AppSnapshot, String> {
    let id = workspace_id_from(&workspace_id)?;
    mutate_workspace_by_id(state, app, id, |workspace| {
        workspace.bonsplit.select_previous_tab();
        true
    })
}
