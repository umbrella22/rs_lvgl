//! File navigation from application-supplied directory entries. The platform
//! performs filesystem I/O in App::update or its own task and supplies the
//! resulting entries. Paths and filenames are displayed without normalization.
//! This composition replaces the deprecated native lv_file_explorer widget.
use super::list::{self, ListItem};
use crate::{symbol, Key, Node, Size, Style, Theme};
use alloc::string::String;

/// A stable entry identity and its activation message. `name` is display text;
/// the application resolves the identity to its filesystem path.
pub struct FileEntry<M> {
    pub id: u64,
    pub name: String,
    pub directory: bool,
    pub activate: M,
}
impl<M> FileEntry<M> {
    pub fn file(id: u64, name: impl Into<String>, activate: M) -> Self {
        Self {
            id,
            name: name.into(),
            directory: false,
            activate,
        }
    }
    pub fn directory(id: u64, name: impl Into<String>, activate: M) -> Self {
        Self {
            id,
            name: name.into(),
            directory: true,
            activate,
        }
    }
}
/// A location label, optional parent action and scrollable directory contents.
/// Entry IDs must be unique; duplicate IDs produce the usual DuplicateKey error.
/// With `input-keypad`, entries join an explicitly enabled UI focus group and
/// Enter emits the same activation message as a pointer click.
pub fn view<M: Clone + 'static>(
    theme: &Theme,
    location: &str,
    parent: Option<M>,
    entries: impl IntoIterator<Item = FileEntry<M>>,
) -> Node<M> {
    let mut header = Node::row().style(Style::body(theme).gap(8).width(Size::Percent(100)));
    if let Some(parent) = parent {
        header = header.child(
            Node::button(symbol::UP)
                .style(Style::default().shadow_width(0))
                .on_click(parent),
        );
    }
    header = header.child(Node::label(location).style(Style::body(theme).flex_grow(1)));
    Node::column()
        .style(
            Style::neutral()
                .width(Size::Percent(100))
                .height(Size::Percent(100)),
        )
        .child(header.key(Key::Named("location")))
        .child(
            list::list(
                theme,
                entries.into_iter().map(|entry| {
                    let row = ListItem::new(entry.name)
                        .icon(if entry.directory {
                            symbol::DIRECTORY
                        } else {
                            symbol::FILE
                        })
                        .chevron(entry.directory)
                        .on_click(entry.activate)
                        .view(theme)
                        .key(Key::Id(entry.id));
                    #[cfg(feature = "input-keypad")]
                    let row = row.focusable(true).state_style(
                        crate::State::FOCUSED,
                        Style::default()
                            .outline_width(1)
                            .outline_color(theme.accent),
                    );
                    row
                }),
            )
            .style(Style::default().flex_grow(1))
            .key(Key::Named("entries")),
        )
}
