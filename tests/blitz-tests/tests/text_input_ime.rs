//! IME composition in text inputs: the DOM events a composition fires, its
//! commit when the focus moves away, and the caret area the IME is told.

use blitz_dom::{BaseDocument, Document, DocumentConfig, EventDriver, EventHandler};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_traits::events::{
    BlitzAccessibilityAction, BlitzAccessibilityEvent, BlitzImeEvent, DomEvent, DomEventData,
    EventState, UiEvent,
};
use blitz_traits::node_id::NodeId;
use blitz_traits::shell::{ColorScheme, ShellProvider, TextInputPurpose, Viewport};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Shell {
    calls: Mutex<Vec<String>>,
}

impl ShellProvider for Shell {
    fn enable_ime(&self, purpose: TextInputPurpose, _x: f32, _y: f32, _w: f32, _h: f32) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("enable {purpose:?}"));
    }
    fn set_ime_enabled(&self, is_enabled: bool) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("enabled {is_enabled}"));
    }
    fn set_ime_cursor_area(&self, x: f32, y: f32, width: f32, height: f32) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("area {x:.0} {y:.0} {width:.0} {height:.0}"));
    }
}

/// Records the events script would hear.
struct Recorder<'a>(&'a mut Vec<String>);

impl EventHandler for Recorder<'_> {
    fn handle_event(
        &mut self,
        _chain: &[NodeId],
        event: &mut DomEvent,
        _doc: &mut dyn Document,
        _event_state: &mut EventState,
    ) {
        let detail = match &event.data {
            DomEventData::CompositionStart(data)
            | DomEventData::CompositionUpdate(data)
            | DomEventData::CompositionEnd(data) => format!(" {:?}", data.data),
            DomEventData::Input(data) => format!(" {:?}", data.value),
            DomEventData::KeyDown(data) => format!(" composing={}", data.is_composing),
            _ => String::new(),
        };
        self.0.push(format!("{}{detail}", event.data.name()));
    }
}

fn document(shell: Arc<Shell>) -> HtmlDocument {
    let mut document = HtmlDocument::from_html(
        r#"<html><body style="margin: 0">
            <input id="search" style="position: absolute; left: 100px; top: 50px; width: 200px; height: 20px; padding: 0; border: 0; font-size: 16px">
            <input id="other" type="password">
        </body></html>"#,
        DocumentConfig {
            viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
            html_parser_provider: Some(Arc::new(HtmlProvider) as _),
            shell_provider: Some(shell),
            ..Default::default()
        },
    );
    document.resolve(0.0);
    document
}

fn send(doc: &mut HtmlDocument, events: &mut Vec<String>, event: UiEvent) {
    let base: &mut BaseDocument = doc;
    EventDriver::new(base, Recorder(events)).handle_ui_event(event);
    doc.resolve(0.0);
}

fn value(doc: &HtmlDocument, id: &str) -> String {
    let node = doc.get_node(doc.get_element_by_id(id).unwrap()).unwrap();
    node.element_data()
        .unwrap()
        .text_input_data()
        .unwrap()
        .editor
        .text()
        .to_string()
}

fn focus(doc: &mut HtmlDocument, events: &mut Vec<String>, id: &str) {
    let target = doc.get_element_by_id(id).unwrap();
    send(
        doc,
        events,
        UiEvent::Accessibility(BlitzAccessibilityEvent {
            target,
            action: BlitzAccessibilityAction::Focus,
        }),
    );
}

#[test]
fn a_composition_fires_its_events_around_the_edit() {
    let shell = Arc::new(Shell::default());
    let mut doc = document(shell.clone());
    let mut events = Vec::new();
    focus(&mut doc, &mut events, "search");
    events.clear();

    send(
        &mut doc,
        &mut events,
        UiEvent::Ime(BlitzImeEvent::Preedit("ni".into(), Some((2, 2)))),
    );
    send(
        &mut doc,
        &mut events,
        UiEvent::Ime(BlitzImeEvent::Preedit("你".into(), Some((3, 3)))),
    );
    assert_eq!(
        value(&doc, "search"),
        "",
        "the preedit isn't the field's value yet"
    );
    send(
        &mut doc,
        &mut events,
        UiEvent::Ime(BlitzImeEvent::Commit("你".into())),
    );
    assert_eq!(
        events,
        [
            r#"compositionstart """#,
            r#"compositionupdate "ni""#,
            r#"compositionupdate "你""#,
            r#"input "你""#,
            r#"compositionend "你""#,
        ]
    );
    assert_eq!(value(&doc, "search"), "你");
}

#[test]
fn moving_the_focus_commits_the_composition() {
    let shell = Arc::new(Shell::default());
    let mut doc = document(shell.clone());
    let mut events = Vec::new();
    focus(&mut doc, &mut events, "search");
    send(
        &mut doc,
        &mut events,
        UiEvent::Ime(BlitzImeEvent::Preedit("かな".into(), None)),
    );
    events.clear();
    focus(&mut doc, &mut events, "other");
    assert_eq!(
        &events[..2],
        [r#"input "かな""#, r#"compositionend "かな""#]
    );
    assert_eq!(value(&doc, "search"), "かな");
    // The password field asks the IME for a password purpose.
    assert!(
        shell
            .calls
            .lock()
            .unwrap()
            .contains(&"enable Password".to_string())
    );
}

#[test]
fn the_ime_follows_the_caret() {
    let shell = Arc::new(Shell::default());
    let mut doc = document(shell.clone());
    let mut events = Vec::new();
    focus(&mut doc, &mut events, "search");
    let areas = |shell: &Shell| {
        shell
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| call.starts_with("area"))
            .cloned()
            .collect::<Vec<_>>()
    };
    let first = areas(&shell);
    let Some(at_start) = first.last().cloned() else {
        panic!("no caret area was sent: {:?}", shell.calls.lock().unwrap());
    };
    // The caret starts at the field's left edge.
    assert!(at_start.starts_with("area 100 "), "{at_start}");
    send(
        &mut doc,
        &mut events,
        UiEvent::Ime(BlitzImeEvent::Commit("abc".into())),
    );
    let after = areas(&shell);
    let moved = after.last().unwrap();
    // Parley bounds the line up to the caret (an area the candidate window
    // keeps clear of): typing moves its right edge.
    let right = |area: &str| -> f32 {
        let v: Vec<f32> = area
            .split(' ')
            .skip(1)
            .map(|v| v.parse().unwrap())
            .collect();
        v[0] + v[2]
    };
    assert!(right(moved) > right(&at_start), "{at_start} then {moved}");
}
