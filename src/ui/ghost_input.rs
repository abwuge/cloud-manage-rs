use console::{Key, Term, measure_text_width, style};

/// Fish-shell-style prompt: a fixed `prefix` plus a `default_suffix` rendered
/// as ghost text. Chars left of the cursor are bright, chars right are dim.
/// Typing overwrites in place; arrows / Home / End / Backspace / Del all work.
/// Enter accepts the whole buffer (so an untouched default is accepted too).
pub fn ghost_input(label: &str, prefix: &str, default_suffix: &str) -> std::io::Result<String> {
    let term = Term::stdout();
    let mut chars: Vec<char> = default_suffix.chars().collect();
    let mut cursor: usize = 0;

    // Header visible width (used for layout) vs. the styled version actually written.
    let header_visible = format!("? {} \u{203A} {}", label, prefix);
    let header_w = measure_text_width(&header_visible);
    let header_styled = format!(
        "{} {} {} {}",
        style("?").yellow().bold(),
        label,
        style("\u{203A}").black().bright(),
        prefix,
    );

    let mut prev_total_lines: usize = 0;
    let mut prev_cursor_row: usize = 0;

    let term_w = || -> usize {
        let (_, cols) = term.size();
        let cols = cols as usize;
        if cols == 0 { 80 } else { cols }
    };

    render(
        &term,
        &header_styled,
        header_w,
        &chars,
        cursor,
        &mut prev_total_lines,
        &mut prev_cursor_row,
        false,
    )?;

    loop {
        match term.read_key()? {
            Key::Enter => break,
            Key::ArrowLeft => {
                if cursor > 0 {
                    cursor -= 1;
                }
            }
            Key::ArrowRight => {
                if cursor < chars.len() {
                    cursor += 1;
                }
            }
            Key::ArrowUp => {
                let w = term_w();
                let abs = header_w + cursor;
                let new_abs = abs.saturating_sub(w);
                // Clamp at header boundary so the cursor never enters the prompt.
                cursor = if new_abs >= header_w {
                    new_abs - header_w
                } else {
                    0
                };
            }
            Key::ArrowDown => {
                let w = term_w();
                let abs = header_w + cursor;
                let new_abs = abs + w;
                let max_abs = header_w + chars.len();
                cursor = if new_abs <= max_abs {
                    new_abs - header_w
                } else {
                    chars.len()
                };
            }
            Key::Home => cursor = 0,
            Key::End => cursor = chars.len(),
            Key::Backspace => {
                if cursor > 0 {
                    chars.remove(cursor - 1);
                    cursor -= 1;
                }
            }
            Key::Del => {
                if cursor < chars.len() {
                    chars.remove(cursor);
                }
            }
            Key::Char(c) if !c.is_control() => {
                if cursor < chars.len() {
                    chars[cursor] = c;
                } else {
                    chars.push(c);
                }
                cursor += 1;
            }
            _ => continue,
        }
        render(
            &term,
            &header_styled,
            header_w,
            &chars,
            cursor,
            &mut prev_total_lines,
            &mut prev_cursor_row,
            true,
        )?;
    }

    let suffix: String = chars.iter().collect();
    let full = format!("{}{}", prefix, suffix);

    cleanup(&term, prev_total_lines, prev_cursor_row)?;
    println!(
        "{} {} \u{00B7} {}",
        style("\u{2714}").green(),
        label,
        style(&full).cyan(),
    );

    Ok(full)
}

#[allow(clippy::too_many_arguments)]
fn render(
    term: &Term,
    header_styled: &str,
    header_w: usize,
    chars: &[char],
    cursor: usize,
    prev_total_lines: &mut usize,
    prev_cursor_row: &mut usize,
    cleanup_first: bool,
) -> std::io::Result<()> {
    if cleanup_first {
        cleanup(term, *prev_total_lines, *prev_cursor_row)?;
    } else {
        term.write_str("\r")?;
        term.clear_line()?;
    }

    let term_w = {
        let (_, cols) = term.size();
        let cols = cols as usize;
        if cols == 0 { 80 } else { cols }
    };

    term.write_str(header_styled)?;
    let typed: String = chars[..cursor].iter().collect();
    let ghost: String = chars[cursor..].iter().collect();
    term.write_str(&typed)?;
    if !ghost.is_empty() {
        term.write_str(&style(&ghost).black().bright().to_string())?;
    }

    // Layout math assumes 1-column-per-char (true for ASCII-only inputs here).
    let total_w = header_w + chars.len();
    let cursor_abs_w = header_w + cursor;
    let total_lines = if total_w == 0 {
        1
    } else {
        (total_w + term_w - 1) / term_w
    };
    let after_row = if total_w == 0 {
        0
    } else {
        (total_w - 1) / term_w
    };
    let target_row = cursor_abs_w / term_w;
    let target_col = cursor_abs_w % term_w;

    let up = after_row.saturating_sub(target_row);
    if up > 0 {
        term.move_cursor_up(up)?;
    }
    term.write_str("\r")?;
    if target_col > 0 {
        term.move_cursor_right(target_col)?;
    }

    *prev_total_lines = total_lines.max(1);
    *prev_cursor_row = target_row;
    Ok(())
}

/// Erase the previously rendered region. Cursor ends at the top-left.
fn cleanup(term: &Term, total_lines: usize, cursor_row: usize) -> std::io::Result<()> {
    if total_lines == 0 {
        return Ok(());
    }
    // Walk to the bottom row, clear it, then clear everything above with
    // `clear_last_lines` (which clears N rows above the current and parks the
    // cursor at the top of the cleared region).
    let down = (total_lines - 1).saturating_sub(cursor_row);
    if down > 0 {
        term.move_cursor_down(down)?;
    }
    term.write_str("\r")?;
    term.clear_line()?;
    if total_lines > 1 {
        term.clear_last_lines(total_lines - 1)?;
    }
    Ok(())
}
