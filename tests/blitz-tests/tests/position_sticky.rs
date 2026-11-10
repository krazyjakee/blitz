//! `position: sticky`: boxes laid out in flow, then shifted to stay inside
//! their scroller's inset edges without leaving their containing block. The
//! shift shows in geometry APIs, hit testing and paint.

use anyrender::render_to_buffer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_dom::{DocumentConfig, ScrollBehavior};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_paint::paint_scene;
use blitz_test_harness::Harness;
use blitz_traits::shell::{ColorScheme, Viewport};
use std::sync::Arc;

/// A sticky header 50px down a 200px-tall scroller, in a 400px section that
/// ends at 450px, followed by more content.
const SCROLLER: &str = r#"<html><body style="margin: 0">
    <div id="scroller" style="height: 200px; overflow: auto">
        <div style="height: 50px"></div>
        <div id="section" style="height: 400px">
            <div id="header" style="position: sticky; top: 0; height: 20px"></div>
            <div id="body" style="height: 380px"></div>
        </div>
        <div style="height: 600px"></div>
    </div>
</body></html>"#;

/// A bar with `bottom: 8px` whose in-flow position (1000px) is below the
/// fold of the 600px viewport.
const BOTTOM_BAR: &str = r#"<html><body style="margin: 0">
    <div style="height: 1000px"></div>
    <div id="bar" style="position: sticky; bottom: 8px; height: 30px"></div>
    <div style="height: 500px"></div>
</body></html>"#;

fn client_y(harness: &Harness<HtmlDocument>, selector: &str) -> f64 {
    let id = harness.node(selector);
    harness.base().get_client_bounding_rect(id).unwrap().y
}

fn scroll_to(harness: &mut Harness<HtmlDocument>, selector: &str, y: f64) {
    let id = harness.node(selector);
    harness
        .base_mut()
        .scroll_to(id, 0.0, y, ScrollBehavior::Instant);
}

#[test]
fn a_sticky_header_sticks_to_the_scroller_top_and_comes_back() {
    let mut harness = Harness::from_html(SCROLLER);
    assert_eq!(client_y(&harness, "#header"), 50.0);

    // Scrolled past its in-flow position: it stays at the scroller's top,
    // straight away (no relayout needed) and after one.
    scroll_to(&mut harness, "#scroller", 100.0);
    assert_eq!(client_y(&harness, "#header"), 0.0);
    assert_eq!(client_y(&harness, "#body"), -30.0);
    harness.pump();
    assert_eq!(client_y(&harness, "#header"), 0.0);

    // Not yet reached: no shift.
    scroll_to(&mut harness, "#scroller", 30.0);
    assert_eq!(client_y(&harness, "#header"), 20.0);

    scroll_to(&mut harness, "#scroller", 0.0);
    assert_eq!(client_y(&harness, "#header"), 50.0);
}

/// A sticky box removed (a view navigated away from) and a scroll before
/// the next layout: the refresh skips it instead of indexing a slot that
/// is gone, and a node given its id later isn't shifted.
#[test]
fn a_removed_sticky_box_is_skipped_when_scrolling_before_the_next_layout() {
    let mut harness = Harness::from_html(SCROLLER);
    scroll_to(&mut harness, "#scroller", 100.0);
    assert_eq!(client_y(&harness, "#header"), 0.0);
    let header = harness.node("#header");
    harness.base_mut().mutate().remove_and_drop_node(header);
    // No layout in between: the scroll refreshes the sticky offsets.
    scroll_to(&mut harness, "#scroller", 120.0);
    scroll_to(&mut harness, "#scroller", 0.0);
    harness.pump();
    assert_eq!(client_y(&harness, "#body"), 50.0);
}

#[test]
fn a_sticky_header_stays_inside_its_containing_block() {
    let mut harness = Harness::from_html(SCROLLER);
    // The section ends at 450px: the 20px header can go no lower than 430px.
    scroll_to(&mut harness, "#scroller", 420.0);
    assert_eq!(client_y(&harness, "#header"), 0.0);
    scroll_to(&mut harness, "#scroller", 440.0);
    assert_eq!(client_y(&harness, "#header"), -10.0);
    scroll_to(&mut harness, "#scroller", 500.0);
    assert_eq!(client_y(&harness, "#header"), -70.0);
    assert_eq!(client_y(&harness, "#section"), -450.0);
}

#[test]
fn a_bottom_sticky_bar_sits_above_the_viewport_bottom() {
    let mut harness = Harness::from_html(BOTTOM_BAR);
    // 600px viewport - 8px - 30px.
    assert_eq!(client_y(&harness, "#bar"), 562.0);

    // Scrolled until its in-flow position is on screen: back in flow.
    harness.base_mut().scroll_viewport_by(0.0, -600.0);
    assert_eq!(harness.base().viewport_scroll().y, 600.0);
    assert_eq!(client_y(&harness, "#bar"), 400.0);

    harness.base_mut().scroll_viewport_by(0.0, 600.0);
    assert_eq!(client_y(&harness, "#bar"), 562.0);
}

#[test]
fn hit_testing_finds_a_stuck_box_where_it_is_drawn() {
    let mut harness = Harness::from_html(SCROLLER);
    let header = harness.node("#header");
    let body = harness.node("#body");
    assert_eq!(harness.hit_node(10.0, 60.0), header);

    scroll_to(&mut harness, "#scroller", 100.0);
    assert_eq!(harness.hit_node(10.0, 10.0), header);
    // Where the header was laid out is now the section's body.
    assert_eq!(harness.hit_node(10.0, 60.0), body);

    let mut harness = Harness::from_html(BOTTOM_BAR);
    let bar = harness.node("#bar");
    assert_eq!(harness.hit_node(10.0, 570.0), bar);
    assert_ne!(harness.hit_node(10.0, 550.0), bar);
    harness.pump();
    assert_eq!(harness.base().element_from_point(10.0, 570.0), Some(bar));
}

#[test]
fn a_sticky_table_header_follows_a_scrolling_table() {
    let mut harness = Harness::from_html(
        r#"<html><body style="margin: 0">
        <div id="scroller" style="height: 100px; overflow: auto">
            <table style="border-spacing: 0">
                <thead style="position: sticky; top: 0">
                    <tr><th id="th" style="height: 20px; padding: 0">Name</th></tr>
                </thead>
                <tbody>
                    <tr><td style="height: 300px; padding: 0">Row</td></tr>
                </tbody>
            </table>
        </div>
    </body></html>"#,
    );
    assert_eq!(client_y(&harness, "#th"), 0.0);
    // The row group has no box of its own: its shift moves its cells.
    scroll_to(&mut harness, "#scroller", 150.0);
    assert_eq!(client_y(&harness, "#th"), 0.0);
}

/// The shape of a virtualised list: a block table under a sticky, stacked
/// header, its rows absolutely placed in a tall body.
#[test]
fn a_virtual_list_header_stays_above_its_rows() {
    let mut harness = Harness::from_html(
        r#"<html><body style="margin: 0">
        <div id="scroller" style="height: 200px; overflow: auto">
            <table style="display: block; width: 100%">
                <thead id="head" style="position: sticky; top: 0; z-index: 20; display: block; height: 32px; background: white"></thead>
                <tbody style="position: relative; display: block; width: 100%; height: 4000px">
                    <tr id="row" style="position: absolute; top: 380px; left: 0; display: block; width: 100%; height: 40px"></tr>
                </tbody>
            </table>
        </div>
    </body></html>"#,
    );
    let head = harness.node("#head");
    let row = harness.node("#row");
    scroll_to(&mut harness, "#scroller", 390.0);
    harness.pump();
    assert_eq!(client_y(&harness, "#head"), 0.0);
    assert_eq!(client_y(&harness, "#row"), 32.0 + 380.0 - 390.0);
    // The row runs under the header: the header is on top.
    assert_eq!(harness.base().element_from_point(10.0, 25.0), Some(head));
    assert_eq!(harness.base().element_from_point(10.0, 50.0), Some(row));
}

fn pixel(doc: &mut HtmlDocument, x: usize, y: usize) -> [u8; 3] {
    let buffer = render_to_buffer::<VelloCpuImageRenderer, _>(
        |scene| paint_scene(scene, doc, 1.0, 100, 100, 0, 0),
        100,
        100,
    );
    let idx = (y * 100 + x) * 4;
    [buffer[idx], buffer[idx + 1], buffer[idx + 2]]
}

#[test]
fn a_stuck_box_and_its_stacked_descendants_paint_where_it_sticks() {
    let mut doc = HtmlDocument::from_html(
        r#"<html><body style="margin: 0; background: white">
        <div id="scroller" style="height: 100px; overflow: hidden">
            <div style="height: 50px"></div>
            <div style="height: 400px">
                <div style="position: sticky; top: 0; height: 20px; background: #ff0000">
                    <div style="position: absolute; z-index: 1; left: 0; top: 0; width: 50px; height: 20px; background: #0000ff"></div>
                </div>
            </div>
        </div>
    </body></html>"#,
        DocumentConfig {
            viewport: Some(Viewport::new(100, 100, 1.0, ColorScheme::Light)),
            html_parser_provider: Some(Arc::new(HtmlProvider) as _),
            ..Default::default()
        },
    );
    doc.resolve(0.0);
    assert_eq!(pixel(&mut doc, 75, 60), [255, 0, 0]);
    assert_eq!(pixel(&mut doc, 25, 60), [0, 0, 255]);
    assert_eq!(pixel(&mut doc, 75, 10), [255, 255, 255]);

    // A scroll repaints without a relayout: the header and the descendant
    // hoisted to the root's stacking context move together.
    let scroller = doc.query_selector("#scroller").unwrap().unwrap();
    doc.scroll_to(scroller, 0.0, 100.0, ScrollBehavior::Instant);
    assert_eq!(pixel(&mut doc, 75, 10), [255, 0, 0]);
    assert_eq!(pixel(&mut doc, 25, 10), [0, 0, 255]);
    assert_eq!(pixel(&mut doc, 75, 60), [255, 255, 255]);

    doc.scroll_to(scroller, 0.0, 0.0, ScrollBehavior::Instant);
    assert_eq!(pixel(&mut doc, 25, 60), [0, 0, 255]);
    assert_eq!(pixel(&mut doc, 25, 10), [255, 255, 255]);
}
