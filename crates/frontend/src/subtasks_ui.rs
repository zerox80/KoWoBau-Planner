use crate::*;

pub(crate) fn subtasks_panel(
    task_id: String,
    subtasks: Vec<SubtaskDto>,
    pct: usize,
    can_edit: bool,
    lang: ReadSignal<Lang>,
    set_data: WriteSignal<Option<BootstrapDto>>,
    set_error: WriteSignal<Option<String>>,
) -> View {
    let (new_title, set_new_title) = create_signal(String::new());
    let (busy, set_busy) = create_signal(false);
    let (local_error, set_local_error) = create_signal::<Option<String>>(None);
    let task_id_for_create = task_id.clone();

    let submit_new = move || {
        if !can_edit || busy.get_untracked() {
            return;
        }
        let title = new_title.get_untracked();
        if !require_title(
            &title,
            "Bitte gib zuerst einen Unteraufgaben-Titel ein.",
            "Add a subtask title first.",
            lang.get_untracked(),
            set_local_error,
        ) {
            return;
        }
        set_busy.set(true);
        let task_id = task_id_for_create.clone();
        spawn_local(async move {
            let payload = CreateSubtaskRequest {
                title: title.trim().to_string(),
            };
            match api_post::<_, TaskDto>(&format!("/api/tasks/{task_id}/subtasks"), &payload).await
            {
                Ok(task) => {
                    replace_task(set_data, task);
                    set_new_title.set(String::new());
                    set_error.set(None);
                    set_local_error.set(None);
                }
                Err(err) => report_api_error(&err, set_local_error, set_error),
            }
            set_busy.set(false);
        });
    };
    let submit_new_for_key = submit_new.clone();

    view! {
        <section>
            <h3>{move || lang.get().tr("Unteraufgaben", "Subtasks")}</h3>
            <div class="progress-line"><i style=format!("width:{pct}%")></i></div>
            <div class="subtask-list">
                {if subtasks.is_empty() {
                    view! {
                        <p class="subtask-empty">
                            {move || lang.get().tr("Keine Unteraufgaben.", "No subtasks.")}
                        </p>
                    }.into_view()
                } else {
                    subtasks.into_iter().map(|sub| {
                        subtask_row(task_id.clone(), sub, can_edit, lang, set_data, set_error)
                    }).collect_view().into_view()
                }}
            </div>
            {if can_edit {
                view! {
                    <div class="subtask-add">
                        <input
                            placeholder=move || lang.get().tr("Neue Unteraufgabe", "New subtask")
                            prop:value=new_title
                            disabled=move || busy.get()
                            on:input=move |ev| {
                                set_new_title.set(event_target_value(&ev));
                                set_local_error.set(None);
                            }
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" {
                                    ev.prevent_default();
                                    submit_new_for_key();
                                }
                            }
                        />
                        <button class="btn primary" disabled=move || busy.get() on:click=move |_| submit_new()>
                            {move || lang.get().tr("Hinzufuegen", "Add")}
                        </button>
                    </div>
                }.into_view()
            } else {
                empty_view()
            }}
            {move || local_error.get().map(|err| view! {
                <div class="modal-error inline subtask-error">{err}</div>
            })}
        </section>
    }.into_view()
}

fn subtask_row(
    task_id: String,
    sub: SubtaskDto,
    can_edit: bool,
    lang: ReadSignal<Lang>,
    set_data: WriteSignal<Option<BootstrapDto>>,
    set_error: WriteSignal<Option<String>>,
) -> View {
    let label = title_for(sub.title, sub.title_en, lang.get());
    let (editing, set_editing) = create_signal(false);
    let (title_edit, set_title_edit) = create_signal(label.clone());
    let (busy, set_busy) = create_signal(false);
    let (local_error, set_local_error) = create_signal::<Option<String>>(None);
    let subtask_id = sub.id;
    let done = sub.done;

    let task_id_for_toggle = task_id.clone();
    let subtask_id_for_toggle = subtask_id.clone();
    let toggle = move |ev| {
        let checked = event_target::<web_sys::HtmlInputElement>(&ev).checked();
        toggle_subtask(
            task_id_for_toggle.clone(),
            subtask_id_for_toggle.clone(),
            checked,
            set_data,
            set_error,
        );
    };

    let task_id_for_save = task_id.clone();
    let subtask_id_for_save = subtask_id.clone();
    let save_title = move || {
        if busy.get_untracked() {
            return;
        }
        let title = title_edit.get_untracked();
        if !require_title(
            &title,
            "Bitte gib zuerst einen Unteraufgaben-Titel ein.",
            "Add a subtask title first.",
            lang.get_untracked(),
            set_local_error,
        ) {
            return;
        }
        set_busy.set(true);
        let task_id = task_id_for_save.clone();
        let subtask_id = subtask_id_for_save.clone();
        spawn_local(async move {
            let payload = UpdateSubtaskRequest {
                title: Some(title.trim().to_string()),
                done: None,
            };
            match api_patch::<_, TaskDto>(
                &format!("/api/tasks/{task_id}/subtasks/{subtask_id}"),
                &payload,
            )
            .await
            {
                Ok(task) => {
                    replace_task(set_data, task);
                    set_editing.set(false);
                    set_error.set(None);
                    set_local_error.set(None);
                }
                Err(err) => report_api_error(&err, set_local_error, set_error),
            }
            set_busy.set(false);
        });
    };
    let task_id_for_delete = task_id;
    let subtask_id_for_delete = subtask_id;
    let label_for_delete = label.clone();
    let delete_subtask = move |_| {
        if busy.get_untracked() || !confirm_delete(&label_for_delete, lang.get_untracked()) {
            return;
        }
        set_busy.set(true);
        let task_id = task_id_for_delete.clone();
        let subtask_id = subtask_id_for_delete.clone();
        spawn_local(async move {
            match api_delete::<TaskDto>(&format!("/api/tasks/{task_id}/subtasks/{subtask_id}"))
                .await
            {
                Ok(task) => {
                    replace_task(set_data, task);
                    set_error.set(None);
                    set_local_error.set(None);
                }
                Err(err) => report_api_error(&err, set_local_error, set_error),
            }
            set_busy.set(false);
        });
    };

    let label_for_edit = label.clone();
    view! {
        <div class="subtask-item">
            {move || if editing.get() {
                let save_title_for_key = save_title.clone();
                let save_title_for_button = save_title.clone();
                let label_for_escape = label_for_edit.clone();
                let label_for_cancel = label_for_edit.clone();
                view! {
                    <div class="subtask subtask-edit-row">
                        <input type="checkbox" checked=done disabled/>
                        <input
                            prop:value=title_edit
                            disabled=move || busy.get()
                            on:input=move |ev| {
                                set_title_edit.set(event_target_value(&ev));
                                set_local_error.set(None);
                            }
                            on:keydown=move |ev| {
                                match ev.key().as_str() {
                                    "Enter" => {
                                        ev.prevent_default();
                                        save_title_for_key();
                                    }
                                    "Escape" => {
                                        set_title_edit.set(label_for_escape.clone());
                                        set_local_error.set(None);
                                        set_editing.set(false);
                                    }
                                    _ => {}
                                }
                            }
                        />
                        <span class="subtask-actions">
                            <button class="link-button" disabled=move || busy.get() on:click=move |_| save_title_for_button()>"OK"</button>
                            <button class="link-button" disabled=move || busy.get() on:click=move |_| {
                                set_title_edit.set(label_for_cancel.clone());
                                set_local_error.set(None);
                                set_editing.set(false);
                            }>{move || lang.get().tr("Abbrechen", "Cancel")}</button>
                        </span>
                    </div>
                }.into_view()
            } else {
                let label_for_checkbox = label.clone();
                let label_for_button = label.clone();
                let toggle_for_input = toggle.clone();
                let delete_for_button = delete_subtask.clone();
                view! {
                    <div class="subtask subtask-row">
                        {if can_edit {
                            view! {
                                <input type="checkbox" checked=done aria-label=label_for_checkbox on:change=toggle_for_input/>
                            }.into_view()
                        } else {
                            view! { <input type="checkbox" checked=done disabled aria-label=label_for_checkbox/> }.into_view()
                        }}
                        <span class="subtask-title">{label.clone()}</span>
                        {if can_edit {
                            view! {
                                <span class="subtask-actions">
                                    <button class="link-button" on:click=move |_| {
                                        set_title_edit.set(label_for_button.clone());
                                        set_local_error.set(None);
                                        set_editing.set(true);
                                    }>{move || lang.get().tr("Umbenennen", "Rename")}</button>
                                    <button class="danger-link" disabled=move || busy.get() on:click=delete_for_button>
                                        {move || lang.get().tr("Loeschen", "Delete")}
                                    </button>
                                </span>
                            }.into_view()
                        } else {
                            empty_view()
                        }}
                    </div>
                }.into_view()
            }}
            {move || local_error.get().map(|err| view! {
                <div class="modal-error inline subtask-error">{err}</div>
            })}
        </div>
    }.into_view()
}
