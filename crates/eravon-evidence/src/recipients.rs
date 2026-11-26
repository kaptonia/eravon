use crate::mark;

pub type Pick = galeed_evidence::Pick;
pub type Binding = galeed_evidence::Binding;
pub type Hole = galeed_evidence::Hole;
pub type Refused = galeed_evidence::Refused;
pub type Unsealed = galeed_evidence::Unsealed;
pub type Sealed = galeed_evidence::Sealed;

pub fn form_ok(r: &str) -> bool {
    mark::mark(mark::FEATURE);
    galeed_evidence::recipient_form(r)
}

pub fn pick(audit_input: &eravon_galeed::Value) -> Pick {
    mark::mark(mark::FEATURE);
    galeed_evidence::pick(audit_input)
}

pub fn seal(to: &Binding, name: &str, media_type: &str, plaintext: &[u8]) -> Result<Sealed, Unsealed> {
    mark::mark(mark::FEATURE);
    galeed_evidence::seal(to, name, media_type, plaintext)
}
