use blitz_dom::{Document, DocumentConfig};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_traits::events::{BlitzAccessibilityAction, BlitzAccessibilityEvent, UiEvent};
use blitz_traits::shell::{ColorScheme, Viewport};
use std::sync::Arc;

fn document(body: &str) -> HtmlDocument {
    let mut document = HtmlDocument::from_html(
        &format!("<html><head></head><body style=\"margin: 0\">{body}</body></html>"),
        DocumentConfig {
            viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
            html_parser_provider: Some(Arc::new(HtmlProvider) as _),
            ..Default::default()
        },
    );
    document.resolve(0.0);
    document
}

fn act(document: &mut HtmlDocument, id: &str, action: BlitzAccessibilityAction) {
    let target = document.get_element_by_id(id).expect("no such id");
    document.handle_ui_event(UiEvent::Accessibility(BlitzAccessibilityEvent {
        target,
        action,
    }));
    document.resolve(0.0);
}

#[test]
fn focus_moves_the_focus() {
    let mut doc = document(r#"<button id="a">A</button><button id="b">B</button>"#);
    act(&mut doc, "b", BlitzAccessibilityAction::Focus);
    assert_eq!(doc.get_focussed_node_id(), doc.get_element_by_id("b"));
}

#[test]
fn click_runs_the_default_action_and_keeps_the_focus() {
    let mut doc = document(r#"<input id="box" type="checkbox"><button id="button">B</button>"#);
    act(&mut doc, "box", BlitzAccessibilityAction::Click);
    let checkbox = doc.get_element_by_id("box").unwrap();
    assert_eq!(
        doc.get_node(checkbox)
            .unwrap()
            .element_data()
            .unwrap()
            .checkbox_input_checked(),
        Some(true)
    );
    assert_eq!(doc.get_focussed_node_id(), Some(checkbox));
    // A button's click has no default action: it keeps the focus it was given.
    act(&mut doc, "button", BlitzAccessibilityAction::Click);
    assert_eq!(doc.get_focussed_node_id(), doc.get_element_by_id("button"));
    assert!(doc.has_changes());
}

#[test]
fn scroll_into_view_scrolls() {
    let mut doc = document(r#"<div style="height: 3000px"></div><button id="far">Far</button>"#);
    assert_eq!(doc.viewport_scroll().y, 0.0);
    act(&mut doc, "far", BlitzAccessibilityAction::ScrollIntoView);
    assert!(doc.viewport_scroll().y > 2000.0);
}
