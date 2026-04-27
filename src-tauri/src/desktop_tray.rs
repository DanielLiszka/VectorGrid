use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

const MAIN_WINDOW_LABEL: &str = "main";
const SHELL_COMMAND_EVENT: &str = "shell-command";
const TRAY_MENU_ID_PREFIX: &str = "vta:tray:";
const SHOW_WINDOW_COMMAND_ID: &str = "show_window";
const COMMAND_PALETTE_COMMAND_ID: &str = "open_command_palette";
const OPEN_LATEST_REPORT_COMMAND_ID: &str = "open_latest_report";
const QUIT_COMMAND_ID: &str = "quit";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShellCommandPayload<'a> {
    command_id: &'a str,
    source: &'static str,
}

fn tray_menu_item_id(command_id: &str) -> String {
    format!("{TRAY_MENU_ID_PREFIX}{command_id}")
}

fn command_id_for_tray_menu_id(menu_id: &str) -> Option<&str> {
    menu_id.strip_prefix(TRAY_MENU_ID_PREFIX)
}

fn emit_shell_command(app: &AppHandle, command_id: &str) {
    let payload = ShellCommandPayload {
        command_id,
        source: "native-tray",
    };
    if let Err(error) = app.emit(SHELL_COMMAND_EVENT, payload) {
        log::warn!("failed to emit shell command from native tray: {error}");
    }
}

fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        log::warn!("native tray requested main window focus before the window existed");
        return;
    };
    if let Err(error) = window.unminimize() {
        log::warn!("failed to unminimize main window from native tray: {error}");
    }
    if let Err(error) = window.show() {
        log::warn!("failed to show main window from native tray: {error}");
    }
    if let Err(error) = window.set_focus() {
        log::warn!("failed to focus main window from native tray: {error}");
    }
}

fn handle_tray_menu_event(app: &AppHandle, menu_id: &str) {
    let Some(command_id) = command_id_for_tray_menu_id(menu_id) else {
        return;
    };
    match command_id {
        SHOW_WINDOW_COMMAND_ID => show_main_window(app),
        QUIT_COMMAND_ID => app.exit(0),
        COMMAND_PALETTE_COMMAND_ID | OPEN_LATEST_REPORT_COMMAND_ID => {
            show_main_window(app);
            emit_shell_command(app, command_id);
        }
        _ => {}
    }
}

fn tray_menu_item(
    app: &AppHandle,
    command_id: &str,
    label: &str,
) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(
        app,
        tray_menu_item_id(command_id),
        label,
        true,
        None::<&str>,
    )
}

pub fn install_desktop_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = tray_menu_item(app, SHOW_WINDOW_COMMAND_ID, "Show VectorGrid")?;
    let command_palette = tray_menu_item(app, COMMAND_PALETTE_COMMAND_ID, "Command Palette")?;
    let open_latest_report =
        tray_menu_item(app, OPEN_LATEST_REPORT_COMMAND_ID, "Open Latest Report")?;
    let quit = tray_menu_item(app, QUIT_COMMAND_ID, "Quit VectorGrid")?;
    let menu = Menu::with_items(app, &[&show, &command_palette, &open_latest_report, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("VectorGrid")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_tray_menu_event(app, event.id().as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    let _tray = builder.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{command_id_for_tray_menu_id, tray_menu_item_id};

    #[test]
    fn tray_menu_ids_round_trip_to_command_ids() {
        let id = tray_menu_item_id("open_command_palette");
        assert_eq!(
            command_id_for_tray_menu_id(&id),
            Some("open_command_palette")
        );
        assert_eq!(command_id_for_tray_menu_id("quit"), None);
    }
}
