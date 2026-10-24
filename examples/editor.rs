//! A UTF-8 document editor with real file I/O, workspace navigation and dirty-close protection.
//! Run with an optional file: cargo run --example editor -- ./notes.txt
//! In debug builds, press F12 to inspect this window with Lemon DevTools.
use lemon::prelude::*;
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};
const MAX_BYTES: u64 = 1024 * 1024;

fn read_document(path: &Path) -> io::Result<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Documents must be at most 1 MiB",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "This file is not UTF-8 text"))
}

fn write_document(path: &Path, text: &str) -> io::Result<()> {
    if text.len() as u64 > MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Documents must be at most 1 MiB",
        ));
    }
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Choose a file name"))?;
    let temporary = path.with_file_name(format!(
        ".{}.lemon-{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&temporary, metadata.permissions())?;
        }
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Clone)]
struct Document {
    path: State<String>,
    text: State<String>,
    saved: State<String>,
    saved_path: State<Option<PathBuf>>,
    message: State<String>,
    pending: State<Option<PathBuf>>,
    confirm: State<bool>,
}

impl Document {
    fn new() -> Self {
        Self {
            path: State::new("Untitled.txt".into()),
            text: State::new(String::new()),
            saved: State::new(String::new()),
            saved_path: State::new(None),
            message: State::new("Open a text file or start writing a new document.".into()),
            pending: State::new(None),
            confirm: State::new(false),
        }
    }

    fn dirty(&self) -> bool {
        self.text.get() != self.saved.get()
            || self
                .saved_path
                .get()
                .is_some_and(|path| path.as_path() != Path::new(&self.path.get()))
    }

    fn open(&self, path: &Path) {
        match read_document(path) {
            Ok(text) => {
                self.path.set(path.to_string_lossy().into_owned());
                self.text.set(text.clone());
                self.saved.set(text);
                self.saved_path.set(Some(path.to_owned()));
                self.message.set("Opened successfully".into());
            }
            Err(error) => self.message.set(format!("Cannot open: {error}")),
        }
    }

    fn request_open(&self, path: PathBuf) {
        if self.dirty() {
            self.pending.set(Some(path));
            self.confirm.set(true);
        } else {
            self.open(&path);
        }
    }

    fn discard(&self) {
        self.text.set(self.saved.get());
        self.path.set(
            self.saved_path
                .get()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled.txt".into()),
        );
        self.message.set("Unsaved changes discarded".into());
    }

    fn save(&self) {
        let path = PathBuf::from(self.path.get());
        let result = (|| {
            if self.saved_path.get().as_ref() == Some(&path) {
                if read_document(&path)? != self.saved.get() {
                    return Err(io::Error::other("File changed on disk. Your edits are kept; choose a new path to save a copy."));
                }
            } else if path.exists() {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "File already exists. Open it first or choose a new path.",
                ));
            }
            write_document(&path, &self.text.get())
        })();
        match result {
            Ok(()) => {
                self.saved.set(self.text.get());
                self.saved_path.set(Some(path));
                self.message.set("Saved to disk".into());
            }
            Err(error) => self.message.set(format!("Cannot save: {error}")),
        }
    }
}

fn workspace(path: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ["txt", "md", "rs", "toml"].contains(&ext))
        })
        .collect();
    paths.sort();
    paths.truncate(200);
    paths
}

fn app(cx: &Cx, document: &Document, files: Rc<Vec<PathBuf>>) -> Element {
    let theme = cx.theme();
    let dirty = document.dirty();
    let text = document.text.get();
    let expanded = cx.state(BTreeSet::from([0]));
    let selected = cx.state(None);
    let open = document.clone();
    let open_files = files.clone();
    let nodes = files
        .iter()
        .enumerate()
        .map(|(id, path)| {
            FileTreeNode::file(
                id as u64 + 1,
                path.file_name().unwrap_or_default().to_string_lossy(),
            )
        })
        .collect();
    let tree = FileTree::new(cx, vec![FileTreeNode::folder(0, "Workspace", nodes)])
        .expanded(expanded)
        .selection(selected)
        .on_select(move |id| {
            if let Some(path) = id.checked_sub(1).and_then(|id| open_files.get(id as usize)) {
                open.request_open(path.clone());
            }
        });
    let save = document.clone();
    let revert = document.clone();
    let open = document.clone();
    let cancel = document.clone();
    let discard = document.clone();
    let dialog=Dialog::new(cx,document.confirm.clone(),ColumnView::new().spacing(16.0)
        .child(Text::new("Discard unsaved changes?").size(20.0).weight(600))
        .child(Text::new("The current document has changes. Save it first, or discard them to open another file."))
        .child(RowView::new().spacing(8.0)
            .child(Button::new(cx,"Keep editing").action(move ||{cancel.pending.set(None);cancel.confirm.set(false);}))
            .child(Button::new(cx,"Discard and open").action(move ||{if let Some(path)=discard.pending.get(){discard.open(&path);}discard.pending.set(None);discard.confirm.set(false);}))))
        .width(460.0);
    let name = PathBuf::from(document.path.get())
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    ColumnView::new()
        .fill()
        .surface(theme.colors.background)
        .child(
            RowView::new()
                .fill_width()
                .inset(16.0)
                .spacing(12.0)
                .align(Align::Center)
                .child(Icon::new(lucide::FILE_TEXT).color(theme.colors.accent))
                .child(
                    Text::new(format!("{name}{}", if dirty { " •" } else { "" }))
                        .size(20.0)
                        .weight(600)
                        .decorate(|view| view.grow(1.0)),
                )
                .child(
                    Button::new(cx, "Discard changes")
                        .style(ButtonStyle::Clean)
                        .disabled(!dirty)
                        .action(move || revert.discard()),
                )
                .child(
                    Button::new(cx, "Save")
                        .style(ButtonStyle::Filled)
                        .action(move || save.save()),
                ),
        )
        .child(
            RowView::new()
                .fill_width()
                .inset_horizontal(16.0)
                .spacing(8.0)
                .child(
                    TextField::new(cx, document.path.clone())
                        .placeholder("Path to a text file")
                        .decorate(|view| view.grow(1.0)),
                )
                .child(
                    Button::new(cx, "Open")
                        .style(ButtonStyle::Bordered)
                        .action(move || open.request_open(PathBuf::from(open.path.get()))),
                ),
        )
        .child(
            RowView::new()
                .grow(1.0)
                .fill_width()
                .height(0.0)
                .child(
                    ResizableView::new(
                        cx,
                        ScrollView::new(cx, tree).decorate(|view| view.fill().inset(8.0)),
                    )
                    .width(220.0)
                    .edges([ResizeEdge::Right]),
                )
                .child(
                    TextArea::new(cx, document.text.clone())
                        .style(TextFieldStyle::Clean)
                        .resizable(false)
                        .placeholder("Write something worth keeping…")
                        .decorate(|view| {
                            view.fill_height()
                                .min_width(0.0)
                                .inset(20.0)
                                .grow(1.0)
                                .clip()
                        }),
                ),
        )
        .child(
            RowView::new()
                .fill_width()
                .inset(12.0)
                .spacing(16.0)
                .border_top(theme.colors.border, 1.0)
                .child(
                    Text::new(document.message.get())
                        .size(12.0)
                        .decorate(|view| view.grow(1.0)),
                )
                .child(
                    Text::new(format!(
                        "{} words · {} lines",
                        text.split_whitespace().count(),
                        text.lines().count().max(1)
                    ))
                    .size(12.0),
                ),
        )
        .child(dialog)
        .into_element()
}

fn main() {
    let document = Document::new();
    let root = std::env::args_os().nth(1).map(PathBuf::from);
    if let Some(path) = &root {
        document.path.set(path.to_string_lossy().into_owned());
        if path.exists() {
            document.open(path);
        } else {
            document
                .message
                .set("New document. Press Save to create this file.".into());
        }
    }
    let directory = root
        .as_deref()
        .and_then(Path::parent)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let files = Rc::new(workspace(directory));
    let close = document.clone();
    run(move |cx| {
        cx.open_window(
            WindowConfig::default()
                .title("Text editor")
                .size(960.0, 680.0)
                .on_close_requested(move || {
                    if close.dirty() {
                        close.message.set(
                            "Unsaved changes. Save or use Discard changes before closing.".into(),
                        );
                        false
                    } else {
                        true
                    }
                }),
            move |cx| app(cx, &document, files.clone()),
        )
    });
}

#[cfg(test)]
#[path = "../tests/examples/editor.rs"]
mod tests;
