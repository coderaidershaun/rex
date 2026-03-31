use crate::models::project::{Category, Complexity};
use crate::models::project_status::ONBOARDING_ITEMS;
use console::style;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, Write};

pub struct TabSelectResult {
    pub category: Category,
    pub selected_items: Vec<String>,
}

const CATEGORIES: [&str; 3] = ["Library", "Binary", "Refactor"];

/// Rows 0 and 1 are actions, rows 2+ map to ONBOARDING_ITEMS.
const ACTION_ROWS: usize = 2;

/// Guard that disables raw mode on drop, even during panics/early returns.
struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

fn is_required(item: &str, category: &Category) -> bool {
    matches!(item, "goal" | "scope" | "uat")
        || (item == "existing-code" && matches!(category, Category::Refactor))
}

fn compute_defaults(complexity: &Complexity, category: &Category) -> Vec<bool> {
    ONBOARDING_ITEMS
        .iter()
        .map(|&item| {
            if is_required(item, category) {
                return true;
            }
            match complexity {
                Complexity::High | Complexity::Medium => true,
                Complexity::Low => false,
            }
        })
        .collect()
}

fn category_from_tab(index: usize) -> Category {
    match index {
        0 => Category::Library,
        1 => Category::Binary,
        2 => Category::Refactor,
        _ => unreachable!(),
    }
}

fn apply_required(selected: &mut [bool], category: &Category) {
    for (i, &item) in ONBOARDING_ITEMS.iter().enumerate() {
        if is_required(item, category) {
            selected[i] = true;
        }
    }
}

fn is_action_disabled(row: usize, selected: &[bool], category: &Category) -> bool {
    if row == 0 {
        // select-all disabled when all selected
        selected.iter().all(|&s| s)
    } else if row == 1 {
        // clear-all disabled when only required remain
        ONBOARDING_ITEMS
            .iter()
            .zip(selected.iter())
            .all(|(&item, &sel)| sel == is_required(item, category))
    } else {
        false
    }
}

fn is_row_disabled(row: usize, selected: &[bool], category: &Category) -> bool {
    if row < ACTION_ROWS {
        is_action_disabled(row, selected, category)
    } else {
        let item_idx = row - ACTION_ROWS;
        is_required(ONBOARDING_ITEMS[item_idx], category)
    }
}

fn skip_disabled(cursor: usize, direction: isize, total: usize, selected: &[bool], category: &Category) -> usize {
    let mut pos = cursor;
    loop {
        pos = ((pos as isize + direction).rem_euclid(total as isize)) as usize;
        if !is_row_disabled(pos, selected, category) || pos == cursor {
            break pos;
        }
    }
}

fn render(
    stdout: &mut io::Stdout,
    tab_index: usize,
    cursor_row: usize,
    selected: &[bool],
    category: &Category,
    complexity: &Complexity,
    total_lines: &mut usize,
) -> io::Result<()> {
    if *total_lines > 0 {
        execute!(stdout, cursor::MoveUp(*total_lines as u16))?;
    }
    execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;

    let mut lines = 0;

    // Tab hint
    write!(stdout, "  {}\r\n", style("Use tab to change category").dim())?;
    lines += 1;
    write!(stdout, "\r\n")?;
    lines += 1;

    // Tab bar
    let mut tab_line = String::from("  ");
    for (i, name) in CATEGORIES.iter().enumerate() {
        if i == tab_index {
            tab_line.push_str(&format!("{}", style(format!("[ {name} ]")).bold().cyan()));
        } else {
            tab_line.push_str(&format!("{}", style(format!("  {name}  ")).dim()));
        }
        if i < CATEGORIES.len() - 1 {
            tab_line.push_str("  ");
        }
    }
    write!(stdout, "{tab_line}\r\n")?;
    lines += 1;

    // Guidance line
    write!(stdout, "\r\n")?;
    lines += 1;
    let recommendation = match complexity {
        Complexity::High | Complexity::Medium => "(recommend selecting all for medium and high complexity)",
        Complexity::Low => {
            if selected.iter().all(|&s| s) {
                "(recommend only a few of these for low complexity)"
            } else {
                ""
            }
        }
    };
    if recommendation.is_empty() {
        write!(stdout, "  Confirm desired project onboarding steps.\r\n")?;
    } else {
        write!(
            stdout,
            "  Confirm desired project onboarding steps. {}\r\n",
            style(recommendation).dim()
        )?;
    }
    lines += 1;
    write!(
        stdout,
        "  {}\r\n",
        style("spacebar selects \u{00b7} enter accepts \u{00b7} esc resets").dim()
    )?;
    lines += 1;
    write!(stdout, "\r\n")?;
    lines += 1;
    write!(stdout, "\r\n")?;
    lines += 1;

    // Action rows: select-all and clear-all
    let all_selected = selected.iter().all(|&s| s);
    let all_cleared = ONBOARDING_ITEMS
        .iter()
        .zip(selected.iter())
        .all(|(&item, &sel)| sel == is_required(item, category));

    let actions = ["select-all", "clear-all"];
    let disabled = [all_selected, all_cleared];
    for (i, action) in actions.iter().enumerate() {
        let is_cursor = i == cursor_row;
        if disabled[i] {
            let marker = if is_cursor {
                style("\u{203a} ").bold()
            } else {
                style("  ").clone()
            };
            let label = format!("{}", style(*action).dim());
            write!(stdout, "  {marker}{label}\r\n")?;
        } else {
            let marker = if is_cursor {
                style("\u{203a} ").bold()
            } else {
                style("  ").clone()
            };
            let label = if is_cursor {
                format!("{}", style(*action).bold().blue())
            } else {
                format!("{}", style(*action).blue())
            };
            write!(stdout, "  {marker}{label}\r\n")?;
        }
        lines += 1;
    }

    write!(stdout, "\r\n")?;
    lines += 1;

    // Checkboxes
    for (i, &item) in ONBOARDING_ITEMS.iter().enumerate() {
        let row = i + ACTION_ROWS;
        let required = is_required(item, category);
        let is_selected = selected[i];
        let is_cursor = row == cursor_row;

        let marker = if is_cursor {
            style("\u{203a} ").bold()
        } else {
            style("  ").clone()
        };

        let checkbox = if required {
            format!("{}", style("[\u{25a0}]").cyan())
        } else if is_selected {
            format!("{}", style("[x]").green())
        } else {
            format!("{}", style("[ ]").clone())
        };

        let label = if required {
            format!(
                "{} {}",
                if is_cursor {
                    style(item).bold().to_string()
                } else {
                    item.to_string()
                },
                style("(required)").dim()
            )
        } else if is_cursor {
            format!("{}", style(item).bold())
        } else {
            item.to_string()
        };

        write!(stdout, "  {marker}{checkbox} {label}\r\n")?;
        lines += 1;
    }

    stdout.flush()?;
    *total_lines = lines;
    Ok(())
}

pub fn tab_select(
    complexity: &Complexity,
) -> Result<TabSelectResult, Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    let total_rows = ACTION_ROWS + ONBOARDING_ITEMS.len();

    // Header (printed before raw mode)
    println!();
    println!(
        "  {}",
        style("Category & Onboarding").bold().underlined()
    );
    println!();

    let mut tab_index: usize = 0;
    let mut category = category_from_tab(tab_index);
    let mut selected = compute_defaults(complexity, &category);
    // Start cursor on first non-required checkbox
    let mut cursor_row: usize = skip_disabled(ACTION_ROWS.saturating_sub(1), 1, total_rows, &selected, &category);
    let mut total_lines: usize = 0;

    terminal::enable_raw_mode()?;
    let _guard = RawModeGuard;

    render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;

    let result = loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match code {
                    KeyCode::Tab | KeyCode::Right => {
                        tab_index = (tab_index + 1) % CATEGORIES.len();
                        category = category_from_tab(tab_index);
                        selected = compute_defaults(complexity, &category);
                        render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;
                    }
                    KeyCode::BackTab | KeyCode::Left => {
                        tab_index = if tab_index == 0 {
                            CATEGORIES.len() - 1
                        } else {
                            tab_index - 1
                        };
                        category = category_from_tab(tab_index);
                        selected = compute_defaults(complexity, &category);
                        render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;
                    }
                    KeyCode::Up => {
                        cursor_row = skip_disabled(cursor_row, -1, total_rows, &selected, &category);
                        render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;
                    }
                    KeyCode::Down => {
                        cursor_row = skip_disabled(cursor_row, 1, total_rows, &selected, &category);
                        render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;
                    }
                    KeyCode::Char(' ') => {
                        if cursor_row == 0 {
                            // select-all
                            if !is_action_disabled(0, &selected, &category) {
                                selected.iter_mut().for_each(|s| *s = true);
                                cursor_row = skip_disabled(cursor_row, 1, total_rows, &selected, &category);
                                render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;
                            }
                        } else if cursor_row == 1 {
                            // clear-all
                            if !is_action_disabled(1, &selected, &category) {
                                selected.iter_mut().for_each(|s| *s = false);
                                apply_required(&mut selected, &category);
                                cursor_row = skip_disabled(cursor_row, 1, total_rows, &selected, &category);
                                render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;
                            }
                        } else {
                            let item_idx = cursor_row - ACTION_ROWS;
                            if !is_required(ONBOARDING_ITEMS[item_idx], &category) {
                                selected[item_idx] = !selected[item_idx];
                                render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let selected_items: Vec<String> = ONBOARDING_ITEMS
                            .iter()
                            .zip(selected.iter())
                            .filter(|(_, sel)| **sel)
                            .map(|(&item, _)| item.to_string())
                            .collect();
                        break Ok(TabSelectResult {
                            category: category.clone(),
                            selected_items,
                        });
                    }
                    KeyCode::Esc => {
                        selected = compute_defaults(complexity, &category);
                        render(&mut stdout, tab_index, cursor_row, &selected, &category, complexity, &mut total_lines)?;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        break Err("Cancelled".into());
                    }
                    _ => {}
                }
            }
        }
    };

    drop(_guard);

    // Clear the widget and print a compact summary
    if total_lines > 0 {
        execute!(stdout, cursor::MoveUp(total_lines as u16))?;
        execute!(stdout, terminal::Clear(ClearType::FromCursorDown))?;
    }

    if let Ok(ref res) = result {
        let selected_count = res.selected_items.len();
        println!(
            "  {} Category: {}  |  Onboarding: {}/{} items selected",
            style("\u{2713}").green().bold(),
            style(&res.category).cyan(),
            selected_count,
            ONBOARDING_ITEMS.len()
        );
    }

    result
}
