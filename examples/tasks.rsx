app Tasks;

extern struct Task = crate::backend::Task {
    id: i64,
    title: str,
    done: bool,
}

extern struct AppError = crate::backend::AppError {
    message: str,
}

extern fn list_tasks() -> [Task] ! AppError = crate::backend::list;
extern fn create_task(title: str) -> [Task] ! AppError = crate::backend::create;
extern fn set_task_done(id: i64, done: bool) -> [Task] ! AppError = crate::backend::set_done;

theme {
    background: "#0f172a",
    surface: "#111827",
    foreground: "#f8fafc",
    muted: "#94a3b8",
    primary: "#7c3aed",
    danger: "#dc2626",
    border: "#334155",
}

state tasks: [Task] = [];
state draft: str = "";
state loading: bool = false;
state error: str = "";

on mount {
    loading = true;
    run list_tasks() -> loaded(_) | failed(_);
}

on edit(value: str) {
    draft = value;
}

on submit {
    if loading || empty(trim(draft)) {
        return;
    }

    loading = true;
    error = "";
    run create_task(trim(draft)) -> created(_) | failed(_);
}

on toggle(id: i64, checked: bool) {
    if loading {
        return;
    }

    loading = true;
    error = "";
    run set_task_done(id, checked) -> updated(_) | failed(_);
}

on retry {
    loading = true;
    error = "";
    run list_tasks() -> loaded(_) | failed(_);
}

on loaded(next: [Task]) {
    tasks = next;
    loading = false;
}

on created(next: [Task]) {
    tasks = next;
    draft = "";
    loading = false;
}

on updated(next: [Task]) {
    tasks = next;
    loading = false;
}

on failed(cause: AppError) {
    loading = false;
    error = cause.message;
}

view {
    col class="w-full h-full p-6 bg-background" {
        col class="w-full max-w-2xl self-center gap-6" {
            row class="w-full items-center justify-between gap-4" {
                text "Tasks" class="text-2xl font-bold text-foreground";
                text len(tasks) class="text-sm text-muted";
            }

            row class="w-full items-center gap-3" {
                input
                    id="new-task"
                    label="New task"
                    value=draft
                    placeholder="What needs doing?"
                    disabled=loading
                    class="w-full px-4 py-3 bg-surface text-foreground border border-border rounded-lg focus:border-primary disabled:opacity-50"
                    -> edit(_);

                button "Add"
                    disabled=(loading || empty(trim(draft)))
                    class="px-4 py-3 bg-primary text-white rounded-lg hover:bg-primary/90 pressed:bg-primary/75 disabled:opacity-50"
                    -> submit;
            }

            if error != "" {
                row class="w-full items-center justify-between gap-4 p-4 bg-danger/90 rounded-lg" {
                    text error class="text-sm text-white";
                    button "Retry"
                        disabled=loading
                        class="px-4 py-2 bg-white text-danger rounded-md disabled:opacity-50"
                        -> retry;
                }
            }

            if loading {
                text "Working..." class="text-sm text-muted";
            }

            if empty(tasks) && !loading {
                col class="w-full p-6 items-center bg-surface border border-border rounded-lg" {
                    text "No tasks yet." class="text-sm text-muted";
                }
            }

            scroll class="w-full h-full" {
                col class="w-full gap-2" {
                    for task in tasks {
                        row class="w-full items-center p-4 bg-surface border border-border rounded-lg" {
                            checkbox
                                label=task.title
                                checked=task.done
                                disabled=loading
                                class="w-full text-foreground disabled:opacity-50"
                                -> toggle(task.id, _);
                        }
                    }
                }
            }
        }
    }
}
