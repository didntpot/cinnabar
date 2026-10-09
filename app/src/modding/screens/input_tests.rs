//! Input regressions across the real inventory suppression and mod-screen paths.
use super::*;
use crate::ui_runtime::drive_chat_keyboard_input;
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, NativeKey},
        mouse::AccumulatedMouseMotion,
    },
    time::Real,
    window::{CursorOptions, PrimaryWindow},
};
use client_ui::test_support::{fixture_font, inventory_session};

const MOD_ID: &str = "input_test";
const VIEW: &str = "ui/view.json";
const ACTION: &str = "input_test.show";

#[derive(Resource, Default)]
struct Observed {
    modifiers: KeyModifiers,
    keys: Vec<ModEvent>,
}
#[derive(Resource)]
struct Host(ModHost);

/// Creates a focused inventory with the production keyboard suppression system.
fn input_app() -> (App, Entity) {
    let mut player = crate::player_runtime::PlayerRuntime::new(1);
    let mut ui = inventory_session(&mut player);
    ui.toggle_inventory(&mut player);
    assert!(ui.inventory_open());
    let mut app = App::new();
    app.add_message::<KeyboardInput>()
        .add_message::<MouseButtonInput>()
        .add_message::<MouseWheel>()
        .init_resource::<Time<Real>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<AccumulatedMouseMotion>()
        .init_resource::<Observed>()
        .insert_resource(player)
        .insert_resource(ui)
        .insert_resource(UiPresentationRuntime::new(fixture_font()).unwrap());
    let window = app
        .world_mut()
        .spawn((
            Window {
                focused: true,
                ..Default::default()
            },
            CursorOptions::default(),
            PrimaryWindow,
        ))
        .id();
    (app, window)
}

/// Sends one physical keyboard edge and lets the input systems process it.
fn send(app: &mut App, window: Entity, key_code: KeyCode, state: ButtonState, text: Option<&str>) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    if state.is_pressed() {
        keys.press(key_code);
    } else {
        keys.release(key_code);
    }
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state,
        text: text.map(Into::into),
        repeat: false,
        window,
    });
    app.update();
}

/// Observes the modifiers that the mod adapter reads after inventory suppression.
fn observe_modifiers(
    mut input: ScreenInput,
    windows: Query<&Window>,
    mut observed: ResMut<Observed>,
) {
    observed.modifiers = input.modifiers(windows.single().unwrap().focused);
}

#[test]
fn mod_modifiers_survive_inventory_suppression_and_release_independently() {
    let (mut app, window) = input_app();
    app.add_systems(
        Update,
        (drive_chat_keyboard_input, observe_modifiers).chain(),
    );
    for key in [
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::ShiftLeft,
        KeyCode::AltRight,
    ] {
        send(&mut app, window, key, ButtonState::Pressed, None);
    }
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .get_pressed()
            .next()
            .is_none()
    );
    assert_eq!(
        app.world().resource::<Observed>().modifiers,
        KeyModifiers {
            ctrl: true,
            shift: true,
            alt: true
        }
    );
    send(
        &mut app,
        window,
        KeyCode::ControlLeft,
        ButtonState::Released,
        None,
    );
    app.update();
    assert!(app.world().resource::<Observed>().modifiers.ctrl);
    for key in [KeyCode::ControlRight, KeyCode::ShiftLeft, KeyCode::AltRight] {
        send(&mut app, window, key, ButtonState::Released, None);
    }
    assert_eq!(
        app.world().resource::<Observed>().modifiers,
        KeyModifiers::default()
    );
    send(
        &mut app,
        window,
        KeyCode::ControlLeft,
        ButtonState::Pressed,
        None,
    );
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    app.update();
    assert_eq!(
        app.world().resource::<Observed>().modifiers,
        KeyModifiers::default()
    );
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    app.update();
    assert_eq!(
        app.world().resource::<Observed>().modifiers,
        KeyModifiers::default()
    );
}

/// Draws an original mod view over the inventory using the installed UI fixture.
fn draw_view(app: &mut App) -> bool {
    let Some(mut presentation) = client_ui::test_support::engine_presentation() else {
        eprintln!(
            "skipping {}: missing installed UI carrier (make assets)",
            std::thread::current()
                .name()
                .unwrap_or("mod screen input test")
        );
        return false;
    };
    let files = Arc::new(server_experience::screen::Files {
        namespace: MOD_ID.into(),
        templates: [(VIEW.into(), serde_json::to_vec(&serde_json::json!({"namespace":MOD_ID,"view":{"type":"panel","size":[100,80],"controls":[{"title":{"type":"label","text":"View"}}]}})).unwrap())].into(),
        textures: Vec::new(),
    });
    let view = VIEW.to_owned();
    presentation.set_mod_screens(Some(ModScreensInput {
        id: MOD_ID,
        files: &files,
        overlay: None,
        view: Some(&view),
        focus: None,
        data: &server_experience::screen::Modal::default(),
    }));
    presentation
        .build(
            app.world()
                .resource::<crate::player_runtime::PlayerRuntime>(),
            app.world().resource::<UiRuntime>(),
            0,
            [1280, 720],
            ui::DpiScale::new(1.0).unwrap(),
        )
        .unwrap();
    assert!(presentation.mod_view_shown());
    app.insert_resource(presentation);
    true
}

/// Delivers declared mod keys through the production adapter after vanilla input.
fn observe_keys(
    mut input: ScreenInput,
    mut host: ResMut<Host>,
    player: Res<crate::player_runtime::PlayerRuntime>,
    ui: Res<UiRuntime>,
    mut presentation: ResMut<UiPresentationRuntime>,
    mut observed: ResMut<Observed>,
) {
    let modifiers = input.modifiers(true);
    let keys = [KeyDecl {
        id: ACTION.into(),
        key: "r".into(),
        modifiers: Vec::new(),
        label: "Show".into(),
    }];
    key_events(
        &mut host.0,
        &player,
        &ui,
        &mut presentation,
        KeyFrame {
            keys: &keys,
            menu: None,
            cursor: None,
            pressed: false,
            modifiers,
        },
        &mut input,
        &mut observed.keys,
    );
}

#[test]
fn hidden_vanilla_text_fields_do_not_receive_typing_under_a_mod_view() {
    for anvil in [false, true] {
        let (mut app, window) = input_app();
        if !draw_view(&mut app) {
            return;
        }
        let mut ui = app.world_mut().resource_mut::<UiRuntime>();
        ui.screen_state_mut().search = "Search".into();
        ui.screen_state_mut().anvil_name = "Name".into();
        ui.screen_state_mut().search_focused = !anvil;
        ui.screen_state_mut().anvil_focused = anvil;
        app.add_systems(Update, drive_chat_keyboard_input);
        send(
            &mut app,
            window,
            KeyCode::KeyR,
            ButtonState::Pressed,
            Some("r"),
        );
        let ui = app.world().resource::<UiRuntime>();
        assert_eq!(ui.screen_state().search, "Search");
        assert_eq!(ui.screen_state().anvil_name, "Name");
        assert!(ui.inventory_open());
    }
}

#[test]
fn a_shown_mod_view_releases_declared_keys_from_hidden_vanilla_text_focus() {
    let (mut app, window) = input_app();
    if !draw_view(&mut app) {
        return;
    }
    app.world_mut()
        .resource_mut::<UiRuntime>()
        .screen_state_mut()
        .search_focused = true;
    let dir = tempfile::tempdir().unwrap();
    let component = dir.path().join("empty.wat");
    std::fs::write(&component, r#"(component (core module $m (func (export "init")) (func (export "frame"))) (core instance $i (instantiate $m)) (func (export "init") (canon lift (core func $i "init"))) (func (export "frame") (canon lift (core func $i "frame"))))"#).unwrap();
    app.insert_resource(Host(ModHost::load(&component).unwrap()));
    app.add_systems(Update, (drive_chat_keyboard_input, observe_keys).chain());
    send(&mut app, window, KeyCode::KeyR, ButtonState::Pressed, None);
    assert_eq!(
        app.world().resource::<Observed>().keys,
        [ModEvent::Key {
            id: ACTION.into(),
            hovered: None,
            row: None
        }]
    );
    app.world_mut().resource_mut::<Observed>().keys.clear();
    app.world_mut()
        .resource_mut::<UiPresentationRuntime>()
        .set_mod_screens(None);
    send(
        &mut app,
        window,
        KeyCode::KeyR,
        ButtonState::Pressed,
        Some("r"),
    );
    assert!(app.world().resource::<Observed>().keys.is_empty());
    assert_eq!(
        app.world().resource::<UiRuntime>().screen_state().search,
        "r"
    );
}
