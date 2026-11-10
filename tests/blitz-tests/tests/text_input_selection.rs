//! Selecting in text inputs: a drag past the input's edge, and a long press.

use blitz_test_harness::Harness;

const PAGE: &str = r#"<html><body style="margin: 0">
    <input value="hello world, and the rest of a long line" style="position: absolute; left: 0; top: 0; width: 60px; height: 20px; padding: 0; border: 0; font-size: 16px">
</body></html>"#;

fn selected(harness: &Harness<blitz_html::HtmlDocument>) -> String {
    let input = harness.node("input");
    harness
        .base()
        .get_node(input)
        .unwrap()
        .element_data()
        .unwrap()
        .text_input_data()
        .unwrap()
        .editor
        .selected_text()
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn a_drag_keeps_selecting_past_the_input() {
    let mut harness = Harness::from_html(PAGE);
    // From the start of the text to well past the narrow input's right edge:
    // the text beyond the edge is selected too.
    harness.drag((1.0, 10.0), (390.0, 10.0), 13);
    assert_eq!(
        selected(&harness),
        "hello world, and the rest of a long line"
    );
}

#[test]
fn a_long_press_selects_the_word() {
    let mut harness = Harness::from_html(PAGE);
    harness.tap_at(5.0, 10.0);
    assert_eq!(selected(&harness), "");
    // Long enough that the press isn't a double tap.
    std::thread::sleep(std::time::Duration::from_millis(600));
    harness.touch_down(1, 5.0, 10.0);
    std::thread::sleep(std::time::Duration::from_millis(600));
    harness.touch_up(1, 5.0, 10.0);
    assert_eq!(selected(&harness), "hello");
}
