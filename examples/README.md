# Example applications

Run from the repository root with `cargo run --example <name>`. These applications
use the same public API as a downstream crate and follow system appearance.

## Tasks

```sh
cargo run --example tasks
```

A session task inbox. Add a title, toggle a task by clicking its checkbox or
pressing Space, filter active/completed tasks, remove a row or clear completed
tasks after confirmation. Rows retain their IDs as the list changes. Data resets
when the application exits; this example does not imply a sync or storage service.

## Editor

```sh
cargo run --example editor
cargo run --example editor -- ./notes.txt
```

An editor for UTF-8 files up to 1 MiB. The workspace lists up to 200 text/source
files in the starting directory. Select a file or enter its path and press Open.
Write a new document by choosing an unused path and pressing Save. Existing files
must be opened before they can be overwritten. Saving uses a temporary sibling
file and rename, preserving file permissions when replacing a document.

Unsaved navigation asks for confirmation. Failed reads/writes keep your current
text, and external changes prevent overwriting; choose a new path to save a copy.
Closing with unsaved changes is blocked until you save or discard them.

## Preferences

```sh
cargo run --example preferences
```

A reader preferences form. Change the name, typeface, text size, spacing and reading
guide. Apply updates all open preview windows; Revert restores the last applied
values and Defaults restores the form's defaults. An empty name prevents Apply.
Open multiple previews to see shared application state. Values last for the session.

## Focus

```sh
cargo run --example focus
```

Choose a 5, 25 or 50 minute session, enter an intention, then Start. Pause freezes
elapsed time, Resume continues and Reset clears the session. A notification marks
completion. Changing the preset intentionally creates a fresh timer. The timer
uses monotonic time; motion schedules UI frames rather than measuring elapsed
seconds. Keep the window open; there is no background daemon or OS notification.

## Testing

`cargo test --examples` exercises real retained trees and logical input, along with
document I/O and deterministic timer logic. `make check` includes these tests.
Headless tests complement native runtime checks; they do not prove native OS input.
