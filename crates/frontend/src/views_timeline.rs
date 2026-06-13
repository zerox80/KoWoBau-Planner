use crate::*;

pub(crate) fn calendar_view(
    boot: BootstrapDto,
    lang: ReadSignal<Lang>,
    set_open_task: WriteSignal<Option<String>>,
) -> View {
    let all_tasks = boot.tasks;
    let (year, month, today_day) = now_date();
    view! {
        <div class="calendar-grid">
            {(1..=days_in_month(year, month)).map(|day| {
                let iso = format!("{year:04}-{month:02}-{day:02}");
                let tasks = all_tasks.iter().filter(|t| t.due_date.as_deref() == Some(iso.as_str())).cloned();
                view! {
                    <div class="day-cell" class:today=move || day == today_day>
                        <strong>{day}</strong>
                        {tasks.take(3).map(|task| {
                            let task_id = task.id.clone();
                            let label = task_title(&task, lang.get());
                            view! { <button class="cal-chip" on:click=move |_| set_open_task.set(Some(task_id.clone()))>{label}</button> }
                        }).collect_view()}
                    </div>
                }
            }).collect_view()}
        </div>
    }.into_view()
}
pub(crate) fn gantt_view(
    boot: BootstrapDto,
    lang: ReadSignal<Lang>,
    set_open_task: WriteSignal<Option<String>>,
) -> View {
    let statuses = boot.statuses.clone();
    let tasks: Vec<ScheduledTask> = boot
        .tasks
        .into_iter()
        .filter_map(|task| {
            let (start, due) =
                scheduled_task_days(task.start_date.as_deref(), task.due_date.as_deref())?;
            Some(ScheduledTask { task, start, due })
        })
        .collect();
    let milestones: Vec<ScheduledMilestone> = boot
        .milestones
        .into_iter()
        .filter_map(|milestone| {
            let day = iso_day_number(&milestone.due_date)?;
            Some(ScheduledMilestone { milestone, day })
        })
        .collect();

    if tasks.is_empty() && milestones.is_empty() {
        return view! {
            <div class="gantt-panel">
                <div class="empty-state compact">
                    <strong>{move || if lang.get() == Lang::De { "Keine Termine geplant" } else { "No scheduled items" }}</strong>
                    <span>{move || if lang.get() == Lang::De { "Aufgaben mit Start- oder Fälligkeitsdatum erscheinen hier." } else { "Tasks with a start or due date will appear here." }}</span>
                </div>
            </div>
        }.into_view();
    }

    let (min_day, max_day) = timeline_bounds(
        tasks.iter().map(|task| (task.start, task.due)),
        milestones.iter().map(|milestone| milestone.day),
    )
    .unwrap_or_else(|| {
        let today = iso_day_number(&today_iso()).unwrap_or(0);
        (today, today)
    });
    let range = (max_day - min_day + 1).max(1) as usize;
    let chart_width = range * GANTT_DAY_WIDTH;
    let row_width = GANTT_LABEL_WIDTH + chart_width;
    view! {
        <div class="gantt-panel">
            <div class="gantt-scroll" style=format!("width:{row_width}px")>
                <div class="gantt-scale" style=format!("grid-template-columns:{GANTT_LABEL_WIDTH}px repeat({range}, {GANTT_DAY_WIDTH}px)")>
                    <span></span>
                    {(0..range).map(|i| {
                        let (_, _, d) = civil_from_days(min_day + i as i64);
                        view! { <span>{d}</span> }
                    }).collect_view()}
                </div>
                <div class="gantt-milestones" style=format!("grid-template-columns:{GANTT_LABEL_WIDTH}px {chart_width}px")>
                    <span>{move || if lang.get() == Lang::De { "Meilensteine" } else { "Milestones" }}</span>
                    <div class="gantt-track">
                        {milestones.into_iter().map(|scheduled| {
                            let left = ((scheduled.day - min_day).max(0) as usize * GANTT_DAY_WIDTH) + (GANTT_DAY_WIDTH / 2);
                            let title = title_for(scheduled.milestone.title, scheduled.milestone.title_en, lang.get());
                            let date = fmt_date(&scheduled.milestone.due_date, lang.get());
                            view! {
                                <span class="gantt-milestone" style=format!("left:{left}px") title=format!("{title} - {date}")>
                                    <i></i>
                                    <b>{title}</b>
                                </span>
                            }
                        }).collect_view()}
                    </div>
                </div>
                {tasks.into_iter().map(|scheduled| {
                    let start = scheduled.start;
                    let due = scheduled.due;
                    let left = (start - min_day).max(0) as usize * GANTT_DAY_WIDTH;
                    let width = ((due - start + 1).max(1) as usize * GANTT_DAY_WIDTH).max(GANTT_DAY_WIDTH);
                    let task = scheduled.task;
                    let task_id = task.id.clone();
                    let key = task.key.clone();
                    let title = task_title(&task, lang.get());
                    let color = status_color(&statuses, &task.status_id);
                    let dep_count = task.dependency_ids.len();
                    view! {
                        <button class="gantt-row" style=format!("width:{row_width}px;grid-template-columns:{GANTT_LABEL_WIDTH}px {chart_width}px") on:click=move |_| set_open_task.set(Some(task_id.clone()))>
                            <span class="gantt-key">
                                <b>{key}</b>
                                {if dep_count > 0 {
                                    view! { <small title=move || if lang.get() == Lang::De { "Hat Abhängigkeiten" } else { "Has dependencies" }>{dep_count}</small> }.into_view()
                                } else {
                                    ().into_view()
                                }}
                            </span>
                            <span class="gantt-track">
                                <i class="gantt-bar" style=format!("left:{left}px;width:{width}px;background:{color}") title=title.clone()>{title}</i>
                            </span>
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }.into_view()
}

const GANTT_DAY_WIDTH: usize = 44;
const GANTT_LABEL_WIDTH: usize = 88;

#[derive(Debug, Clone)]
struct ScheduledTask {
    task: TaskDto,
    start: i64,
    due: i64,
}

#[derive(Debug, Clone)]
struct ScheduledMilestone {
    milestone: MilestoneDto,
    day: i64,
}

pub(crate) fn scheduled_task_days(start: Option<&str>, due: Option<&str>) -> Option<(i64, i64)> {
    let start = start.and_then(iso_day_number);
    let due = due.and_then(iso_day_number);
    match (start, due) {
        (Some(start), Some(due)) => Some((start.min(due), start.max(due))),
        (Some(day), None) | (None, Some(day)) => Some((day, day)),
        (None, None) => None,
    }
}

pub(crate) fn timeline_bounds(
    task_ranges: impl IntoIterator<Item = (i64, i64)>,
    milestone_days: impl IntoIterator<Item = i64>,
) -> Option<(i64, i64)> {
    let mut bounds: Option<(i64, i64)> = None;
    for (start, due) in task_ranges {
        bounds = Some(match bounds {
            Some((min, max)) => (min.min(start), max.max(due)),
            None => (start, due),
        });
    }
    for day in milestone_days {
        bounds = Some(match bounds {
            Some((min, max)) => (min.min(day), max.max(day)),
            None => (day, day),
        });
    }
    bounds
}
pub(crate) fn roadmap_view(
    boot: BootstrapDto,
    lang: ReadSignal<Lang>,
    set_open_task: WriteSignal<Option<String>>,
) -> View {
    let phases = [
        (
            "planung",
            if lang.get() == Lang::De {
                "Planung"
            } else {
                "Planning"
            },
        ),
        (
            "vergabe",
            if lang.get() == Lang::De {
                "Vergabe"
            } else {
                "Tendering"
            },
        ),
        (
            "ausfuehrung",
            if lang.get() == Lang::De {
                "Ausführung"
            } else {
                "Execution"
            },
        ),
        (
            "abnahme",
            if lang.get() == Lang::De {
                "Abnahme"
            } else {
                "Handover"
            },
        ),
    ];
    let all_tasks = boot.tasks;
    view! {
        <div class="roadmap-grid">
            {phases.into_iter().map(|(phase, label)| {
                let tasks = all_tasks.iter().filter(|t| t.phase == phase).cloned().collect::<Vec<_>>();
                let done = tasks.iter().filter(|t| t.status_is_done).count();
                let pct = if tasks.is_empty() { 0 } else { done * 100 / tasks.len() };
                view! {
                    <section class="road-card">
                        <header><h3>{label}</h3><small>{pct}"%"</small></header>
                        <span class="bar"><i style=format!("width:{pct}%")></i></span>
                        {tasks.into_iter().map(|task| {
                            let task_id = task.id.clone();
                            let title = task_title(&task, lang.get());
                            view! { <button on:click=move |_| set_open_task.set(Some(task_id.clone()))>{title}</button> }
                        }).collect_view()}
                    </section>
                }
            }).collect_view()}
        </div>
    }.into_view()
}
pub(crate) fn team_view(boot: BootstrapDto, lang: ReadSignal<Lang>) -> View {
    view! {
        <div class="team-grid">
            {boot.members.iter().map(|m| view! {
                <article class="member-card">
                    <span class="avatar large">{m.initials.clone()}</span>
                    <div>
                        <h3>{m.name.clone()}</h3>
                        <p>{role_label(&m.role, lang.get())}</p>
                        <small>
                            <strong>{m.open_tasks}</strong>
                            {move || if lang.get() == Lang::De { " offen" } else { " open" }}
                            " / "
                            <strong>{m.done_tasks}</strong>
                            {move || if lang.get() == Lang::De { " fertig" } else { " done" }}
                        </small>
                    </div>
                </article>
            }).collect_view()}
        </div>
    }
    .into_view()
}
