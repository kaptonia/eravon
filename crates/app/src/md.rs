#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Heading(usize, String),
    Item(String),
    Quote(String),
    Row(Vec<String>),
    Para(String),
    Blank,
}

pub fn blocks(md: &str) -> Vec<Block> {
    md.lines()
        .filter(|line| !is_rule(line))
        .map(|line| {
            let t = line.trim_end();
            if t.trim().is_empty() {
                return Block::Blank;
            }
            let hashes = t.chars().take_while(|c| *c == '#').count();
            if (1..=6).contains(&hashes) && t.chars().nth(hashes) == Some(' ') {
                return Block::Heading(hashes, t[hashes + 1..].trim().to_string());
            }
            if let Some(rest) = t.trim_start().strip_prefix("> ") {
                return Block::Quote(rest.to_string());
            }
            let l = t.trim_start();
            if let Some(rest) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
                return Block::Item(rest.to_string());
            }
            if let Some(rest) = numbered(l) {
                return Block::Item(rest);
            }
            if l.starts_with('|') && l.ends_with('|') && l.len() > 2 {
                return Block::Row(l.trim_matches('|').split('|').map(|c| c.trim().to_string()).collect());
            }
            Block::Para(t.to_string())
        })
        .collect()
}

fn numbered(line: &str) -> Option<String> {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    (digits > 0 && line[digits..].starts_with(". ")).then(|| line[digits + 2..].to_string())
}

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.ends_with('|') && t.len() > 2 && t.trim_matches('|').chars().all(|c| matches!(c, '-' | ':' | '|' | ' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# 判词\n\n正文一段。\n\n- 第一项\n2. 第二项\n\n> 引一句\n\n| 甲 | 乙 |\n|---|---|\n| 1 | 2 |\n";

    #[test]
    fn the_four_kinds_come_out_as_their_own_blocks() {
        let got = blocks(DOC);
        assert_eq!(got[0], Block::Heading(1, "判词".into()));
        assert!(got.contains(&Block::Para("正文一段。".into())));
        assert!(got.contains(&Block::Item("第一项".into())));
        assert!(got.contains(&Block::Item("第二项".into())), "`1.` 那一形也是列表项");
        assert!(got.contains(&Block::Quote("引一句".into())));
        assert_eq!(got.iter().filter(|b| matches!(b, Block::Row(_))).count(), 2, "表头与一行体,分隔线不成块");
    }

    #[test]
    fn a_line_it_cannot_read_comes_out_whole() {
        let odd = "#没空格不是标题\n#######七个井号\n-没空格不是列表";
        let got = blocks(odd);
        assert_eq!(got[0], Block::Para("#没空格不是标题".into()));
        assert_eq!(got[1], Block::Para("#######七个井号".into()));
        assert_eq!(got[2], Block::Para("-没空格不是列表".into()));
    }
}
