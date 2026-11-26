pub use eravon_cli::deskops::complaint::{company_party, files_arg, files_of, home, listing_unread, picked, said_of, seated, write, written_say, Case, Complaint, Form, Guild};

use crate::house::seller::Said;

pub fn read(data: &str, me: &str, instance: &str, case_id: &str, judge: &str, pool: Result<Option<String>, Said>, now: u64) -> Complaint {
    let w = eravon_cands::wire::Wire::of(data, me, &crate::house::orders::injected(), now);
    eravon_cli::deskops::complaint::read(w, instance, case_id, judge, pool)
}
