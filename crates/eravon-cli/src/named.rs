use crate::fault::Fault;
use crate::out::Said;

pub fn credential(t: &eravon_seal::Trouble) -> Option<Fault> {
    use eravon_seal::Code as C;
    let said = match t.code {
        C::CredentialExpired => Said::CredentialExpired,
        C::CredentialMissing => Said::CredentialMissing,
        C::CredentialShape => Said::CredentialShape,
        C::CredentialPassMissing => Said::CredentialPassMissing,
        C::CredentialPassUnknown => Said::CredentialPassUnknown,
        _ => return None,
    };
    let token = crate::sealed::word(t.code);
    let (reason, _) = crate::codes_own::desk_reason(&token)?;
    Some(Fault::Named { name: reason.as_str(), why: format!("{}({token} {})", said.text(), t.tail) })
}
