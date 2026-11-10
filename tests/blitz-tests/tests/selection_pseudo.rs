//! `::selection` gives selected text its background, and descendants take
//! their ancestor's (highlight inheritance).

use blitz_test_harness::Harness;

#[test]
fn selection_backgrounds_come_from_the_selection_pseudo_element() {
    let harness = Harness::from_html(
        r#"<html><head><style>
            #themed::selection { background-color: rgb(10, 20, 30); }
        </style></head><body>
            <div id="themed"><input id="inside"><p id="text">Some text</p></div>
            <input id="plain">
        </body></html>"#,
    );
    let background = |selector: &str| {
        harness
            .base()
            .get_node(harness.node(selector))
            .unwrap()
            .selection_background()
            .map(|color| {
                color
                    .to_color_space(style::color::ColorSpace::Srgb)
                    .raw_components()
                    .map(|c| (c * 255.0).round())
            })
    };
    assert_eq!(background("#themed"), Some([10.0, 20.0, 30.0, 255.0]));
    assert_eq!(background("#inside"), Some([10.0, 20.0, 30.0, 255.0]));
    assert_eq!(background("#text"), Some([10.0, 20.0, 30.0, 255.0]));
    assert_eq!(background("#plain"), None);
}
