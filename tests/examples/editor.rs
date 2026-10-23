use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "lemon-editor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn open_save_and_external_conflicts_preserve_unsaved_content() {
    let directory = Directory::new();
    let path = directory.0.join("notes.txt");
    fs::write(&path, "Original").unwrap();
    let document = Document::new();
    document.open(&path);
    assert!(!document.dirty());
    document.text.set("Draft".into());
    assert!(document.dirty());
    document.save();
    assert_eq!(fs::read_to_string(&path).unwrap(), "Draft");
    assert!(!document.dirty());
    document.text.set("Keep my edits".into());
    fs::write(&path, "Changed elsewhere").unwrap();
    document.save();
    assert_eq!(fs::read_to_string(&path).unwrap(), "Changed elsewhere");
    assert_eq!(document.text.get(), "Keep my edits");
    assert!(document.message.get().contains("changed on disk"));
    let copy = directory.0.join("copy.txt");
    document.path.set(copy.to_string_lossy().into());
    document.save();
    assert_eq!(fs::read_to_string(copy).unwrap(), "Keep my edits");
}

#[test]
fn invalid_opens_and_dirty_navigation_do_not_discard_the_current_document() {
    let directory = Directory::new();
    let other = directory.0.join("other.txt");
    fs::write(&other, "Other").unwrap();
    let document = Document::new();
    document.text.set("Unsaved".into());
    document.request_open(other.clone());
    assert!(document.confirm.get());
    assert_eq!(document.pending.get(), Some(other));
    assert_eq!(document.text.get(), "Unsaved");
    let binary = directory.0.join("binary.txt");
    fs::write(&binary, [255, 254]).unwrap();
    document.open(&binary);
    assert_eq!(document.text.get(), "Unsaved");
    assert!(document.message.get().contains("UTF-8"));
    let large = directory.0.join("large.txt");
    fs::write(&large, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
    assert!(read_document(&large).is_err());
}

#[test]
fn editor_mounts_and_lays_out_at_useful_window_sizes() {
    let document = Document::new();
    let mut ui = lemon::ui::UiState::default();
    ui.mount(move |cx| app(cx, &document, Rc::new(Vec::new())))
        .unwrap();
    for (width, height) in [(960.0, 680.0), (700.0, 480.0)] {
        ui.update(lemon::Viewport { width, height }, 1.0, None, false)
            .unwrap();
    }
}

#[test]
fn discarding_restores_the_saved_text_and_path() {
    let directory = Directory::new();
    let path = directory.0.join("saved.txt");
    fs::write(&path, "Saved").unwrap();
    let document = Document::new();
    document.open(&path);
    document.text.set("Changed".into());
    document.path.set("another-name.txt".into());
    document.discard();
    assert_eq!(document.text.get(), "Saved");
    assert_eq!(PathBuf::from(document.path.get()), path);
    assert!(!document.dirty());
}
