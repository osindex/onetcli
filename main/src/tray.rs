use std::cell::RefCell;
use std::sync::mpsc;
use std::time::Duration;

use gpui::{App, AsyncApp, Window};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::app_init;

thread_local! {
    static TRAY_ICON: RefCell<Option<TrayIcon>> = const { RefCell::new(None) };
}

enum TrayAction {
    Show,
    Hide,
    Quit,
}

pub fn install_tray(cx: &mut App, icon: Icon) -> anyhow::Result<()> {
    let already_installed = TRAY_ICON.with(|cell| cell.borrow().is_some());
    if already_installed {
        return Ok(());
    }

    let show_item = MenuItem::with_id("tray.show", "Show OnetCli", true, None);
    let hide_item = MenuItem::with_id("tray.hide", "Hide/Minimize", true, None);
    let quit_item = MenuItem::with_id("tray.quit", "Quit", true, None);

    let menu = Menu::new();
    menu.append_items(&[
        &show_item,
        &hide_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ])?;

    let tray = TrayIconBuilder::new()
        .with_tooltip("OnetCli")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .build()?;

    TRAY_ICON.with(|cell| *cell.borrow_mut() = Some(tray));

    let show_id = show_item.id().clone();
    let hide_id = hide_item.id().clone();
    let quit_id = quit_item.id().clone();
    let (tx, rx) = mpsc::channel::<TrayAction>();

    let tx_menu = tx.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let action = if event.id == show_id {
            Some(TrayAction::Show)
        } else if event.id == hide_id {
            Some(TrayAction::Hide)
        } else if event.id == quit_id {
            Some(TrayAction::Quit)
        } else {
            None
        };

        if let Some(action) = action {
            let _ = tx_menu.send(action);
        }
    }));

    let tx_tray = tx;
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if matches!(event, TrayIconEvent::DoubleClick { .. }) {
            let _ = tx_tray.send(TrayAction::Show);
        }
    }));

    cx.spawn(|cx: &mut AsyncApp| {
        let mut cx = cx.clone();
        async move {
            loop {
                cx.background_executor().timer(Duration::from_millis(16)).await;

                while let Ok(action) = rx.try_recv() {
                    match action {
                        TrayAction::Show => {
                            let _ = app_init::toggle_main_window(&mut cx);
                        }
                        TrayAction::Hide => {
                            let _ = app_init::toggle_main_window(&mut cx);
                        }
                        TrayAction::Quit => {
                            let _ = cx.update(|app| app.quit());
                        }
                    }
                }
            }
        }
    })
    .detach();

    Ok(())
}

pub fn hide_to_tray(window: &mut Window) {
    window.minimize_window();
}
