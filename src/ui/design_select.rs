use crate::errors::{RexError, RexResult};
use crate::models::project::{Category, Complexity};
use crate::models::project_status::DESIGN_ITEMS;
use console::style;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, Write};

pub struct DesignSelectResult {
    pub selected_items: Vec<String>,
}

/// Rows 0 and 1 are actions, rows 2+ map to DESIGN_ITEMS.
const ACTION_ROWS: usize = 2;

/// Guard that disables raw mode on drop, even during panics/early returns.
struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

fn is_required(item: &str, category: &Category) -> bool {
    matches!(
        item,
        "module-design" | "architecture-design" | "error-handling" | "architecture-proposal"
    ) || (item == "existing-code-exploration" && matches!(category, Category::Refactor))
}

fn compute_defaults(complexity: &Complexity, category: &Category) -> Vec<bool> {
    DESIGN_ITEMS
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

fn apply_required(selected: &mut [bool], category: &Category) {
    for (i, &item) in DESIGN_ITEMS.iter().enumerate() {
        if is_required(item, category) {
            selected[i] = true;
        }
    }
}

fn is_action_disabled(row: usize, selected: &[bool], category: &Category) -> bool {
    if row == 0 {
        selected.iter().all(|&s| s)
    } else if row == 1 {
        DESIGN_ITEMS
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
        is_required(DESIGN_ITEMS[item_idx], category)
    }
}

fn skip_disabled(
    cursor: usize,
    direction: isize,
    total: usize,
    selected: &[bool],
    category: &Category,
) -> usize {
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

    // Guidance
    let recommendation = match complexity {
        Complexity::High | Complexity::Medium => {
            "(recommend selecting all for medium and high complexity)"
        }
        Complexity::Low => {
            if selected.iter().all(|&s| s) {
                "(recommend only a few of these for low complexity)"
            } else {
                ""
            }
        }
    };
    if recommendation.is_empty() {
        write!(stdout, "  Confirm desired project design steps.\r\n")?;
    } else {
        write!(
            stdout,
            "  Confirm desired project design steps. {}\r\n",
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

    // Action rows
    let all_selected = selected.iter().all(|&s| s);
    let all_cleared = DESIGN_ITEMS
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
                style("  ")
            };
            let label = format!("{}", style(*action).dim());
            write!(stdout, "  {marker}{label}\r\n")?;
        } else {
            let marker = if is_cursor {
                style("\u{203a} ").bold()
            } else {
                style("  ")
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
    for (i, &item) in DESIGN_ITEMS.iter().enumerate() {
        let row = i + ACTION_ROWS;
        let required = is_required(item, category);
        let is_selected = selected[i];
        let is_cursor = row == cursor_row;

        let marker = if is_cursor {
            style("\u{203a} ").bold()
        } else {
            style("  ")
        };

        let checkbox = if required {
            format!("{}", style("[\u{25a0}]").cyan())
        } else if is_selected {
            format!("{}", style("[x]").green())
        } else {
            format!("{}", style("[ ]"))
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

pub fn design_select(
    complexity: &Complexity,
    category: &Category,
) -> RexResult<DesignSelectResult> {
    let mut stdout = io::stdout();
    let total_rows = ACTION_ROWS + DESIGN_ITEMS.len();

    // Header (printed before raw mode)
    println!();
    println!("  {}", style("Design Steps").bold().underlined());
    println!();

    let mut selected = compute_defaults(complexity, category);
    let mut cursor_row: usize =
        skip_disabled(ACTION_ROWS.saturating_sub(1), 1, total_rows, &selected, category);
    let mut total_lines: usize = 0;

    terminal::enable_raw_mode()?;
    let _guard = RawModeGuard;

    render(
        &mut stdout,
        cursor_row,
        &selected,
        category,
        complexity,
        &mut total_lines,
    )?;

    let result = loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match code {
                    KeyCode::Up => {
                        cursor_row =
                            skip_disabled(cursor_row, -1, total_rows, &selected, category);
                        render(
                            &mut stdout,
                            cursor_row,
                            &selected,
                            category,
                            complexity,
                            &mut total_lines,
                        )?;
                    }
                    KeyCode::Down => {
                        cursor_row =
                            skip_disabled(cursor_row, 1, total_rows, &selected, category);
                        render(
                            &mut stdout,
                            cursor_row,
                            &selected,
                            category,
                            complexity,
                            &mut total_lines,
                        )?;
                    }
                    KeyCode::Char(' ') => {
                        if cursor_row == 0 {
                            // select-all
                            if !is_action_disabled(0, &selected, category) {
                                selected.iter_mut().for_each(|s| *s = true);
                                cursor_row = skip_disabled(
                                    cursor_row,
                                    1,
                                    total_rows,
                                    &selected,
                                    category,
                                );
                                render(
                                    &mut stdout,
                                    cursor_row,
                                    &selected,
                                    category,
                                    complexity,
                                    &mut total_lines,
                                )?;
                            }
                        } else if cursor_row == 1 {
                            // clear-all
                            if !is_action_disabled(1, &selected, category) {
                                selected.iter_mut().for_each(|s| *s = false);
                                apply_required(&mut selected, category);
                                cursor_row = skip_disabled(
                                    cursor_row,
                                    1,
                                    total_rows,
                                    &selected,
                                    category,
                                );
                                render(
                                    &mut stdout,
                                    cursor_row,
                                    &selected,
                                    category,
                                    complexity,
                                    &mut total_lines,
                                )?;
                            }
                        } else {
                            let item_idx = cursor_row - ACTION_ROWS;
                            if !is_required(DESIGN_ITEMS[item_idx], category) {
                                selected[item_idx] = !selected[item_idx];
                                render(
                                    &mut stdout,
                                    cursor_row,
                                    &selected,
                                    category,
                                    complexity,
                                    &mut total_lines,
                                )?;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let selected_items: Vec<String> = DESIGN_ITEMS
                            .iter()
                            .zip(&selected)
                            .filter_map(|(&item, &sel)| sel.then(|| item.to_owned()))
                            .collect();
                        break Ok(DesignSelectResult { selected_items });
                    }
                    KeyCode::Esc => {
                        selected = compute_defaults(complexity, category);
                        render(
                            &mut stdout,
                            cursor_row,
                            &selected,
                            category,
                            complexity,
                            &mut total_lines,
                        )?;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        break Err(RexError::Cancelled);
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
            "  {} Design: {}/{} items selected",
            style("\u{2713}").green().bold(),
            selected_count,
            DESIGN_ITEMS.len()
        );
    }

    result
}
