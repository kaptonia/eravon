use crate::json;

#[derive(Clone, Debug)]
pub struct Family {
    pub name: String,
    pub source: String,
    pub literals: Vec<String>,
}

const FENCE_OPEN: &str = "```json";
const FENCE_CLOSE: &str = "```";

pub fn fenced_block(doc: &str) -> Result<&str, &'static str> {
    let mut found: Option<(usize, usize)> = None;
    let mut line_start = 0usize;
    let mut open_at: Option<usize> = None;
    for line in doc.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\n', '\r']);
        let next = line_start + line.len();
        match open_at {
            None => {
                if trimmed == FENCE_OPEN {
                    open_at = Some(next);
                }
            }
            Some(start) => {
                if trimmed == FENCE_CLOSE {
                    if found.is_some() {
                        return Err("E_FOREIGN_MANY_BLOCKS");
                    }
                    found = Some((start, line_start));
                    open_at = None;
                }
            }
        }
        line_start = next;
    }
    if open_at.is_some() {
        return Err("E_FOREIGN_UNCLOSED");
    }
    match found {
        Some((a, b)) => Ok(&doc[a..b]),
        None => Err("E_FOREIGN_NO_BLOCK"),
    }
}

fn own_family(v: &json::In) -> Result<&str, &'static str> {
    if let Some(f) = v.get("family").and_then(|x| x.as_str()) {
        return Ok(f);
    }
    let r = v.get("registry").and_then(|x| x.as_str()).ok_or("E_FOREIGN_NO_REGISTRY")?;
    Ok(r.split('-').next().unwrap_or(r))
}

fn registry_of(doc: &str) -> Result<json::In, &'static str> {
    if doc.trim_start().starts_with('{') {
        return json::read(doc.as_bytes());
    }
    json::read(fenced_block(doc)?.as_bytes())
}

pub fn literals_of(doc: &str, family: &str, source: &str) -> Result<Family, &'static str> {
    let v = registry_of(doc)?;
    if own_family(&v)? == family {
        return Ok(Family { name: family.to_string(), source: source.to_string(), literals: own_entries(&v)? });
    }
    foreign_entries(&v, family, source)
}


pub const FOREIGN_READ_ROLES: [&str; 2] = ["foreign-document", "zikaron-contact"];

fn own_entries(v: &json::In) -> Result<Vec<String>, &'static str> {
    let arr = v.get("entries").and_then(|x| x.as_arr()).ok_or("E_FOREIGN_NO_ENTRIES")?;
    let mut out = Vec::with_capacity(arr.len());
    for e in arr {
        let s = e.get("literal").and_then(|x| x.as_str()).ok_or("E_FOREIGN_ENTRY_FORM")?;
        if e.get("role").and_then(|x| x.as_str()).is_some_and(|r| FOREIGN_READ_ROLES.contains(&r)) {
            continue;
        }
        out.push(s.to_string());
    }
    if out.is_empty() {
        return Err("E_FOREIGN_EMPTY");
    }
    Ok(out)
}

fn foreign_entries(v: &json::In, family: &str, source: &str) -> Result<Family, &'static str> {
    let arr = v
        .get("foreign")
        .and_then(|f| f.get(family))
        .and_then(|f| f.get("literals"))
        .and_then(|x| x.as_arr())
        .ok_or("E_FOREIGN_NO_FAMILY")?;
    let mut out = Vec::with_capacity(arr.len());
    for e in arr {
        out.push(e.as_str().ok_or("E_FOREIGN_ENTRY_FORM")?.to_string());
    }
    if out.is_empty() {
        return Err("E_FOREIGN_EMPTY");
    }
    let mut src = String::from(source);
    src.push_str(" · foreign.");
    src.push_str(family);
    Ok(Family { name: family.to_string(), source: src, literals: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "说明\n\n```json\n{\"registry\":\"galeed-family-literals\",\
\"entries\":[{\"literal\":\"galeed/1\"},{\"literal\":\"galeed:guild:\"}],\
\"foreign\":{\"zikaron\":{\"literals\":[\"zikaron/1\",\"zikaron.kit/1\"]}}}\n```\n尾巴\n";

    #[test]
    fn own_family_comes_from_entries() {
        let f = literals_of(DOC, "galeed", "样本").expect("取得到");
        assert_eq!(f.literals, vec!["galeed/1", "galeed:guild:"]);
    }

    #[test]
    fn a_foreign_family_comes_from_the_foreign_block() {
        let f = literals_of(DOC, "zikaron", "样本").expect("取得到");
        assert_eq!(f.literals, vec!["zikaron/1", "zikaron.kit/1"]);
    }

    #[test]
    fn a_family_the_document_does_not_carry_is_named_not_guessed() {
        assert_eq!(literals_of(DOC, "hermes", "样本").err(), Some("E_FOREIGN_NO_FAMILY"));
    }

    #[test]
    fn a_bare_json_registry_is_read_by_its_family_name() {
        let bare = "{\"schema\":\"kin-family-literals/1\",\"family\":\"kin\",\"entries\":[{\"role\":\"name\",\"literal\":\"KIN\",\"at\":\"x\",\"says\":\"y\"},{\"role\":\"file-magic\",\"literal\":\"kin-local/1\\n\",\"at\":\"x\",\"says\":\"y\"}],\"pairs\":[]}";
        let f = literals_of(bare, "kin", "样本").expect("取得到");
        assert_eq!(f.literals, vec!["KIN", "kin-local/1\n"]);
        assert_eq!(literals_of(bare, "galeed", "样本").err(), Some("E_FOREIGN_NO_FAMILY"));
    }

    #[test]
    fn a_document_with_no_block_is_named() {
        assert_eq!(fenced_block("只有散文\n").err(), Some("E_FOREIGN_NO_BLOCK"));
    }

    #[test]
    fn rows_a_registry_marks_as_another_familys_are_not_its_literals() {
        let doc = "```json\n{\"registry\":\"galeed-family-literals\",\"entries\":[\
{\"literal\":\"galeed/1\",\"role\":\"signing-domain\"},\
{\"literal\":\"chelek.proposal/1\",\"role\":\"foreign-document\"},\
{\"literal\":\".zikaron-desk\",\"role\":\"zikaron-contact\"}]}\n```\n";
        let f = literals_of(doc, "galeed", "t").expect("读得出");
        assert_eq!(f.literals, vec!["galeed/1".to_string()]);
    }
}
