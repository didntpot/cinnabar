use super::*;
use crate::ui_runtime::forms::EngineFrame;
use json_ui::{LayoutEnv, ResolvedControl, TextMeasure, TextureMeta, TextureSource, ViewState};
use server_experience::screen::Rect;

struct EmptyArt;
impl TextMeasure for EmptyArt {
    fn extent(&self, _: &str) -> [f64; 2] {
        [0.0; 2]
    }
}
impl TextureSource for EmptyArt {
    fn texture(&self, _: &str) -> Option<TextureMeta> {
        None
    }
}

/// Lays out an original button with primary and secondary actions.
fn button_frame(name: &str, size: [f64; 2]) -> EngineFrame {
    let props = serde_json::json!({
        "size": size, "anchor_from": "top_left", "anchor_to": "top_left",
        "button_mappings": [
            {"from_button_id":"button.menu_select", "to_button_id":name, "mapping_type":"pressed"},
            {"from_button_id":"button.menu_secondary_select", "to_button_id":format!("{name}.secondary"), "mapping_type":"pressed"}
        ]
    });
    let root = ResolvedControl {
        name: name.into(),
        control_type: Some("button".into()),
        base: None,
        unresolved_base: None,
        properties: props
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<std::collections::BTreeMap<_, _>>()
            .into(),
        children: Vec::new(),
        factory: None,
    };
    let (laid, report) = json_ui::layout_with(
        &root,
        [200.0, 100.0],
        &LayoutEnv {
            text: &EmptyArt,
            textures: &EmptyArt,
        },
        &ViewState::default(),
    );
    EngineFrame {
        identity: None,
        hits: json_ui::hit_regions(&laid).into(),
        report,
        cancel_target: None,
        origin: [0.0; 2],
        scale: 1.0,
        panel: None,
        edit_texts: Vec::new(),
        top: Vec::new(),
    }
}

/// Installs an overlay button beside a view that also has a full-screen dismiss hit.
fn overlapping_screens() -> UiPresentationRuntime {
    let mut presentation = crate::test_support::mini_engine_presentation();
    let files = Arc::new(screen::Files {
        namespace: "input_test".into(),
        templates: Default::default(),
        textures: Vec::new(),
    });
    presentation.set_mod_screens(Some(ModScreensInput {
        id: "input_test",
        files: &files,
        overlay: None,
        view: None,
        focus: None,
        data: &screen::Modal::default(),
    }));
    let screens = presentation.form_presentation.mod_screens.as_mut().unwrap();
    screens.overlay.frame = Some(button_frame("overlay", [20.0, 20.0]));
    screens.view.frame = Some(button_frame("dismiss", [200.0, 100.0]));
    screens.layout = Some(ScreenLayout {
        screen: "inventory".into(),
        size: GuiSize {
            width: 200.0,
            height: 100.0,
            scale: 1.0,
        },
        gui: Rect {
            x: 50.0,
            y: 20.0,
            width: 100.0,
            height: 60.0,
        },
        exclusions: Vec::new(),
        view: Some(Rect {
            x: 50.0,
            y: 20.0,
            width: 100.0,
            height: 60.0,
        }),
    });
    presentation
}

#[test]
fn overlay_primary_and_secondary_clicks_win_over_full_screen_dismiss_hits() {
    for secondary in [false, true] {
        let mut presentation = overlapping_screens();
        let click = |p: &mut UiPresentationRuntime, point, pressed, released| {
            if secondary {
                p.secondary_press_mod_screens(point, pressed, released)
            } else {
                p.press_mod_screens(point, pressed, released)
            }
        };
        let target = if secondary {
            "overlay.secondary"
        } else {
            "overlay"
        };
        assert_eq!(
            click(&mut presentation, Some([5.0, 5.0]), true, false),
            None
        );
        assert_eq!(
            click(&mut presentation, Some([5.0, 5.0]), false, true),
            Some((target.into(), None))
        );
        assert_eq!(
            click(&mut presentation, Some([5.0, 5.0]), true, false),
            None
        );
        assert_eq!(
            click(&mut presentation, Some([180.0, 80.0]), false, true),
            None
        );
        let target = if secondary {
            "dismiss.secondary"
        } else {
            "dismiss"
        };
        assert_eq!(
            click(&mut presentation, Some([180.0, 80.0]), true, false),
            None
        );
        assert_eq!(
            click(&mut presentation, Some([180.0, 80.0]), false, true),
            Some((target.into(), None))
        );
    }
}
