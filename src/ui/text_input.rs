use console::style;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, Write};

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

/// A text input that shows a dim placeholder when empty.
/// Tab fills the placeholder into the input for editing.
/// Enter submits the current input, or the placeholder if input is empty.
pub fn text_input(
    prompt: &str,
    placeholder: &str,
    validator: Option<&dyn Fn(&str) -> Option<String>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    let mut input = String::new();
    let mut error_msg: Option<String> = None;
    let mut lines_rendered: usize = 0;

    terminal::enable_raw_mode()?;
    let _guard = RawModeGuard;

    let value = loop {
        // Clear previous render
        if lines_rendered > 1 {
            execute!(
                stdout,
                cursor::MoveUp((lines_rendered - 1) as u16)
            )?;
        }
        if lines_rendered > 0 {
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                terminal::Clear(ClearType::FromCursorDown)
            )?;
        }

        lines_rendered = 1;

        if input.is_empty() {
            // Print prompt, save cursor position, then print dim placeholder
            let prompt_with_space = format!("{prompt} ");
            write!(stdout, "{prompt_with_space}{}", style(placeholder).dim())?;
            // Move cursor back to right after the prompt
            let placeholder_len = placeholder.len() as u16;
            execute!(stdout, cursor::MoveLeft(placeholder_len))?;
        } else {
            write!(stdout, "{prompt} {input}")?;
        }

        if let Some(ref err) = error_msg {
            write!(stdout, "\r\n  {}", style(err).red())?;
            lines_rendered = 2;
        }

        stdout.flush()?;

        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            error_msg = None;
            match code {
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    return Err("Cancelled".into());
                }
                KeyCode::Tab => {
                    if input.is_empty() && !placeholder.is_empty() {
                        input = placeholder.to_string();
                    }
                }
                KeyCode::Enter => {
                    let value = if input.is_empty() {
                        placeholder.to_string()
                    } else {
                        input.clone()
                    };
                    if let Some(validate) = &validator {
                        if let Some(err) = validate(&value) {
                            error_msg = Some(err);
                            continue;
                        }
                    }
                    break value;
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => {
                    input.push(c);
                }
                _ => {}
            }
        }
    };

    drop(_guard);

    // Clear interactive area and show final accepted value
    if lines_rendered > 1 {
        execute!(
            stdout,
            cursor::MoveUp((lines_rendered - 1) as u16)
        )?;
    }
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        terminal::Clear(ClearType::FromCursorDown)
    )?;
    println!(
        "{} {prompt} {value}",
        style("\u{2714}").green()
    );

    Ok(value)
}
