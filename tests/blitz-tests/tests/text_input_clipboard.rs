//! Clipboard shortcuts and commands in text fields, read-only fields and
//! passwords.

use blitz_dom::{
    BaseDocument, Document, DocumentConfig, EventDriver, EventHandler, TextFieldCommand,
};
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_traits::events::{
    BlitzAccessibilityAction, BlitzAccessibilityEvent, BlitzKeyEvent, DomEvent, EventState,
    KeyState, UiEvent,
};
use blitz_traits::node_id::NodeId;
use blitz_traits::shell::{ClipboardError, ColorScheme, ShellProvider, Viewport};
use keyboard_types::{Code, Key, Location, Modifiers};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Shell {
    clipboard: Mutex<Option<String>>,
    paste_requests: Mutex<usize>,
}

impl ShellProvider for Shell {
    fn set_clipboard_text(&self, text: String) -> Result<(), ClipboardError> {
        *self.clipboard.lock().unwrap() = Some(text);
        Ok(())
    }
    fn request_clipboard_text(&self) -> Result<(), ClipboardError> {
        *self.paste_requests.lock().unwrap() += 1;
        Ok(())
    }
}

struct Recorder<'a>(&'a mut Vec<String>);

impl EventHandler for Recorder<'_> {
    fn handle_event(
        &mut self,
        _chain: &[NodeId],
        event: &mut DomEvent,
        _doc: &mut dyn Document,
        _event_state: &mut EventState,
    ) {
        self.0.push(event.data.name().to_owned());
    }
}

#[cfg(target_os = "macos")]
const ACTION: Modifiers = Modifiers::META;
#[cfg(not(target_os = "macos"))]
const ACTION: Modifiers = Modifiers::CONTROL;

struct Page {
    doc: HtmlDocument,
    shell: Arc<Shell>,
    events: Vec<String>,
}

impl Page {
    fn new() -> Self {
        let shell = Arc::new(Shell::default());
        let mut doc = HtmlDocument::from_html(
            r#"<html><body>
                <input id="text" value="hello">
                <input id="locked" value="fixed" readonly>
                <input id="secret" type="password" value="hunter2">
            </body></html>"#,
            DocumentConfig {
                viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
                html_parser_provider: Some(Arc::new(HtmlProvider) as _),
                shell_provider: Some(shell.clone()),
                ..Default::default()
            },
        );
        doc.resolve(0.0);
        Page {
            doc,
            shell,
            events: Vec::new(),
        }
    }

    fn id(&self, id: &str) -> NodeId {
        self.doc.get_element_by_id(id).unwrap()
    }

    fn send(&mut self, event: UiEvent) {
        let base: &mut BaseDocument = &mut self.doc;
        EventDriver::new(base, Recorder(&mut self.events)).handle_ui_event(event);
        self.doc.resolve(0.0);
    }

    fn focus(&mut self, id: &str) {
        let target = self.id(id);
        self.send(UiEvent::Accessibility(BlitzAccessibilityEvent {
            target,
            action: BlitzAccessibilityAction::Focus,
        }));
        self.events.clear();
    }

    fn key(&mut self, key: &str, modifiers: Modifiers) {
        for state in [KeyState::Pressed, KeyState::Released] {
            let event = BlitzKeyEvent {
                key: Key::Character(key.into()),
                code: Code::Unidentified,
                modifiers,
                location: Location::Standard,
                is_auto_repeating: false,
                is_composing: false,
                state,
                text: Some(key.into()),
            };
            self.send(match state {
                KeyState::Pressed => UiEvent::KeyDown(event),
                KeyState::Released => UiEvent::KeyUp(event),
            });
        }
    }

    fn command(&mut self, id: &str, command: TextFieldCommand) {
        let node = self.id(id);
        let events = &mut self.events;
        assert!(self.doc.text_field_command(node, command, |event| {
            events.push(event.data.name().to_owned())
        }));
        self.doc.resolve(0.0);
    }

    fn value(&self, id: &str) -> String {
        let node = self.doc.get_node(self.id(id)).unwrap();
        node.element_data()
            .unwrap()
            .text_input_data()
            .unwrap()
            .editor
            .text()
            .to_string()
    }

    fn clipboard(&self) -> Option<String> {
        self.shell.clipboard.lock().unwrap().clone()
    }

    fn inputs(&self) -> usize {
        self.events.iter().filter(|name| *name == "input").count()
    }
}

#[test]
fn copy_is_not_an_edit_and_shortcuts_ignore_shift() {
    let mut page = Page::new();
    page.focus("text");
    page.key("a", ACTION);
    page.key("C", ACTION | Modifiers::SHIFT);
    assert_eq!(page.clipboard().as_deref(), Some("hello"));
    assert_eq!(page.inputs(), 0, "{:?}", page.events);
    page.key("x", ACTION);
    assert_eq!(page.value("text"), "");
    assert_eq!(page.inputs(), 1);
    page.key("v", ACTION);
    assert_eq!(*page.shell.paste_requests.lock().unwrap(), 1);
}

#[test]
fn a_single_line_field_drops_pasted_line_breaks() {
    let mut page = Page::new();
    page.focus("text");
    page.key("a", ACTION);
    page.send(UiEvent::Paste("one\r\ntwo\nthree".into()));
    assert_eq!(page.value("text"), "onetwothree");
}

#[test]
fn a_read_only_field_selects_and_copies_but_takes_no_edits() {
    let mut page = Page::new();
    page.focus("locked");
    page.key("z", Modifiers::empty());
    page.key("a", ACTION);
    page.key("x", ACTION);
    page.send(UiEvent::Paste("pasted".into()));
    assert_eq!(page.value("locked"), "fixed");
    assert_eq!(page.inputs(), 0);
    page.key("c", ACTION);
    assert_eq!(page.clipboard().as_deref(), Some("fixed"));
}

#[test]
fn a_password_is_not_copied_out() {
    let mut page = Page::new();
    page.focus("secret");
    page.key("a", ACTION);
    page.key("c", ACTION);
    page.key("x", ACTION);
    assert_eq!(page.clipboard(), None);
    assert_eq!(page.value("secret"), "hunter2");
}

#[test]
fn context_menu_commands() {
    let mut page = Page::new();
    page.command("text", TextFieldCommand::SelectAll);
    page.command("text", TextFieldCommand::Copy);
    assert_eq!(page.clipboard().as_deref(), Some("hello"));
    page.command("text", TextFieldCommand::Cut);
    assert_eq!(page.value("text"), "");
    assert_eq!(page.inputs(), 1);
    // Paste focuses the field: the clipboard's text arrives there.
    page.command("text", TextFieldCommand::Paste);
    assert_eq!(page.doc.get_focussed_node_id(), Some(page.id("text")));
    assert_eq!(*page.shell.paste_requests.lock().unwrap(), 1);
    page.command("locked", TextFieldCommand::Paste);
    assert_eq!(
        *page.shell.paste_requests.lock().unwrap(),
        1,
        "read-only takes no paste"
    );
}
