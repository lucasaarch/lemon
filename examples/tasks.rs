//! A task inbox: add, complete, filter and remove tasks with stable component identity.
//! Data belongs to the current session. No files or remote services are used.
use lemon::prelude::*;

#[derive(Clone)]
struct Task {
    id: u64,
    title: String,
    done: State<bool>,
}

#[derive(Clone)]
struct TaskProps {
    task: Task,
    tasks: State<Vec<Task>>,
}

// IDs are never reused; each ID keeps the same state cells for this session.
impl PartialEq for TaskProps {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id && self.task.title == other.task.title
    }
}

fn task_row(cx: &Cx, props: &TaskProps) -> Element {
    let theme = cx.theme();
    let task = &props.task;
    let tasks = props.tasks.clone();
    let id = task.id;
    RowView::new()
        .fill_width()
        .spacing(12.0)
        .inset(12.0)
        .align(Align::Center)
        .distribute(Distribute::Between)
        .surface(theme.colors.surface)
        .rounding(theme.radius.md)
        .child(
            Checkbox::new(cx, task.done.clone())
                .label(&task.title)
                .decorate(|view| view.opacity(if task.done.get() { 0.6 } else { 1.0 })),
        )
        .child(
            Button::new(cx, "Remove")
                .style(ButtonStyle::Clean)
                .action(move || tasks.update(|tasks| tasks.retain(|task| task.id != id))),
        )
        .into_element()
}

fn app(cx: &Cx) -> Element {
    let theme = cx.theme();
    let tasks = cx.state(vec![
        Task {
            id: 1,
            title: "Plan the next release".into(),
            done: State::new(false),
        },
        Task {
            id: 2,
            title: "Review the documentation".into(),
            done: State::new(false),
        },
        Task {
            id: 3,
            title: "Share a working prototype".into(),
            done: State::new(true),
        },
    ]);
    let next_id = cx.state(4_u64);
    let draft = cx.state(String::new());
    let filter = cx.state(Some(0));
    let confirm = cx.state(false);
    let all = tasks.get();
    let completed = all.iter().filter(|task| task.done.get()).count();
    let selected = filter.get().unwrap_or(0);
    let mut rows = ColumnView::new().fill_width().spacing(8.0);
    let mut count = 0;
    for task in all.iter().filter(|task| match selected {
        1 => !task.done.get(),
        2 => task.done.get(),
        _ => true,
    }) {
        count += 1;
        rows = rows.child(
            Component::new_with_props(
                task_row,
                TaskProps {
                    task: task.clone(),
                    tasks: tasks.clone(),
                },
            )
            .key(task.id),
        );
    }
    if count == 0 {
        rows = rows.child(
            Text::new("Nothing here. Add a task or change the filter.")
                .color(theme.colors.foreground_secondary),
        );
    }
    let add_tasks = tasks.clone();
    let add_draft = draft.clone();
    let add_id = next_id.clone();
    let clear = confirm.clone();
    let cancel = confirm.clone();
    let remove = confirm.clone();
    let clear_tasks = tasks.clone();
    let dialog = Dialog::new(
        cx,
        confirm,
        ColumnView::new()
            .spacing(16.0)
            .child(Text::new("Clear completed tasks?").size(20.0))
            .child(Text::new(format!(
                "Remove {completed} completed tasks from this session?"
            )))
            .child(
                RowView::new()
                    .spacing(8.0)
                    .child(Button::new(cx, "Cancel").action(move || cancel.set(false)))
                    .child(
                        Button::new(cx, "Clear completed")
                            .style(ButtonStyle::Filled)
                            .action(move || {
                                clear_tasks.update(|tasks| tasks.retain(|task| !task.done.get()));
                                remove.set(false);
                            }),
                    ),
            ),
    );
    ColumnView::new()
        .fill()
        .inset(24.0)
        .spacing(16.0)
        .surface(theme.colors.background)
        .child(Text::new("Today's inbox").size(28.0).weight(650))
        .child(
            Text::new(format!(
                "{} remaining · {completed} completed",
                all.len() - completed
            ))
            .color(theme.colors.foreground_secondary),
        )
        .child(
            RowView::new()
                .fill_width()
                .spacing(8.0)
                .child(
                    TextField::new(cx, draft.clone())
                        .placeholder("What needs doing?")
                        .decorate(|view| view.grow(1.0)),
                )
                .child(
                    Button::new(cx, "Add task")
                        .style(ButtonStyle::Filled)
                        .disabled(draft.get().trim().is_empty())
                        .action(move || {
                            let title = add_draft.get().trim().to_owned();
                            if title.is_empty() {
                                return;
                            }
                            let id = add_id.get();
                            add_id.set(id + 1);
                            add_tasks.update(|tasks| {
                                tasks.push(Task {
                                    id,
                                    title,
                                    done: State::new(false),
                                })
                            });
                            add_draft.set(String::new());
                        }),
                ),
        )
        .child(
            RowView::new()
                .spacing(12.0)
                .align(Align::Center)
                .child(
                    Select::new(
                        cx,
                        filter,
                        vec![
                            (0, "All tasks".into()),
                            (1, "Active".into()),
                            (2, "Completed".into()),
                        ],
                    )
                    .width(180.0),
                )
                .child(
                    Button::new(cx, "Clear finished")
                        .style(ButtonStyle::Bordered)
                        .disabled(completed == 0)
                        .action(move || clear.set(true)),
                ),
        )
        .child(ScrollView::new(cx, rows).decorate(|view| view.fill_width().grow(1.0).height(0.0)))
        .child(
            Text::new("Session inbox · Tab to navigate, Space to toggle a task")
                .size(12.0)
                .color(theme.colors.foreground_secondary),
        )
        .child(dialog)
        .into_element()
}

fn main() {
    run(|cx| {
        cx.open_window(
            WindowConfig::default().title("Inbox").size(640.0, 560.0),
            app,
        )
    });
}

#[cfg(test)]
#[path = "../tests/examples/tasks.rs"]
mod tests;
