---
name: rex-ui-ratatui
description: Build terminal UIs with ratatui (v0.30+) following Rust ergonomic rules. Covers Elm-style update/view loop, StatefulWidget, layout, async events via crossterm EventStream, ratatui-image, color-eyre panic-restore, release profile. Use when user wants TUI, says "ratatui", "terminal UI", "TUI app", "crossterm", or pipeline orchestrator dispatches `rex-ui-ratatui` step.
disable-model-invocation: false
user-invocable: true
---

# rex-ui-ratatui

Build terminal UIs with ratatui. Apply `rex-code-ergonomics` (newtypes, no bool flags in pub fn, `&[T]`/`&str`/`&Path` args, methods on types, no lifetimes in structs unless needed) and `rex-code-commenting` (no WHAT comments) throughout.

I/O contract for pipeline dispatch lives in `rex-utils-task-request`.

## Quick start

Pick template, copy, run:

```bash
cp -r .claude/skills/rex-ui-ratatui/assets/templates/<template>/* .
cargo run
```

| Template | When |
|----------|------|
| `hello-world` | Smallest demo. Learning |
| `simple-app` | Single-screen sync tool |
| `async-app` | Background tasks / network |
| `component-app` | Multi-view, config, logging |

## Cargo

```toml
[dependencies]
ratatui    = "0.30"
crossterm  = { version = "0.29", features = ["event-stream"] }
color-eyre = "0.6"
tokio      = { version = "1", features = ["full"] }     # async-app+
futures    = "0.3"                                      # async-app+
ratatui-image = { version = "5", features = ["chafa-static"] }  # if images

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

## Architecture: Elm-style

Model → Message → update → view → repeat.

```rust
struct App { counter: i32, should_quit: bool }
enum Msg { Inc, Dec, Quit }

impl App {
    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Inc => self.counter += 1,
            Msg::Dec => self.counter -= 1,
            Msg::Quit => self.should_quit = true,
        }
    }
    fn view(&self, frame: &mut Frame) {
        frame.render_widget(Paragraph::new(format!("Counter: {}", self.counter)), frame.area());
    }
}
```

## Styling — `Stylize` trait

```rust
use ratatui::style::Stylize;

"hello".bold().cyan().on_dark_gray()
```

Don't write `Style::default().fg(Color::White)`. Verbose, hardcodes themes.

Palette: `.cyan()`/`.green()` primary, `.red()` error, `.yellow()` warn, `.dim()`/`.dark_gray()` muted, `.magenta()` accent.

## Layout

```rust
let [header, body, footer] = Layout::vertical([
    Constraint::Length(1),
    Constraint::Fill(1),
    Constraint::Length(1),
]).areas(frame.area());
```

`Constraint::{Length, Fill, Percentage, Min, Max}`. Destructure with `.areas()`.

## StatefulWidget

```rust
struct MyList { items: Vec<String> }
struct MyListState { selected: usize }

impl StatefulWidget for MyList {
    type State = MyListState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) { /* ... */ }
}

frame.render_stateful_widget(my_list, area, &mut state);
```

Built-in: `ListState`, `TableState`, `ScrollbarState`.

## Async event loop

```rust
use crossterm::event::{EventStream, Event, KeyCode};
use futures::StreamExt;

async fn run(mut app: App, mut term: Terminal<impl Backend>) -> Result<()> {
    let mut events = EventStream::new();
    while !app.should_quit {
        term.draw(|f| app.view(f))?;
        tokio::select! {
            Some(Ok(Event::Key(key))) = events.next() => match key.code {
                KeyCode::Char('q') => app.update(Msg::Quit),
                KeyCode::Up        => app.update(Msg::Inc),
                KeyCode::Down      => app.update(Msg::Dec),
                _ => {}
            }
        }
    }
    Ok(())
}
```

Add channels (background tasks, timers) as extra `select!` arms.

## Images via ratatui-image

```rust
use ratatui_image::{picker::Picker, StatefulImage, Resize};

let mut picker = Picker::from_query_stdio()?;
let (tx, rx) = std::sync::mpsc::channel();
std::thread::spawn(move || {
    let img = image::open("photo.png").unwrap();
    let proto = picker.new_protocol(img, area.into(), Resize::Fit(None));
    tx.send(proto).unwrap();
});
if let Ok(proto) = rx.try_recv() { image_state = Some(proto); }
if let Some(ref mut img) = image_state {
    frame.render_stateful_widget(StatefulImage::default(), area, img);
}
```

Query terminal once. Resize off-thread. `StatefulImage` reuses encoded protocol across redraws.

## Panic-safe terminal restore

```rust
fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original(info);
    }));
    run()
}
```

Without this, panic leaves the terminal in raw mode. User has to `reset`. Don't ship a TUI without it.

## Errors

`color_eyre::Result` in bins. `?` everywhere. `.wrap_err("...")` at boundaries. No `.unwrap()` outside tests. See `rex-code-error-writing`.

## Common patterns

Centered popup:

```rust
fn centered(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [_, mid_v, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ]).areas(area);
    let [_, mid, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ]).areas(mid_v);
    mid
}
```

Key-binding hint line:

```rust
Line::from(vec![
    " q ".bold().cyan(), "quit ".dim(),
    " ↑↓ ".bold().cyan(), "navigate ".dim(),
    " Enter ".bold().cyan(), "select ".dim(),
])
```

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| `Style::default().fg(Color::White)` | Verbose, hardcodes theme | `.white()` via `Stylize` |
| `bool` flag in widget constructor | Two bools = caller confusion | Enum option type |
| Per-frame `Picker::from_query_stdio()` | Hits stdio every redraw | Query once at startup |
| `.unwrap()` in render path | Panics during draw → broken terminal | Bubble `Result` to main |
| Lifetime annotation on `App` struct | Viral pain across modules | Owned `String`, not `&'a str` |
| `// Render` / `// Handle events` | WHAT-comments. Insult reader | Delete. Names already say it |
| Ship without panic hook | `reset` for the user | Install hook in `main` |

## Ship checklist

- [ ] `cargo fmt`
- [ ] `cargo clippy --all-features` clean
- [ ] No `.unwrap()` outside tests
- [ ] Panic hook restores terminal
- [ ] `cargo build --release` succeeds
- [ ] Tested on target terminal (kitty / alacritty / Terminal.app / Windows Terminal)

## Deeper dives

- [`references/architecture-patterns.md`](references/architecture-patterns.md) — modular component structure
- [`references/async-patterns.md`](references/async-patterns.md) — channels, timers, background tasks
- [`references/image-integration.md`](references/image-integration.md) — ratatui-image deep dive
- [`references/style-guide.md`](references/style-guide.md) — full styling reference
