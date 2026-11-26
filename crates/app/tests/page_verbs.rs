fn src(rel: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn code(rel: &str) -> String {
    let s = src(rel);
    let mut out = String::new();
    let mut rest = s.as_str();
    while let Some(i) = rest.find("#[cfg(test)]\nmod ") {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        let Some(open) = after.find('{') else { break };
        let mut depth = 0usize;
        let mut end = after.len();
        for (j, ch) in after[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + j + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

const PAGES: &[(&str, &str, &[&str], &str, &[&str])] = &[
    ("candidates list", "app/src/house/seller/cands.rs", &["eravon_cli::deskops::candsview::{brief_out"], "eravon-cli/src/seller/cands.rs", &["candsview::brief_out", "eravon_cands::view::list_on"]),
    ("candidate detail", "app/src/house/seller/cands.rs", &["eravon_cli::deskops::candsview::detail_out", "eravon_cli::deskops::candsview::codes"], "eravon-cli/src/seller/cands.rs", &["candsview::detail_out", "candsview::codes", "eravon_cands::view::detail_on"]),
    ("seller orders list", "app/src/house/orders/answer.rs", &["eravon_cli::deskops::orderpage::page_row"], "eravon-cli/src/seller/orders.rs", &["deskops::orderpage::row_cells"]),
    ("seller orders row", "eravon-cli/src/deskops/orderpage.rs", &["let mut m = row_cells(r);"], "eravon-cli/src/seller/orders.rs", &["deskops::orderpage::row_cells"]),
    ("buyer orders list", "app/src/house/buyer_orders.rs", &["eravon_cli::deskops::buyerpage::rows_of", "eravon_cli::deskops::buyerpage::shown_of"], "eravon-cli/src/buyer/orders.rs", &["bp::rows_of", "bp::shown_of", "bp::row_out"]),
    ("buyer orders row", "app/src/house/buyer_orders/answer.rs", &["eravon_cli::deskops::buyerpage::row_out", "eravon_cli::deskops::buyerpage::acts("], "eravon-cli/src/buyer/orders.rs", &["bp::row_out", "bp::read_only"]),
    ("seller order card", "app/src/house/orders/answer.rs", &["eravon_cli::deskops::orderpage::detail_out", "eravon_cli::deskops::orderpage::page_row"], "eravon-cli/src/seller/orders.rs", &["deskops::orderpage::read_detail", "deskops::orderpage::detail_out"]),
    ("buyer order card", "app/src/house/buyer_orders/answer.rs", &["eravon_cli::deskops::buyerpage::detail_out"], "eravon-cli/src/buyer/orders.rs", &["bp::detail_out", "bp::fee_line"]),
    ("payment link page", "app/src/house/paylink/verify.rs", &["eravon_cli::deskops::paypage::verify_listing", "eravon_cli::deskops::paypage::opened_rows"], "eravon-cli/src/buyer/order.rs", &["pp::verify_listing", "pp::opened_rows"]),
];

#[test]
fn every_page_and_its_verb_answer_from_one_build() {
    for (page, app, app_calls, verb, verb_calls) in PAGES {
        let a = code(app);
        for c in *app_calls {
            assert!(a.contains(c), "{page}:{app} 不经 {c} 出页答面");
        }
        let v = code(verb);
        for c in *verb_calls {
            assert!(v.contains(c), "{page}:{verb} 不经 {c} 出动词的答");
        }
    }
}

#[test]
fn the_window_heads_and_rows_come_from_the_shared_build() {
    let w = code("app/src/window/house/buyer_orders.rs");
    assert!(w.contains("eravon_cli::deskops::buyerpage::pre_state(r)"), "买家订单页的页头词不经共用构造");
    let p = code("app/src/house/paylink.rs");
    assert!(p.contains("pub use eravon_cli::deskops::paypage::{Cand, Opened, State};"), "付款链接页的行不是共用构造的型");
}

#[test]
fn the_rotation_list_reads_cases_as_the_order_page_does() {
    let r = code("eravon-cli/src/vaultverbs/rotate.rs");
    assert!(r.contains("eravon_orders::table::table(data, key, eravon_scan::scan::Side::Seller"), "换钥清单的在途案不读订单页那一张表");
    assert!(r.contains("Table::complete"), "表没读全也答成全集");
}

#[test]
fn the_complaint_sheet_and_its_verbs_are_one_build() {
    let page = code("app/src/house/complaint.rs");
    assert!(page.contains("pub use eravon_cli::deskops::complaint::"), "窗的投诉单没取共用的那一份");
    for own in ["fn write(", "fn seated", "fn picked", "fn files_of", "fn guild_of_listing", "registry::guilds_listing"] {
        assert!(!page.contains(own), "窗里另有一份 {own}");
    }
    let seller = code("app/src/house/orders/judge.rs");
    assert!(seller.contains("deskops::complaint::seller_about"), "卖家订单页的投诉另判");
    let buyer = code("app/src/house/buyer_orders/judge.rs");
    assert!(buyer.contains("deskops::complaint::buyer_about"), "买家订单页的投诉另判");
    let verbs = code("eravon-cli/src/complaintverbs.rs");
    for call in ["cp::buyer_about", "cp::seller_about", "cp::read(", "cp::write(", "cp::complaint_out", "orderpage::order_company"] {
        assert!(verbs.contains(call), "投诉动词没调 {call}");
    }
    let orders = code("app/src/house/orders.rs");
    assert!(orders.contains("eravon_cli::deskops::orderpage::order_company"), "订单页的公司路另判");
}

#[test]
fn each_order_card_is_one_read_and_one_build() {
    let read = code("app/src/house/orders/list.rs");
    assert!(read.contains("eravon_cli::deskops::orderpage::read_detail"), "卖家订单卡不经共用读口");
    let page_files = ["app/src/house/orders/list.rs", "app/src/house/orders/answer.rs", "app/src/house/buyer_orders/answer.rs", "app/src/house/buyer_orders.rs"];
    for f in page_files {
        let c = code(f);
        for own in ["eravon_orders::detail::overview", "eravon_orders::detail::delivery", "eravon_orders::detail::dispute", "eravon_orders::detail::record(", "eravon_orders::detail::fee(", "fn fee_rows", "fn earns", "eravon_wallet::tx::fee_total"] {
            assert!(!c.contains(own), "{f} 另读或另拼订单卡:{own}");
        }
    }
    let buyer = code("app/src/house/buyer_orders/answer.rs");
    assert!(buyer.contains("eravon_cli::deskops::buyerpage::fee_line(row, earns, s2)"), "买家费用页签的判官费另判");
}
