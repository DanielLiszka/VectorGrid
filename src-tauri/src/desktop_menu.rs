use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, State};

const SHELL_COMMAND_EVENT: &str = "shell-command";
const SHELL_MENU_ID_PREFIX: &str = "vta:shell:";
const QUIT_COMMAND_ID: &str = "quit";

#[derive(Default)]
pub struct DesktopMenuState {
    command_items: Mutex<HashMap<String, MenuItem<tauri::Wry>>>,
    check_command_items: Mutex<HashMap<String, CheckMenuItem<tauri::Wry>>>,
}

impl DesktopMenuState {
    fn register_command_item(&self, command_id: &str, item: &MenuItem<tauri::Wry>) {
        match self.command_items.lock() {
            Ok(mut items) => {
                items.insert(command_id.to_string(), item.clone());
            }
            Err(error) => {
                log::warn!("failed to register native menu item {command_id}: {error}");
            }
        }
    }

    fn register_check_command_item(&self, command_id: &str, item: &CheckMenuItem<tauri::Wry>) {
        match self.check_command_items.lock() {
            Ok(mut items) => {
                items.insert(command_id.to_string(), item.clone());
            }
            Err(error) => {
                log::warn!("failed to register native check menu item {command_id}: {error}");
            }
        }
    }

    fn set_command_state(
        &self,
        command_id: &str,
        enabled: bool,
        checked: Option<bool>,
    ) -> Result<bool, String> {
        let item = match self.command_items.lock() {
            Ok(items) => items.get(command_id).cloned(),
            Err(error) => {
                return Err(format!("failed to lock native menu state: {error}"));
            }
        };
        if let Some(item) = item {
            return item
                .set_enabled(enabled)
                .map(|_| true)
                .map_err(|error| error.to_string());
        }

        let check_item = match self.check_command_items.lock() {
            Ok(items) => items.get(command_id).cloned(),
            Err(error) => {
                return Err(format!("failed to lock native check menu state: {error}"));
            }
        };
        let Some(check_item) = check_item else {
            return Ok(false);
        };
        check_item
            .set_enabled(enabled)
            .map_err(|error| error.to_string())?;
        if let Some(checked) = checked {
            check_item
                .set_checked(checked)
                .map_err(|error| error.to_string())?;
        }
        Ok(true)
    }
}

#[derive(Clone, Copy)]
struct DesktopMenuCommand {
    command_id: &'static str,
    label: &'static str,
    accelerator: Option<&'static str>,
    initial_enabled: bool,
}

#[derive(Clone, Copy)]
struct DesktopMenuCheckCommand {
    command_id: &'static str,
    label: &'static str,
    accelerator: Option<&'static str>,
    initial_enabled: bool,
    initial_checked: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShellCommandPayload<'a> {
    command_id: &'a str,
    source: &'static str,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellMenuCommandEnabledState {
    command_id: String,
    enabled: bool,
    checked: Option<bool>,
}

#[tauri::command]
pub fn set_shell_menu_command_states(
    state: State<'_, DesktopMenuState>,
    states: Vec<ShellMenuCommandEnabledState>,
) -> Result<(), String> {
    for entry in states {
        let command_id = entry.command_id.trim();
        if command_id.is_empty() {
            continue;
        }
        match state.set_command_state(command_id, entry.enabled, entry.checked) {
            Ok(true) | Ok(false) => {}
            Err(error) => {
                log::warn!("failed to update native menu state for {command_id}: {error}");
            }
        }
    }
    Ok(())
}

const COMMAND_PALETTE: DesktopMenuCommand = DesktopMenuCommand {
    command_id: "open_command_palette",
    label: "Command Palette",
    accelerator: Some("CmdOrCtrl+K"),
    initial_enabled: true,
};

const FILE_COMMANDS: &[DesktopMenuCommand] = &[
    DesktopMenuCommand {
        command_id: "browse_csv",
        label: "Browse CSV",
        accelerator: Some("CmdOrCtrl+O"),
        initial_enabled: true,
    },
    DesktopMenuCommand {
        command_id: "open_polygon",
        label: "Open Polygon Downloader",
        accelerator: Some("CmdOrCtrl+Shift+O"),
        initial_enabled: true,
    },
    DesktopMenuCommand {
        command_id: "open_latest_report",
        label: "Open Latest Report",
        accelerator: Some("CmdOrCtrl+Shift+R"),
        initial_enabled: false,
    },
    DesktopMenuCommand {
        command_id: "open_latest_meta_report",
        label: "Open Latest Meta Report",
        accelerator: Some("CmdOrCtrl+Shift+M"),
        initial_enabled: false,
    },
];

const RUN_COMMANDS: &[DesktopMenuCommand] = &[
    DesktopMenuCommand {
        command_id: "run",
        label: "Run Optimization",
        accelerator: Some("CmdOrCtrl+Enter"),
        initial_enabled: false,
    },
    DesktopMenuCommand {
        command_id: "cancel_run",
        label: "Cancel Run",
        accelerator: None,
        initial_enabled: false,
    },
    DesktopMenuCommand {
        command_id: "validate",
        label: "Run Validation",
        accelerator: Some("CmdOrCtrl+Shift+V"),
        initial_enabled: false,
    },
];

const EXPORT_COMMANDS: &[DesktopMenuCommand] = &[
    DesktopMenuCommand {
        command_id: "export_csv",
        label: "Export Top CSV",
        accelerator: Some("CmdOrCtrl+Shift+C"),
        initial_enabled: false,
    },
    DesktopMenuCommand {
        command_id: "export_json",
        label: "Export Top JSON",
        accelerator: Some("CmdOrCtrl+Shift+J"),
        initial_enabled: false,
    },
];

const CHECKED_VIEW_COMMANDS: &[DesktopMenuCheckCommand] = &[DesktopMenuCheckCommand {
    command_id: "toggle_controls",
    label: "Show Controls Panel",
    accelerator: None,
    initial_enabled: true,
    initial_checked: true,
}];

const VIEW_COMMANDS: &[DesktopMenuCommand] = &[DesktopMenuCommand {
    command_id: "open_task_center",
    label: "Task Center",
    accelerator: None,
    initial_enabled: true,
}];

fn shell_menu_item_id(command_id: &str) -> String {
    format!("{SHELL_MENU_ID_PREFIX}{command_id}")
}

#[cfg(test)]
fn shared_shell_menu_command_ids() -> Vec<&'static str> {
    FILE_COMMANDS
        .iter()
        .chain(RUN_COMMANDS.iter())
        .chain(EXPORT_COMMANDS.iter())
        .chain(VIEW_COMMANDS.iter())
        .map(|command| command.command_id)
        .chain(
            CHECKED_VIEW_COMMANDS
                .iter()
                .map(|command| command.command_id),
        )
        .collect()
}

#[cfg(test)]
fn checked_shell_menu_command_ids() -> Vec<&'static str> {
    CHECKED_VIEW_COMMANDS
        .iter()
        .map(|command| command.command_id)
        .collect()
}

#[cfg(test)]
fn initially_disabled_shell_menu_command_ids() -> Vec<&'static str> {
    FILE_COMMANDS
        .iter()
        .chain(RUN_COMMANDS.iter())
        .chain(EXPORT_COMMANDS.iter())
        .filter(|command| !command.initial_enabled)
        .map(|command| command.command_id)
        .collect()
}

pub fn shell_command_for_menu_id(menu_id: &str) -> Option<&str> {
    menu_id.strip_prefix(SHELL_MENU_ID_PREFIX)
}

fn command_menu_item(
    app: &AppHandle,
    command: DesktopMenuCommand,
) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(
        app,
        shell_menu_item_id(command.command_id),
        command.label,
        command.initial_enabled,
        command.accelerator,
    )
}

fn check_command_menu_item(
    app: &AppHandle,
    command: DesktopMenuCheckCommand,
) -> tauri::Result<CheckMenuItem<tauri::Wry>> {
    CheckMenuItem::with_id(
        app,
        shell_menu_item_id(command.command_id),
        command.label,
        command.initial_enabled,
        command.initial_checked,
        command.accelerator,
    )
}

fn append_command_items<'a>(
    app: &'a AppHandle,
    menu_state: &DesktopMenuState,
    mut submenu: SubmenuBuilder<'a, tauri::Wry, AppHandle>,
    commands: &[DesktopMenuCommand],
) -> tauri::Result<SubmenuBuilder<'a, tauri::Wry, AppHandle>> {
    for command in commands {
        let item = command_menu_item(app, *command)?;
        menu_state.register_command_item(command.command_id, &item);
        submenu = submenu.item(&item);
    }
    Ok(submenu)
}

fn append_check_command_items<'a>(
    app: &'a AppHandle,
    menu_state: &DesktopMenuState,
    mut submenu: SubmenuBuilder<'a, tauri::Wry, AppHandle>,
    commands: &[DesktopMenuCheckCommand],
) -> tauri::Result<SubmenuBuilder<'a, tauri::Wry, AppHandle>> {
    for command in commands {
        let item = check_command_menu_item(app, *command)?;
        menu_state.register_check_command_item(command.command_id, &item);
        submenu = submenu.item(&item);
    }
    Ok(submenu)
}

fn emit_shell_command(app: &AppHandle, command_id: &str) {
    let payload = ShellCommandPayload {
        command_id,
        source: "native-menu",
    };
    if let Err(error) = app.emit(SHELL_COMMAND_EVENT, payload) {
        log::warn!("failed to emit shell command from native menu: {error}");
    }
}

fn handle_desktop_menu_event(app: &AppHandle, menu_id: &str) {
    let Some(command_id) = shell_command_for_menu_id(menu_id) else {
        return;
    };
    if command_id == QUIT_COMMAND_ID {
        app.exit(0);
        return;
    }
    emit_shell_command(app, command_id);
}

pub fn install_desktop_menu(app: &AppHandle) -> tauri::Result<()> {
    let menu_state = app.state::<DesktopMenuState>();
    let command_palette = command_menu_item(app, COMMAND_PALETTE)?;
    let quit = MenuItem::with_id(
        app,
        shell_menu_item_id(QUIT_COMMAND_ID),
        "Quit VectorGrid",
        true,
        Some("CmdOrCtrl+Q"),
    )?;

    let app_menu = SubmenuBuilder::new(app, "App")
        .item(&command_palette)
        .separator()
        .item(&quit)
        .build()?;

    let file_menu = append_command_items(
        app,
        &menu_state,
        SubmenuBuilder::new(app, "File"),
        FILE_COMMANDS,
    )?
    .separator();
    let file_menu = append_command_items(app, &menu_state, file_menu, EXPORT_COMMANDS)?.build()?;

    let run_menu = append_command_items(
        app,
        &menu_state,
        SubmenuBuilder::new(app, "Run"),
        RUN_COMMANDS,
    )?
    .build()?;
    let view_menu = append_command_items(
        app,
        &menu_state,
        SubmenuBuilder::new(app, "View"),
        VIEW_COMMANDS,
    )?
    .separator();
    let view_menu =
        append_check_command_items(app, &menu_state, view_menu, CHECKED_VIEW_COMMANDS)?.build()?;

    let menu = Menu::with_items(app, &[&app_menu, &file_menu, &run_menu, &view_menu])?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        handle_desktop_menu_event(app, event.id().as_ref());
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        checked_shell_menu_command_ids, initially_disabled_shell_menu_command_ids,
        shared_shell_menu_command_ids, shell_command_for_menu_id, shell_menu_item_id,
    };

    #[test]
    fn shell_menu_ids_round_trip_to_command_ids() {
        let menu_id = shell_menu_item_id("run");
        assert_eq!(shell_command_for_menu_id(&menu_id), Some("run"));
        assert_eq!(shell_command_for_menu_id("about"), None);
    }

    #[test]
    fn shared_shell_menu_ids_cover_frontend_command_inventory() {
        assert_eq!(
            shared_shell_menu_command_ids(),
            vec![
                "browse_csv",
                "open_polygon",
                "open_latest_report",
                "open_latest_meta_report",
                "run",
                "cancel_run",
                "validate",
                "export_csv",
                "export_json",
                "open_task_center",
                "toggle_controls",
            ]
        );
    }

    #[test]
    fn stateful_and_startup_disabled_menu_inventory_is_explicit() {
        assert_eq!(checked_shell_menu_command_ids(), vec!["toggle_controls"]);
        assert_eq!(
            initially_disabled_shell_menu_command_ids(),
            vec![
                "open_latest_report",
                "open_latest_meta_report",
                "run",
                "cancel_run",
                "validate",
                "export_csv",
                "export_json",
            ]
        );
    }
}
