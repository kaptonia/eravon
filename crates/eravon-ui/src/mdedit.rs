use crate::palette::C;
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Heading,
    Bold,
    Italic,
    List,
    Numbered,
    Quote,
    Code,
}

pub const TOOLS: [Tool; 7] = [Tool::Heading, Tool::Bold, Tool::Italic, Tool::List, Tool::Numbered, Tool::Quote, Tool::Code];

pub const ID_HEAD: &str = "eravon-mdedit";

#[derive(Clone, Copy, PartialEq, Default)]
struct Look {
    hidden: bool,
    bold: bool,
    italic: bool,
    code: bool,
    heading: u8,
    quote: bool,
    list: bool,
}

fn line_prefix(line: &str) -> (usize, Look) {
    let mut look = Look::default();
    for (marker, level) in [("### ", 3u8), ("## ", 2), ("# ", 1)] {
        if line.starts_with(marker) {
            look.heading = level;
            return (marker.chars().count(), look);
        }
    }
    if line.starts_with("- ") {
        look.list = true;
        return (2, look);
    }
    if line.starts_with("> ") {
        look.quote = true;
        return (2, look);
    }
    (0, look)
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
struct Style {
    bold: bool,
    italic: bool,
    code: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
struct Line {
    prefix: String,
    chars: Vec<(char, Style)>,
}

struct Parsed {
    line: Line,
    looks: Vec<Look>,
    at: Vec<usize>,
}

fn parse(line: &str) -> Parsed {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let (skip, base) = line_prefix(line);
    let mut hidden = vec![false; n];
    for h in hidden.iter_mut().take(skip) {
        *h = true;
    }
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut i = skip;
    while i < n {
        match chars[i] {
            '`' => match (i + 1..n).find(|&j| chars[j] == '`') {
                Some(j) => {
                    spans.push((i, j));
                    i = j + 1;
                }
                None => i += 1,
            },
            '\\' if i + 1 < n && matches!(chars[i + 1], '*' | '`' | '\\' | '-' | '>' | '#') => {
                hidden[i] = true;
                i += 2;
            }
            '*' => {
                let k = (i..n).find(|&j| chars[j] != '*').unwrap_or(n);
                if k - i <= 3 {
                    runs.push((i, k - i));
                }
                i = k;
            }
            _ => i += 1,
        }
    }
    let mut live = vec![true; runs.len()];
    loop {
        let mut changed = false;
        for flag in [2usize, 1] {
            let mine: Vec<usize> = (0..runs.len()).filter(|&r| live[r] && runs[r].1 & flag != 0).collect();
            if mine.len() % 2 == 1 {
                live[*mine.last().expect("odd count")] = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut toggle_at: Vec<usize> = vec![0; n];
    for (r, &(at, len)) in runs.iter().enumerate() {
        if live[r] {
            toggle_at[at] = len;
            for h in hidden.iter_mut().skip(at).take(len) {
                *h = true;
            }
        }
    }
    let mut in_code = vec![false; n];
    for &(a, b) in &spans {
        hidden[a] = true;
        hidden[b] = true;
        for c in in_code.iter_mut().take(b).skip(a + 1) {
            *c = true;
        }
    }
    let mut style = Style::default();
    let mut looks = Vec::with_capacity(n);
    let mut out = Line { prefix: chars[..skip].iter().collect(), chars: Vec::new() };
    let mut at = Vec::new();
    for j in 0..n {
        if toggle_at[j] & 2 != 0 {
            style.bold = !style.bold;
        }
        if toggle_at[j] & 1 != 0 {
            style.italic = !style.italic;
        }
        let st = Style { code: in_code[j], ..style };
        let mut l = base;
        l.hidden = hidden[j];
        l.bold = st.bold;
        l.italic = st.italic;
        l.code = st.code;
        looks.push(l);
        if !hidden[j] {
            out.chars.push((chars[j], st));
            at.push(j);
        }
    }
    Parsed { line: out, looks, at }
}

fn boundary(a: Style, b: Style, out: &mut String) {
    let stars = a.bold != b.bold || a.italic != b.italic;
    if a.code && (!b.code || stars) {
        out.push('`');
    }
    if a.bold != b.bold {
        out.push_str("**");
    }
    if a.italic != b.italic {
        out.push('*');
    }
    if b.code && (!a.code || stars) {
        out.push('`');
    }
}

fn write(line: &Line) -> String {
    let mut out = line.prefix.clone();
    let mut cur = Style::default();
    let seen: String = line.chars.iter().map(|(c, _)| *c).collect();
    let lead = line.prefix.is_empty() && line_prefix(&seen).0 > 0;
    for (k, &(c, st)) in line.chars.iter().enumerate() {
        let st = if c == '`' { Style { code: false, ..st } } else { st };
        boundary(cur, st, &mut out);
        if !st.code && (matches!(c, '*' | '`' | '\\') || (k == 0 && lead && out.is_empty())) {
            out.push('\\');
        }
        out.push(c);
        cur = st;
    }
    boundary(cur, Style::default(), &mut out);
    out
}

struct Doc {
    lines: Vec<Parsed>,
    starts: Vec<usize>,
}

fn read_doc(src: &str) -> Doc {
    let mut lines = Vec::new();
    let mut starts = Vec::new();
    let mut at = 0;
    for l in src.split('\n') {
        starts.push(at);
        at += l.chars().count() + 1;
        lines.push(parse(l));
    }
    Doc { lines, starts }
}

#[derive(Clone, Debug, PartialEq)]
enum Seen {
    Char(char, Style),
    Break(String),
}

impl Doc {
    fn seen(&self) -> Vec<Seen> {
        let mut out = Vec::new();
        for (i, p) in self.lines.iter().enumerate() {
            if i > 0 {
                out.push(Seen::Break(p.line.prefix.clone()));
            }
            out.extend(p.line.chars.iter().map(|&(c, s)| Seen::Char(c, s)));
        }
        out
    }

    fn place(&self, raw: usize) -> usize {
        let mut before = 0;
        for (i, p) in self.lines.iter().enumerate() {
            let s = self.starts[i];
            let len = p.looks.len();
            if raw <= s + len {
                let col = raw.saturating_sub(s);
                return before + p.at.iter().filter(|&&a| a < col).count();
            }
            before += p.line.chars.len() + 1;
        }
        before.saturating_sub(1)
    }

    fn in_prefix(&self, raw: usize) -> Option<usize> {
        (0..self.lines.len()).find(|&i| {
            let s = self.starts[i];
            let k = self.lines[i].line.prefix.chars().count();
            k > 0 && raw >= s && raw < s + k
        })
    }

    fn raw(&self, place: usize, before_next: bool) -> usize {
        let mut left = place;
        for (i, p) in self.lines.iter().enumerate() {
            let s = self.starts[i];
            let n = p.line.chars.len();
            if left <= n {
                if before_next && left < n {
                    return s + p.at[left];
                }
                return if left == 0 { s + p.line.prefix.chars().count() } else { s + p.at[left - 1] + 1 };
            }
            left -= n + 1;
        }
        self.starts.last().copied().unwrap_or(0) + self.lines.last().map_or(0, |p| p.looks.len())
    }
}

fn write_seen(seen: &[Seen], first_prefix: &str) -> String {
    let mut lines = vec![Line { prefix: first_prefix.to_string(), chars: Vec::new() }];
    for s in seen {
        match s {
            Seen::Char(c, st) => lines.last_mut().expect("one line at least").chars.push((*c, *st)),
            Seen::Break(prefix) => lines.push(Line { prefix: prefix.clone(), chars: Vec::new() }),
        }
    }
    lines.iter().map(write).collect::<Vec<_>>().join("\n")
}

fn line_looks(line: &str) -> Vec<Look> {
    parse(line).looks
}

fn reinterpret(old: &str, new: &str, old_sel: (usize, usize), new_sel: (usize, usize)) -> (String, (usize, usize)) {
    let o: Vec<char> = old.chars().collect();
    let w: Vec<char> = new.chars().collect();
    let cp = o.iter().zip(&w).take_while(|(a, b)| a == b).count().min(old_sel.0).min(new_sel.0);
    let max_cs = o.len().min(w.len()) - cp;
    let cs = o.iter().rev().zip(w.iter().rev()).take(max_cs).take_while(|(a, b)| a == b).count().min(o.len().saturating_sub(old_sel.1.max(cp))).min(w.len().saturating_sub(new_sel.1.max(cp)));
    let (del_a, del_b) = (cp, o.len() - cs);
    let ins: Vec<char> = w[cp..w.len() - cs].to_vec();
    let doc = read_doc(old);
    let mut seen = doc.seen();
    let first_prefix = doc.lines.first().map_or_else(String::new, |p| p.line.prefix.clone());
    let (mut fa, mut fb) = (doc.place(del_a), doc.place(del_b));
    let mut first_prefix = first_prefix;
    if fa == fb && del_a < del_b && ins.is_empty() {
        match doc.in_prefix(del_a) {
            Some(0) => first_prefix.clear(),
            Some(line) => {
                let at = (0..seen.len()).filter(|&k| matches!(seen[k], Seen::Break(_))).nth(line - 1);
                if let Some(k) = at {
                    seen[k] = Seen::Break(String::new());
                }
            }
            None if new_sel.0 < old_sel.0 || old_sel.0 >= del_b => fa = fa.saturating_sub(1),
            None => fb = (fb + 1).min(seen.len()),
        }
    }
    let style_here = |seen: &[Seen], k: usize| -> Style {
        match (k.checked_sub(1).and_then(|j| seen.get(j)), seen.get(k)) {
            (Some(Seen::Char(_, st)), _) => *st,
            (_, Some(Seen::Char(_, st))) => *st,
            _ => Style::default(),
        }
    };
    let line_prefix_at = |seen: &[Seen], k: usize| -> String {
        (0..k).rev().find_map(|j| if let Seen::Break(p) = &seen[j] { Some(p.clone()) } else { None }).unwrap_or_else(|| first_prefix.clone())
    };
    let mut put: Vec<Seen> = Vec::new();
    let mut st = style_here(&seen, fa);
    if fb > fa {
        if let Some(Seen::Char(_, s0)) = seen.get(fa) {
            st = *s0;
        }
    }
    let carried = line_prefix_at(&seen, fa);
    for c in &ins {
        if *c == '\n' {
            let keep = if ins.len() == 1 && (carried.starts_with("- ") || carried.starts_with("> ")) { carried.clone() } else { String::new() };
            put.push(Seen::Break(keep));
        } else {
            put.push(Seen::Char(*c, st));
        }
    }
    let put_n = put.len();
    seen.splice(fa..fb, put);
    let text = write_seen(&seen, &first_prefix);
    let doc = read_doc(&text);
    let sel = if new_sel.0 == new_sel.1 || put_n == 0 {
        let at = doc.raw(fa + put_n, false);
        (at, at)
    } else {
        (doc.raw(fa, true), doc.raw(fa + put_n, false))
    };
    (text, sel)
}

fn settle(src: &str, a: usize, b: usize) -> (usize, usize) {
    let doc = read_doc(src);
    if a == b {
        let at = doc.raw(doc.place(a), false);
        return (at, at);
    }
    let (lo, hi) = (a.min(b), a.max(b));
    let (pl, ph) = (doc.place(lo), doc.place(hi));
    if pl == ph {
        let at = doc.raw(pl, false);
        return (at, at);
    }
    (doc.raw(pl, true), doc.raw(ph, false))
}

fn step(src: &str, from: usize, to: usize) -> usize {
    let doc = read_doc(src);
    let total = doc.seen().len();
    let (p0, p1) = (doc.place(from), doc.place(to));
    let p = match (to.cmp(&from), p1 == p0) {
        (std::cmp::Ordering::Greater, true) => (p0 + 1).min(total),
        (std::cmp::Ordering::Less, true) => p0.saturating_sub(1),
        _ => p1,
    };
    doc.raw(p, false)
}

fn format_of(l: &Look) -> TextFormat {
    if l.hidden {
        return TextFormat { font_id: FontId::new(1.0, FontFamily::Proportional), color: Color32::TRANSPARENT, ..Default::default() };
    }
    let size = match l.heading {
        1 => 20.0,
        2 => 17.0,
        3 => 15.0,
        _ if l.code => 14.0,
        _ => crate::tokens::Type::Body.size(),
    };
    let font = if l.code {
        FontId::new(size, FontFamily::Monospace)
    } else if l.bold || l.heading > 0 {
        FontId::new(size, crate::fonts::strong())
    } else {
        FontId::new(size, FontFamily::Proportional)
    };
    TextFormat {
        font_id: font,
        color: if l.quote { crate::palette::c(C::Ink2) } else { crate::palette::c(C::Ink) },
        italics: l.italic || l.quote,
        background: if l.code { crate::palette::c(C::Tag) } else { Color32::TRANSPARENT },
        line_height: Some(if l.heading == 1 { 28.0 } else { 24.0 }),
        ..Default::default()
    }
}

fn layout(src: &str, wrap: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let mut first_line = true;
    for line in src.split('\n') {
        if !first_line {
            job.append("\n", 0.0, format_of(&Look::default()));
        }
        first_line = false;
        let looks = line_looks(line);
        let (_, base) = line_prefix(line);
        let mut run = String::new();
        let mut run_look: Option<Look> = None;
        let mut lead = if base.list || base.quote { 18.0 } else { 0.0 };
        for (c, l) in line.chars().zip(looks) {
            if run_look.is_some_and(|r| r != l) {
                let r = run_look.take().expect("checked");
                job.append(&run, if r.hidden { 0.0 } else { std::mem::take(&mut lead) }, format_of(&r));
                run.clear();
            }
            run_look = Some(l);
            run.push(c);
        }
        if let Some(r) = run_look {
            job.append(&run, if r.hidden { 0.0 } else { lead }, format_of(&r));
        }
    }
    job
}

fn line_starts(src: &str) -> Vec<(usize, Look)> {
    let mut out = Vec::new();
    let mut at = 0;
    for line in src.split('\n') {
        out.push((at, line_prefix(line).1));
        at += line.chars().count() + 1;
    }
    out
}

fn wordish(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\''
}

fn restyle(src: &mut String, a: usize, b: usize, flag: fn(&mut Style) -> &mut bool) -> (usize, usize) {
    let doc = read_doc(src);
    let mut seen = doc.seen();
    let first_prefix = doc.lines.first().map_or_else(String::new, |p| p.line.prefix.clone());
    let (mut fa, mut fb) = (doc.place(a), doc.place(b));
    let cur = fa;
    let collapsed = fa == fb;
    if collapsed {
        let is_word = |k: usize| matches!(seen.get(k), Some(Seen::Char(c, _)) if wordish(*c));
        while fa > 0 && is_word(fa - 1) {
            fa -= 1;
        }
        while is_word(fb) {
            fb += 1;
        }
        if fa == fb {
            return (a, b);
        }
    }
    let is_code = {
        let mut probe = Style::default();
        *flag(&mut probe) = true;
        probe.code
    };
    let all = seen[fa..fb].iter().all(|x| match x {
        Seen::Char(c, st) if !(is_code && *c == '`') => {
            let mut st = *st;
            *flag(&mut st)
        }
        _ => true,
    });
    for x in &mut seen[fa..fb] {
        if let Seen::Char(_, st) = x {
            *flag(st) = !all;
        }
    }
    *src = write_seen(&seen, &first_prefix);
    let doc = read_doc(src);
    if collapsed {
        let at = doc.raw(cur, false);
        return (at, at);
    }
    (doc.raw(fa, true), doc.raw(fb, false))
}

fn mark_lines(src: &mut String, a: usize, b: usize, tool: Tool) -> (usize, usize) {
    let lines: Vec<String> = src.split('\n').map(str::to_string).collect();
    let starts: Vec<usize> = line_starts(src).into_iter().map(|(s, _)| s).collect();
    let touched: Vec<usize> = (0..lines.len()).filter(|&i| {
        let s = starts[i];
        let e = s + lines[i].chars().count();
        (s <= b && e >= a) || (a == b && s <= a && a <= e)
    }).collect();
    let has = |l: &str| match tool {
        Tool::Heading => l.starts_with("## "),
        Tool::List => l.starts_with("- "),
        Tool::Quote => l.starts_with("> "),
        _ => l.split_once(". ").is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())),
    };
    let all = touched.iter().all(|&i| has(&lines[i]));
    let mut delta_a: isize = 0;
    let mut delta_b: isize = 0;
    let mut out = Vec::with_capacity(lines.len());
    let mut n = 0;
    for (i, l) in lines.iter().enumerate() {
        if !touched.contains(&i) {
            out.push(l.clone());
            continue;
        }
        let (skip, _) = line_prefix(l);
        let numbered = l.split_once(". ").filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())).map(|(n, _)| n.chars().count() + 2).unwrap_or(0);
        let bare: String = l.chars().skip(skip.max(numbered)).collect();
        let new = if all {
            bare
        } else {
            n += 1;
            let pre = match tool {
                Tool::Heading => "## ".to_string(),
                Tool::List => "- ".to_string(),
                Tool::Quote => "> ".to_string(),
                _ => format!("{n}. "),
            };
            format!("{pre}{bare}")
        };
        let change = new.chars().count() as isize - l.chars().count() as isize;
        if starts[i] < a || (a == b && starts[i] <= a) {
            delta_a += change;
        }
        delta_b += change;
        out.push(new);
    }
    *src = out.join("\n");
    let len = src.chars().count() as isize;
    (((a as isize) + delta_a).clamp(0, len) as usize, ((b as isize) + delta_b).clamp(0, len) as usize)
}

fn apply(src: &mut String, a: usize, b: usize, tool: Tool) -> (usize, usize) {
    match tool {
        Tool::Bold => restyle(src, a, b, |s| &mut s.bold),
        Tool::Italic => restyle(src, a, b, |s| &mut s.italic),
        Tool::Code => restyle(src, a, b, |s| &mut s.code),
        other => {
            let (x, y) = mark_lines(src, a, b, other);
            settle(src, x, y)
        }
    }
}

pub fn md_editor(ui: &mut Ui, id: &str, src: &mut String, tools: [&str; 7], height: f32, extra: impl FnOnce(&mut Ui)) -> bool {
    let edit_id = ui.make_persistent_id((ID_HEAD, id));
    let mut changed = false;
    let mut pressed: Option<Tool> = None;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 8)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 4.0);
                for (tool, word) in TOOLS.iter().zip(tools) {
                    if crate::button::key(ui, word, crate::button::Role::Plain, true).clicked() {
                        pressed = Some(*tool);
                    }
                }
                let (sep, _) = ui.allocate_exact_size(egui::vec2(13.0, 18.0), egui::Sense::hover());
                ui.painter().vline(sep.center().x, sep.y_range(), egui::Stroke::new(1.0_f32, crate::palette::c(C::Line)));
                extra(ui);
            });
        });
        crate::paint::rule(ui, 0.0);
    });
    if pressed.is_none() && ui.memory(|m| m.has_focus(edit_id)) {
        let take = |ui: &mut Ui, key: egui::Key| ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key));
        if take(ui, egui::Key::B) {
            pressed = Some(Tool::Bold);
        } else if take(ui, egui::Key::I) {
            pressed = Some(Tool::Italic);
        }
    }
    if let Some(tool) = pressed {
        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), edit_id).unwrap_or_default();
        let len = src.chars().count();
        let [a, b] = state.cursor.char_range().map_or([CCursor::new(len), CCursor::new(len)], |r| r.sorted_cursors());
        let (na, nb) = apply(src, a.index.min(len), b.index.min(len), tool);
        state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(na), CCursor::new(nb))));
        state.store(ui.ctx(), edit_id);
        ui.memory_mut(|m| m.request_focus(edit_id));
        changed = true;
    }
    let mut layouter = move |ui: &Ui, buf: &dyn egui::TextBuffer, wrap_w: f32| ui.fonts_mut(|f| f.layout_job(layout(buf.as_str(), wrap_w)));
    let entry = src.clone();
    {
        let undoing = ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { key: egui::Key::Z | egui::Key::Y, pressed: true, modifiers, .. } if modifiers.command)));
        let before = src.clone();
        let old_sel = egui::text_edit::TextEditState::load(ui.ctx(), edit_id).and_then(|st| st.cursor.char_range()).map(|r| (r.secondary.index, r.primary.index));
        let arrow = ui.input(|i| i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowRight));
        let focused = ui.memory(|m| m.has_focus(edit_id));
        scrolled(ui, focused, egui::ScrollArea::vertical().id_salt((ID_HEAD, "scroll", id)).max_height(height).auto_shrink([false, false]), |ui| {
            egui::Frame::new().inner_margin(egui::Margin::symmetric(20, 16)).show(ui, |ui| {
                ui.set_min_height(height - 32.0);
                let mut composing = false;
                let mut out = crate::compose::around(ui, edit_id, src, |ui, src| {
                    composing = *src != entry || (ui.memory(|m| m.has_focus(edit_id)) && ui.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Ime(_)))));
                    egui::TextEdit::multiline(src).id(edit_id).desired_width(f32::INFINITY).desired_rows(12).frame(egui::Frame::NONE).layouter(&mut layouter).show(ui)
                });
                changed |= out.response.changed();
                if let (false, Some(r)) = (composing, out.state.cursor.char_range()) {
                    let (sec, pri) = (r.secondary.index, r.primary.index);
                    let len = before.chars().count();
                    let (os, op) = old_sel.unwrap_or((len, len));
                    let fixed = if out.response.changed() && !undoing {
                        let (text, (x, y)) = reinterpret(&before, src, (os.min(op), os.max(op)), (sec.min(pri), sec.max(pri)));
                        *src = text;
                        if sec <= pri { (x, y) } else { (y, x) }
                    } else if arrow && !out.response.changed() {
                        let at = step(src, op, pri);
                        if sec == pri { (at, at) } else { (sec, at) }
                    } else {
                        let (lo, hi) = settle(src, sec.min(pri), sec.max(pri));
                        if sec <= pri { (lo, hi) } else { (hi, lo) }
                    };
                    if fixed != (sec, pri) || out.response.changed() {
                        out.state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(fixed.0), CCursor::new(fixed.1))));
                        out.state.clone().store(ui.ctx(), edit_id);
                    }
                    if fixed != (sec, pri) || *src != out.galley.job.text {
                        ui.ctx().request_discard("md editor wrote the text or the cursor again");
                    }
                }
                let starts = line_starts(src);
                let mut at = 0usize;
                let mut next = starts.iter().peekable();
                for placed in &out.galley.rows {
                    while next.peek().is_some_and(|(s, _)| *s < at) {
                        next.next();
                    }
                    if let Some((s, look)) = next.peek() {
                        if *s == at {
                            let r = placed.rect().translate(out.galley_pos.to_vec2());
                            if look.list {
                                ui.painter().circle_filled(egui::pos2(r.left() + 6.0, r.center().y), 2.5, crate::palette::c(C::Ink));
                            } else if look.quote {
                                ui.painter().vline(r.left() + 5.0, r.y_range(), egui::Stroke::new(3.0_f32, crate::palette::c(C::Line)));
                            }
                        }
                    }
                    at += placed.char_count_including_newline();
                }
            });
        });
    }
    changed && *src != entry
}

fn scrolled<R>(ui: &mut Ui, focused: bool, area: egui::ScrollArea, add: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R> {
    if focused {
        return crate::scroll::nested(ui, area, add);
    }
    area.scroll_source(egui::scroll_area::ScrollSource { mouse_wheel: false, ..egui::scroll_area::ScrollSource::ALL }).show(ui, add)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_still_composing_never_reach_the_text() {
        let ctx = egui::Context::default();
        let _ = crate::fonts::install(&ctx, &crate::fonts::Found::none());
        let tools = ["h", "b", "i", "l", "n", "q", "c"];
        let mut src = String::from("ab");
        let frame = |events: Vec<egui::Event>, src: &mut String| {
            let raw = egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() };
            let _ = ctx.run_ui(raw, |root| {
                egui::CentralPanel::default().show_inside(root, |ui| {
                    let id = ui.make_persistent_id((ID_HEAD, "ime"));
                    ui.memory_mut(|m| m.request_focus(id));
                    md_editor(ui, "ime", src, tools, 200.0, |_| {});
                });
            });
        };
        frame(Vec::new(), &mut src);
        frame(vec![egui::Event::Ime(egui::ImeEvent::Enabled), egui::Event::Ime(egui::ImeEvent::Preedit("ni".into()))], &mut src);
        assert_eq!(src, "ab", "the letters being composed are not in the text");
        frame(Vec::new(), &mut src);
        assert_eq!(src, "ab", "nor in a frame with no word from the method");
        frame(vec![egui::Event::Ime(egui::ImeEvent::Preedit("nih".into()))], &mut src);
        assert_eq!(src, "ab");
        frame(vec![egui::Event::Ime(egui::ImeEvent::Commit("你".into())), egui::Event::Ime(egui::ImeEvent::Disabled)], &mut src);
        assert_eq!((src.matches('你').count(), src.contains('n'), src.chars().count()), (1, false, 3), "the committed character is put in: {src}");
        frame(Vec::new(), &mut src);
        assert_eq!(src.chars().count(), 3, "and stays: {src}");
    }

    fn edit_frame(ctx: &egui::Context, src: &mut String, events: Vec<egui::Event>, set: Option<(usize, usize)>) -> (Vec<String>, Option<(usize, usize)>) {
        let tools = ["h", "b", "i", "l", "n", "q", "c"];
        let raw = egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), ..Default::default() };
        let mut id = None;
        let out = ctx.run_ui(raw, |root| {
            egui::CentralPanel::default().show_inside(root, |ui| {
                let edit = ui.make_persistent_id((ID_HEAD, "b471"));
                ui.memory_mut(|m| m.request_focus(edit));
                if let Some((a, b)) = set {
                    let mut st = egui::text_edit::TextEditState::load(ui.ctx(), edit).unwrap_or_default();
                    st.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(a), CCursor::new(b))));
                    st.store(ui.ctx(), edit);
                }
                md_editor(ui, "b471", src, tools, 200.0, |_| {});
                id = Some(edit);
            });
        });
        let painted = out.shapes.iter().filter_map(|c| match &c.shape { egui::Shape::Text(t) => Some(t.galley.job.text.clone()), _ => None }).collect();
        let sel = id.and_then(|e| egui::text_edit::TextEditState::load(ctx, e)).and_then(|st| st.cursor.char_range()).map(|r| (r.secondary.index, r.primary.index));
        (painted, sel)
    }

    fn ctx_with_fonts() -> egui::Context {
        let ctx = egui::Context::default();
        let _ = crate::fonts::install(&ctx, &crate::fonts::Found::none());
        ctx
    }

    #[test]
    fn a_frame_paints_the_text_as_it_was_written_again() {
        let ctx = ctx_with_fonts();
        for (start, at, typed, want) in [("- a\nb", 3usize, egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }, "- a\n- \nb"), ("ab", 1, egui::Event::Text("*".into()), "a\\*b")] {
            let mut src = start.to_string();
            let _ = edit_frame(&ctx, &mut src, Vec::new(), Some((at, at)));
            let (painted, _) = edit_frame(&ctx, &mut src, vec![typed], None);
            assert_eq!(src, want);
            assert!(painted.iter().any(|t| *t == src), "the frame painted {painted:?}, the text is {src:?}");
        }
    }

    #[test]
    fn command_b_and_i_format_the_selection_and_type_nothing() {
        let ctx = ctx_with_fonts();
        let key = |k: egui::Key| egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::COMMAND };
        let mut src = "say hello now".to_string();
        let _ = edit_frame(&ctx, &mut src, Vec::new(), Some((4, 9)));
        let (_, sel) = edit_frame(&ctx, &mut src, vec![key(egui::Key::B)], None);
        assert_eq!(src, "say **hello** now", "bold wraps the selection");
        let (a, b) = sel.expect("a selection");
        let chars: Vec<char> = src.chars().collect();
        assert_eq!(chars[a.min(b)..a.max(b)].iter().collect::<String>(), "hello", "the selection stays on the word");
        let _ = edit_frame(&ctx, &mut src, vec![key(egui::Key::I)], None);
        assert_eq!(src, "say ***hello*** now", "italic joins it, the selection kept");
        let _ = edit_frame(&ctx, &mut src, vec![key(egui::Key::B)], None);
        assert_eq!(src, "say *hello* now", "pressed again, bold comes off");
        assert_eq!(shown(&src), "say hello now", "no letter typed");
    }

    #[test]
    fn wrapping_twice_takes_the_format_off() {
        let mut s = "a word here".to_string();
        let (a, b) = apply(&mut s, 2, 6, Tool::Bold);
        assert_eq!(s, "a **word** here");
        assert_eq!((a, b), (4, 8));
        let (a, b) = apply(&mut s, a, b, Tool::Bold);
        assert_eq!(s, "a word here");
        assert_eq!((a, b), (2, 6));
    }

    #[test]
    fn line_tools_mark_and_unmark_every_touched_line() {
        let mut s = "one\ntwo\nthree".to_string();
        let _ = apply(&mut s, 0, 6, Tool::List);
        assert_eq!(s, "- one\n- two\nthree");
        let _ = apply(&mut s, 0, 8, Tool::List);
        assert_eq!(s, "one\ntwo\nthree");
        let _ = apply(&mut s, 0, 6, Tool::Numbered);
        assert_eq!(s, "1. one\n2. two\nthree");
    }

    fn shown(src: &str) -> String {
        src.split('\n').map(|l| parse(l).line.chars.iter().map(|(c, _)| *c).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn styles_combine_and_come_off_one_at_a_time() {
        let mut s = "a word here".to_string();
        let (a, b) = apply(&mut s, 2, 6, Tool::Bold);
        let (a, b) = apply(&mut s, a, b, Tool::Italic);
        assert_eq!(s, "a ***word*** here");
        assert!(parse(&s).line.chars[2..6].iter().all(|(_, st)| st.bold && st.italic));
        let _ = apply(&mut s, a, b, Tool::Bold);
        assert_eq!(s, "a *word* here");
        assert_eq!(shown(&s), "a word here");
    }

    #[test]
    fn a_tool_across_lines_or_prefixes_formats_each_line_and_hides_every_marker() {
        let mut s = "- one\ntwo".to_string();
        let len = s.chars().count();
        let _ = apply(&mut s, 0, len, Tool::Bold);
        assert_eq!(s, "- **one**\n**two**");
        assert_eq!(shown(&s), "one\ntwo");
        let mut s = "x **bold** y".to_string();
        let _ = apply(&mut s, 5, 7, Tool::Code);
        assert_eq!(shown(&s), "x bold y");
        let p = parse(&s);
        assert!(p.line.chars[3].1.code && p.line.chars[3].1.bold && !p.line.chars[2].1.code);
    }

    #[test]
    fn a_tool_with_nothing_selected_takes_the_word_and_leaves_no_empty_pair() {
        let mut s = "say hello now".to_string();
        let _ = apply(&mut s, 6, 6, Tool::Bold);
        assert_eq!(s, "say **hello** now");
        let mut s = "a  b".to_string();
        let _ = apply(&mut s, 2, 2, Tool::Italic);
        assert_eq!(s, "a  b");
    }

    #[test]
    fn typing_and_deleting_edit_what_is_seen() {
        let (t, _) = reinterpret("ab", "a*b", (1, 1), (2, 2));
        assert_eq!(t, "a\\*b");
        assert_eq!(shown(&t), "a*b");
        let (t, sel) = reinterpret("**ab**", "**abc**", (4, 4), (5, 5));
        assert_eq!((t.as_str(), sel), ("**abc**", (5, 5)));
        let (t, _) = reinterpret("- item", "-item", (2, 2), (1, 1));
        assert_eq!(t, "item");
        let (t, _) = reinterpret("x**word**", "x*word**", (3, 3), (2, 2));
        assert_eq!(shown(&t), "word");
        assert!(parse(&t).line.chars.iter().all(|(_, st)| st.bold));
        let (t, _) = reinterpret("- item", "x- item", (0, 0), (1, 1));
        assert_eq!(t, "- xitem");
        assert_eq!(settle("- **ab**", 0, 0), (2, 2));
        assert_eq!(settle("- **ab**", 3, 3), (2, 2));
        assert_eq!(settle("- **ab**", 8, 8), (6, 6));
        assert_eq!(settle("**ab**", 0, 2), (0, 0));
        let (t, sel) = reinterpret("the cat", "the  cat", (3, 3), (4, 4));
        assert_eq!((t.as_str(), sel), ("the  cat", (4, 4)));
        let (t, _) = reinterpret("- a\nb", "- a\n\nb", (3, 3), (4, 4));
        assert_eq!(t, "- a\n- \nb");
        let (t, _) = reinterpret("a\n- b", "a\n\n- b", (1, 1), (2, 2));
        assert_eq!(t, "a\n\n- b");
        assert_eq!(step("a**b**", 1, 2), 4);
        assert_eq!(step("a**b**c", 4, 5), 7);
        assert_eq!(step("a**b**", 4, 3), 1);
    }

    #[test]
    fn seen_text_that_looks_like_a_prefix_or_a_list_stays_text() {
        for (seen, style) in [("- x", Style::default()), ("> q", Style::default()), ("# h", Style::default()), (" hi", Style { italic: true, ..Style::default() })] {
            let l = Line { prefix: String::new(), chars: seen.chars().map(|c| (c, style)).collect() };
            assert_eq!(parse(&write(&l)).line, l, "{seen}");
        }
        let mut s = "`a`\\``b`".to_string();
        let len = s.chars().count();
        let _ = apply(&mut s, 0, len, Tool::Code);
        assert!(parse(&s).line.chars.iter().all(|(_, st)| !st.code), "{s}");
    }

    #[test]
    fn written_lines_read_back_as_they_were() {
        for src in ["a ***b*** *c* `d` \\* e", "## **a*b***c", "> `x`**y**", "1. plain"] {
            let p = parse(src);
            assert_eq!(parse(&write(&p.line)).line, p.line, "{src}");
        }
    }

    #[test]
    fn markers_are_hidden_and_what_they_enclose_is_formatted() {
        let looks = line_looks("## a **b** *c* `d` e*f");
        let text: Vec<char> = "## a **b** *c* `d` e*f".chars().collect();
        let shown: String = text.iter().zip(&looks).filter(|(_, l)| !l.hidden).map(|(c, _)| *c).collect();
        assert_eq!(shown, "a b c d e*f");
        assert!(looks.iter().all(|l| l.heading == 2));
    }
}
