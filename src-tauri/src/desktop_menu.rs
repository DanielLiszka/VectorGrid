use serde::Serialize;
use tauri::menu::{Menu, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter};

const SHELL_COMMAND_EVENT: &str = "shell-command";
const SHELL_MENU_ID_PREFIX: &str = "vta:shell:";
const QUIT_COMMAND_ID: &str = "quit";

#[derive(Clone, Copy)]
struct DesktopMenuCommand {
    command_id: &'static str,
    label: &'static str,
    accelerator: Option<&'static str>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShellCommandPayload<'a> {
    command_id: &'a str,
    source: &'static str,
}

const COMMAND_PALETTE: DesktopMenuCommand = DesktopMenuCommand {
    command_id: "open_command_palette",
    label: "Command Palette",
    accelerator: Some("CmdOrCtrl+K"),
};

const FILE_COMMANDS: &[DesktopMenuCommand] = &[
    DesktopMenuCommand {
        command_id: "browse_csv",
        label: "Browse CSV",
        accelerator: Some("CmdOrCtrl+O"),
    },
    DesktopMenuCommand {
        command_id: "open_polygon",
        label: "Open Polygon Downloader",
        accelerator: Some("CmdOrCtrl+Shift+O"),
    },
    DesktopMenuCommand {
        command_id: "open_latest_report",
        label: "Open Latest Report",
        accelerator: Some("CmdOrCtrl+Shift+R"),
    },
    DesktopMenuCommand {
        command_id: "open_latest_meta_report",
        label: "Open Latest Meta Report",
        accelerator: Some("CmdOrCtrl+Shift+M"),
    },
];

const RUN_COMMANDS: &[DesktopMenuCommand] = &[
    DesktopMenuCommand {
        command_id: "run",
        label: "Run Optimization",
        accelerator: Some("CmdOrCtrl+Enter"),
    },
    DesktopMenuCommand {
        command_id: "validate",
        label: "Run Validation",
        accelerator: Some("CmdOrCtrl+Shift+V"),
    },
];

const EXPORT_COMMANDS: &[DesktopMenuCommand] = &[
    DesktopMenuCommand {
        command_id: "export_csv",
        label: "Export Top CSV",
        accelerator: Some("CmdOrCtrl+Shift+C"),
    },
    DesktopMenuCommand {
        command_id: "export_json",
        label: "Export Top JSON",
        accelerator: Some("CmdOrCtrl+Shift+J"),
    },
];

const VIEW_COMMANDS: &[DesktopMenuCommand] = &[
    DesktopMenuCommand {
        command_id: "toggle_controls",
        label: "Toggle Controls Panel",
        accelerator: None,
    },
];

fn shell_menu_item_id(command_id: &str) -> String {
    format!("{SHELL_MENU_ID_PREFIX}{command_id}")
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
        true,
        command.accelerator,
    )
}

fn append_command_items<'a>(
    app: &'a AppHandle,
    mut submenu: SubmenuBuilder<'a, tauri::Wry, AppHandle>,
    commands: &[DesktopMenuCommand],
) -> tauri::Result<SubmenuBuilder<'a, tauri::Wry, AppHandle>> {
    for command in commands {
        let item = command_menu_item(app, *command)?;
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

    let file_menu = append_command_items(app, SubmenuBuilder::new(app, "File"), FILE_COMMANDS)?
        .separator();
    let file_menu = append_command_items(app, file_menu, EXPORT_COMMANDS)?.build()?;

    let run_menu = append_command_items(app, SubmenuBuilder::new(app, "Run"), RUN_COMMANDS)?.build()?;
    let view_menu = append_command_items(app, SubmenuBuilder::new(app, "View"), VIEW_COMMANDS)?.build()?;

    let menu = Menu::with_items(app, &[&app_menu, &file_menu, &run_menu, &view_menu])?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        handle_desktop_menu_event(app, event.id().as_ref());
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{shell_command_for_menu_id, shell_menu_item_id};

    #[test]
    fn shell_menu_ids_round_trip_to_command_ids() {
        let menu_id = shell_menu_item_id("run");
        assert_eq!(shell_command_for_menu_id(&menu_id), Some("run"));
        assert_eq!(shell_command_for_menu_id("about"), None);
    }
}
