#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

rust_i18n::i18n!("locales", fallback = "en");

mod auth;

mod app_init;
mod home;
mod home_tab;
mod license;
pub mod new_connection;
mod onetcli_app;
mod setting_tab;
mod settings;
mod update;
mod tray;
mod user_avatar;

use crate::onetcli_app::OnetCliApp;
use db::GlobalDbState;
use gpui::*;
use rust_i18n::t;

use gpui_component::Root;
use gpui_component_assets::Assets;
use image::load_from_memory;

fn main() {
    if update::handle_update_command() {
        return;
    }

    let app = Application::new()
        .with_assets(Assets)
        .with_quit_mode(QuitMode::LastWindowClosed);

    app.run(move |cx| {
        onetcli_app::init(cx);

        setting_tab::init_settings(cx);
        let db_state = GlobalDbState::new();
        db_state.start_cleanup_task(cx);
        cx.set_global(db_state);

        db_view::init_ask_ai_notifier(cx);

        let mut window_size = size(px(1600.0), px(1200.0));
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;
            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            #[cfg(not(target_os = "linux"))]
            titlebar: Some(gpui_component::TitleBar::title_bar_options()),
            window_min_size: Some(Size {
                width: px(640.),
                height: px(480.),
            }),
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            kind: WindowKind::Normal,
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(options, |window, cx| {
                window.activate_window();
                let window_handle = window.window_handle();
                window.on_window_should_close(cx, move |window, cx| {
                    let settings = setting_tab::AppSettings::global(cx);
                    match settings.close_behavior.as_str() {
                        "quit" => true,
                        "prompt" => {
                            let title = t!("App.ClosePrompt.title");
                            let message = t!("App.ClosePrompt.message");
                            let minimize_label = t!("App.ClosePrompt.minimize");
                            let quit_label = t!("App.ClosePrompt.quit");
                            let answer = window.prompt(
                                PromptLevel::Warning,
                                title.as_ref(),
                                Some(message.as_ref()),
                                &[minimize_label.as_ref(), quit_label.as_ref()],
                                cx,
                            );

                            cx.spawn(async move |cx| {
                                let should_quit = answer.await.ok().map(|idx| idx == 1).unwrap_or(false);
                                if should_quit {
                                    let _ = cx.update_window(window_handle, |_, window, _| {
                                        window.remove_window();
                                    });
                                } else {
                                    let _ = cx.update_window(window_handle, |_, window, _| {
                                        window.minimize_window();
                                    });
                                }
                            })
                            .detach();
                            false
                        }
                        "tray" => {
                            tray::hide_to_tray(window);
                            false
                        }
                        _ => {
                            tray::hide_to_tray(window);
                            false
                        }
                    }
                });
                app_init::init_window_systems(window, cx);
                if let Ok(icon) = icon_from_embedded_ico() {
                    let _ = tray::install_tray(cx, icon);
                }
                update::schedule_update_check(window, cx);
                let view = cx.new(|cx| OnetCliApp::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

fn icon_from_embedded_ico() -> anyhow::Result<tray_icon::Icon> {
    let bytes = include_bytes!("../../resources/windows/onetcli.ico");
    let image = load_from_memory(bytes)?.into_rgba8();
    let (width, height) = image.dimensions();
    Ok(tray_icon::Icon::from_rgba(image.into_raw(), width, height)?)
}
