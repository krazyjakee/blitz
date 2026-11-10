//! An SVG background image tiles as `background-repeat` says, as a raster
//! one does (Blitz drew one tile).

use anyrender::render_to_buffer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_dom::DocumentConfig;
use blitz_dom::node::{ImageData, SvgImageData};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_paint::paint_scene;
use blitz_traits::shell::{ColorScheme, Viewport};
use std::sync::Arc;

/// A 40×40 tile: a red 20×20 square in its middle, white around it.
const TILE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40"><rect width="40" height="40" fill="#fff"/><rect x="10" y="10" width="20" height="20" fill="#f00"/></svg>"##;

/// A 40×40 tile whose only content is a filter: flooding it green. Drawn
/// as vectors (without the filter) it would be a black square.
const FILTERED: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40"><filter id="f" x="0" y="0" width="100%" height="100%"><feFlood flood-color="#0f0"/></filter><rect width="40" height="40" filter="url(#f)"/></svg>"##;

fn render(background: &str) -> Vec<u8> {
    render_tile(background, TILE)
}

fn render_tile(background: &str, tile: &str) -> Vec<u8> {
    let html = format!(
        r#"<html><body style="margin:0; background:#0000ff;">
            <div id="box" style="width:120px; height:120px; background: {background};"></div>
        </body></html>"#
    );
    let mut doc = HtmlDocument::from_html(
        &html,
        DocumentConfig {
            viewport: Some(Viewport::new(120, 120, 1.0, ColorScheme::Light)),
            html_parser_provider: Some(Arc::new(HtmlProvider) as _),
            ..Default::default()
        },
    );
    doc.resolve(0.0);
    let box_id = doc.query_selector("#box").unwrap().expect("#box");
    {
        let svg = SvgImageData::from_data(tile.as_bytes(), &usvg::Options::default()).unwrap();
        let node = doc.get_node_mut(box_id).unwrap();
        let el = node.element_data_mut().unwrap();
        for layer in el.background_images.iter_mut().flatten() {
            layer.status = blitz_dom::node::Status::Ok;
            layer.image = ImageData::Svg(svg.clone());
        }
    }
    doc.resolve(0.0);
    render_to_buffer::<VelloCpuImageRenderer, _>(
        |scene| paint_scene(scene, doc.as_mut(), 1.0, 120, 120, 0, 0),
        120,
        120,
    )
}

fn pixel(buffer: &[u8], x: usize, y: usize) -> [u8; 3] {
    let idx = (y * 120 + x) * 4;
    [buffer[idx], buffer[idx + 1], buffer[idx + 2]]
}

const RED: [u8; 3] = [255, 0, 0];
const WHITE: [u8; 3] = [255, 255, 255];
const BLUE: [u8; 3] = [0, 0, 255];

#[test]
fn an_svg_background_repeats_across_the_box() {
    let buffer = render("url('https://example.com/tile.svg')");
    // The middle of every tile, the first and the last.
    for (x, y) in [(20, 20), (60, 20), (100, 20), (20, 100), (100, 100)] {
        assert_eq!(pixel(&buffer, x, y), RED, "tile at {x},{y}");
    }
    assert_eq!(pixel(&buffer, 42, 42), WHITE);
}

#[test]
fn an_svg_background_tiles_at_its_background_size() {
    let buffer = render("url('https://example.com/tile.svg') 0 0 / 60px 60px");
    assert_eq!(pixel(&buffer, 30, 30), RED);
    assert_eq!(pixel(&buffer, 90, 90), RED);
}

#[test]
fn a_no_repeat_svg_background_draws_once() {
    let buffer = render("url('https://example.com/tile.svg') no-repeat");
    assert_eq!(pixel(&buffer, 20, 20), RED);
    assert_eq!(pixel(&buffer, 60, 20), BLUE);
}

#[test]
fn an_svg_background_with_a_filter_is_drawn_with_it() {
    let buffer = render_tile("url('https://example.com/flood.svg')", FILTERED);
    for (x, y) in [(20, 20), (100, 100)] {
        assert_eq!(pixel(&buffer, x, y), [0, 255, 0], "tile at {x},{y}");
    }
}
