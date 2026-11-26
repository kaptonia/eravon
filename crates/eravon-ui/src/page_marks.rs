#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageMark {
    People,
    Money,
    Box,
    Bell,
    Link,
    Upload,
    Card,
    Scale,
    Bolt,
    Building,
    Coin,
    Stones,
}

impl PageMark {
    pub const ALL: [PageMark; 12] = [
        PageMark::People,
        PageMark::Money,
        PageMark::Box,
        PageMark::Bell,
        PageMark::Link,
        PageMark::Upload,
        PageMark::Card,
        PageMark::Scale,
        PageMark::Bolt,
        PageMark::Building,
        PageMark::Coin,
        PageMark::Stones,
    ];
}

fn ellipse24(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<(f32, f32)> {
    let n = 32;
    (0..=n)
        .map(|k| {
            let a = (k as f32 / n as f32) * std::f32::consts::TAU;
            ((cx + rx * a.cos()) / 24.0, (cy + ry * a.sin()) / 24.0)
        })
        .collect()
}

fn h(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    points.iter().map(|(x, y)| (x / 24.0, y / 24.0)).collect()
}

fn arc24(cx: f32, cy: f32, r: f32, from: f32, to: f32) -> Vec<(f32, f32)> {
    arc(cx * 16.0 / 24.0, cy * 16.0 / 24.0, r * 16.0 / 24.0, from, to)
}

fn rounded24(x: f32, y: f32, w: f32, hh: f32, r: f32) -> Vec<(f32, f32)> {
    let mut v = Vec::new();
    v.extend(arc24(x + w - r, y + r, r, -90.0, 0.0));
    v.extend(arc24(x + w - r, y + hh - r, r, 0.0, 90.0));
    v.extend(arc24(x + r, y + hh - r, r, 90.0, 180.0));
    v.extend(arc24(x + r, y + r, r, 180.0, 270.0));
    if let Some(first) = v.first().copied() {
        v.push(first);
    }
    v
}

fn g(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    points.iter().map(|(x, y)| (x / 16.0, y / 16.0)).collect()
}

fn arc(cx: f32, cy: f32, r: f32, from: f32, to: f32) -> Vec<(f32, f32)> {
    let n = 16;
    (0..=n)
        .map(|k| {
            let a = (from + (to - from) * k as f32 / n as f32).to_radians();
            ((cx + r * a.cos()) / 16.0, (cy + r * a.sin()) / 16.0)
        })
        .collect()
}

pub fn path(m: PageMark) -> Vec<Vec<(f32, f32)>> {
    match m {
        PageMark::People => vec![
            arc(6.0, 5.0, 2.5, 0.0, 360.0),
            arc(6.0, 14.0, 4.5, 180.0, 360.0),
            arc(11.5, 5.5, 2.0, -90.0, 90.0),
            arc(11.0, 14.0, 4.0, 270.0, 360.0),
        ],
        PageMark::Box => vec![
            g(&[(2.0, 5.0), (8.0, 2.0), (14.0, 5.0), (14.0, 11.0), (8.0, 14.0), (2.0, 11.0), (2.0, 5.0)]),
            g(&[(2.0, 5.0), (8.0, 8.0), (14.0, 5.0)]),
            g(&[(8.0, 8.0), (8.0, 14.0)]),
        ],
        PageMark::Money => vec![
            g(&[(2.0, 4.5), (14.0, 4.5), (14.0, 11.5), (2.0, 11.5), (2.0, 4.5)]),
            arc(8.0, 8.0, 1.8, 0.0, 360.0),
            g(&[(4.5, 8.0), (4.6, 8.0)]),
            g(&[(11.4, 8.0), (11.5, 8.0)]),
        ],
        PageMark::Bell => {
            let mut body = arc(8.0, 6.0, 4.0, 180.0, 360.0);
            body.extend(g(&[(12.0, 9.0), (13.0, 11.0), (3.0, 11.0), (4.0, 9.0), (4.0, 6.0)]));
            vec![body, arc(8.0, 13.0, 2.0, 0.0, 180.0)]
        }
        PageMark::Link => vec![
            g(&[(6.2, 9.8), (9.8, 6.2)]),
            g(&[(7.4, 4.8), (9.2, 3.0), (11.0, 2.6), (12.6, 3.4), (13.4, 5.0), (13.0, 6.8), (11.2, 8.6)]),
            g(&[(8.6, 11.2), (6.8, 13.0), (5.0, 13.4), (3.4, 12.6), (2.6, 11.0), (3.0, 9.2), (4.8, 7.4)]),
        ],
        PageMark::Upload => {
            let mut tray = h(&[(5.0, 13.0), (5.0, 19.0)]);
            tray.extend(arc24(6.0, 19.0, 1.0, 180.0, 90.0));
            tray.extend(h(&[(18.0, 20.0)]));
            tray.extend(arc24(18.0, 19.0, 1.0, 90.0, 0.0));
            tray.extend(h(&[(19.0, 13.0)]));
            vec![h(&[(12.0, 15.0), (12.0, 3.0)]), h(&[(7.0, 8.0), (12.0, 3.0), (17.0, 8.0)]), tray]
        }
        PageMark::Card => vec![rounded24(3.0, 5.0, 18.0, 14.0, 2.0), h(&[(3.0, 10.0), (21.0, 10.0)]), h(&[(7.0, 15.0), (11.0, 15.0)])],
        PageMark::Scale => {
            let pan = |x: f32| {
                let mut v = h(&[(x, 7.0), (x - 3.5, 14.0)]);
                v.extend(arc24(x, 14.0, 3.5, 180.0, 0.0));
                v.extend(h(&[(x, 7.0)]));
                v
            };
            vec![h(&[(12.0, 3.0), (12.0, 21.0)]), h(&[(5.0, 7.0), (19.0, 7.0)]), pan(7.5), pan(16.5), h(&[(8.0, 21.0), (16.0, 21.0)])]
        }
        PageMark::Bolt => vec![h(&[(13.0, 3.0), (5.0, 13.0), (11.0, 13.0), (10.0, 21.0), (18.0, 11.0), (12.0, 11.0), (13.0, 3.0)])],
        PageMark::Building => {
            let mut tower = h(&[(4.0, 21.0), (4.0, 5.0)]);
            tower.extend(arc24(5.0, 5.0, 1.0, 180.0, 270.0));
            tower.extend(h(&[(14.0, 4.0)]));
            tower.extend(arc24(14.0, 5.0, 1.0, 270.0, 360.0));
            tower.extend(h(&[(15.0, 21.0)]));
            let mut wing = h(&[(15.0, 9.0), (19.0, 9.0)]);
            wing.extend(arc24(19.0, 10.0, 1.0, 270.0, 360.0));
            wing.extend(h(&[(20.0, 21.0)]));
            vec![tower, wing, h(&[(3.0, 21.0), (21.0, 21.0)]), h(&[(8.0, 8.0), (11.0, 8.0)]), h(&[(8.0, 12.0), (11.0, 12.0)]), h(&[(8.0, 16.0), (11.0, 16.0)])]
        }
        PageMark::Coin => vec![
            ellipse24(12.0, 12.0, 8.5, 8.5),
            h(&[(14.8, 9.2), (13.8, 8.1), (12.0, 7.8), (10.1, 8.2), (9.2, 9.9), (10.2, 11.3), (12.0, 12.0), (13.8, 12.6), (14.8, 14.1), (14.0, 15.7), (12.0, 16.2), (10.2, 15.9), (9.1, 14.8)]),
            h(&[(12.0, 6.0), (12.0, 7.8)]),
            h(&[(12.0, 16.2), (12.0, 18.0)]),
        ],
        PageMark::Stones => vec![ellipse24(12.0, 5.0, 3.2, 1.7), ellipse24(12.0, 11.3, 5.3, 2.1), ellipse24(12.0, 18.3, 7.6, 2.4)],
    }
}

pub fn draw(p: &egui::Painter, m: PageMark, rect: egui::Rect, colour: egui::Color32) {
    let stroke = egui::Stroke::new(crate::tokens::ICON_STROKE, colour);
    for stroke_path in path(m) {
        let pts: Vec<egui::Pos2> = stroke_path.iter().map(|(x, y)| egui::pos2(rect.left() + x * rect.width(), rect.top() + y * rect.height())).collect();
        if pts.len() >= 2 {
            p.add(egui::Shape::line(pts, stroke));
        }
    }
}

pub fn at(p: &egui::Painter, m: PageMark, center: egui::Pos2, size: f32, colour: egui::Color32) {
    draw(p, m, egui::Rect::from_center_size(center, egui::vec2(size, size)), colour);
}

pub const ICON_MASTER_SIDE: usize = 1024;

pub fn master_rgba(png: &[u8]) -> Result<(usize, Vec<u8>), String> {
    let be = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize;
    if png.len() < 8 || png[..8] != [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a] {
        return Err("不是 PNG".into());
    }
    let (mut at, mut side, mut z) = (8usize, 0usize, Vec::new());
    while at + 12 <= png.len() {
        let len = be(&png[at..]);
        let kind = &png[at + 4..at + 8];
        let body = png.get(at + 8..at + 8 + len).ok_or("块截断了")?;
        match kind {
            b"IHDR" => {
                if body.len() < 13 || be(body) != be(&body[4..]) || body[8..13] != [8, 6, 0, 0, 0] {
                    return Err("母本要是方形 8 位 RGBA、不隔行".into());
                }
                side = be(body);
            }
            b"IDAT" => z.extend_from_slice(body),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    let row = side * 4 + 1;
    let mut raw = Vec::with_capacity(side * row);
    let mut i = 2;
    while i + 5 <= z.len() {
        let last = z[i] & 1;
        if z[i] & 0b110 != 0 {
            return Err("母本的图要是 zlib 存储块".into());
        }
        let n = usize::from(u16::from_le_bytes([z[i + 1], z[i + 2]]));
        raw.extend_from_slice(z.get(i + 5..i + 5 + n).ok_or("存储块截断了")?);
        i += 5 + n;
        if last == 1 {
            break;
        }
    }
    if side == 0 || raw.len() != side * row || raw.chunks(row).any(|r| r[0] != 0) {
        return Err("母本的行要是不过滤的整幅".into());
    }
    Ok((side, raw.chunks(row).flat_map(|r| r[1..].iter().copied()).collect()))
}

pub fn icon_at(master: &[u8], side: usize, n: usize) -> Vec<u8> {
    let step = side as f64 / n.max(1) as f64;
    let span = |o: usize| {
        let (lo, hi) = (o as f64 * step, (o + 1) as f64 * step);
        (lo.floor() as usize..(hi.ceil() as usize).min(side)).map(move |s| (s, (hi.min((s + 1) as f64) - lo.max(s as f64)).max(0.0)))
    };
    let mut out = Vec::with_capacity(n * n * 4);
    for oy in 0..n {
        for ox in 0..n {
            let mut acc = [0f64; 4];
            let mut weight = 0f64;
            for (sy, wy) in span(oy) {
                for (sx, wx) in span(ox) {
                    let w = wx * wy;
                    let p = &master[(sy * side + sx) * 4..][..4];
                    let a = f64::from(p[3]) / 255.0;
                    for c in 0..3 {
                        acc[c] += f64::from(p[c]) * a * w;
                    }
                    acc[3] += a * w;
                    weight += w;
                }
            }
            let a = if weight > 0.0 { acc[3] / weight } else { 0.0 };
            let colour = |c: usize| if acc[3] > 0.0 { (acc[c] / acc[3]).round().clamp(0.0, 255.0) as u8 } else { 0 };
            out.extend_from_slice(&[colour(0), colour(1), colour(2), (a * 255.0).round().clamp(0.0, 255.0) as u8]);
        }
    }
    out
}
