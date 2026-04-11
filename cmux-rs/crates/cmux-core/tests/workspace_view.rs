use cmux_core::bonsplit::SplitOrientation;
use cmux_core::workspace::{TabManager, Workspace};

#[test]
fn workspace_snapshot_includes_root_tab_tree() {
    let workspace = Workspace::with_starting_tab("Workspace 1", "/tmp", "Welcome");
    let snapshot = workspace.snapshot();

    assert_eq!(snapshot.title, "Workspace 1");
    match snapshot.tree {
        cmux_core::bonsplit::ExternalTreeNode::Pane { pane } => {
            assert_eq!(pane.tabs.len(), 1);
            assert_eq!(pane.tabs[0].title, "Welcome");
        }
        _ => panic!("expected a single pane tree"),
    }
}

#[test]
fn tab_manager_snapshot_tracks_selected_workspace() {
    let mut tab_manager = TabManager::new();
    let first = Workspace::with_starting_tab("Workspace 1", "/tmp", "Welcome");
    let second = Workspace::with_starting_tab("Workspace 2", "/tmp", "Welcome");
    let first_id = first.id;
    let second_id = second.id;
    tab_manager.push_workspace(first);
    tab_manager.push_workspace(second);
    tab_manager.select_workspace(first_id);

    let snapshot = tab_manager.snapshot();
    assert_eq!(snapshot.selected_workspace_id, Some(first_id));
    assert_eq!(snapshot.workspaces.len(), 2);
    assert_eq!(snapshot.workspaces[0].id, first_id);
    assert_eq!(snapshot.workspaces[1].id, second_id);
}

#[test]
fn workspace_split_updates_snapshot_tree() {
    let mut workspace = Workspace::with_starting_tab("Workspace 1", "/tmp", "Welcome");
    let pane_id = workspace.focused_pane_id().expect("pane");
    let new_pane = workspace
        .split_pane(pane_id, SplitOrientation::Horizontal, None, false)
        .expect("split");
    let snapshot = workspace.snapshot();

    match snapshot.tree {
        cmux_core::bonsplit::ExternalTreeNode::Split { split } => {
            assert_eq!(split.orientation, "horizontal");
            assert!(split.first != split.second);
            let first = match *split.first {
                cmux_core::bonsplit::ExternalTreeNode::Pane { pane } => pane,
                _ => panic!("expected pane"),
            };
            let second = match *split.second {
                cmux_core::bonsplit::ExternalTreeNode::Pane { pane } => pane,
                _ => panic!("expected pane"),
            };
            assert!(first.id == pane_id || first.id == new_pane);
            assert!(second.id == pane_id || second.id == new_pane);
        }
        _ => panic!("expected a split tree"),
    }
}
