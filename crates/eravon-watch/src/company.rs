use eravon_standard as a1;

use crate::order::{self, Order};
use crate::{why, Why, Wire};

fn a1w(f: a1::fault::Fault) -> Why {
    (f.name(), f.detail())
}

pub fn open(w: &Wire<'_>, data: &str, me: &str, o: &Order, from: &str, now: u64) -> Result<(a1::proposal::Plan, String), Why> {
    crate::mark::mark(crate::mark::FEATURE);
    let d = crate::poll::deal(data, me, o, w)?;
    if d.past {
        return Err(why(a1::fault::PAST_LISTING, a1::fault::PAST_LISTING_SAYS));
    }
    if d.guild {
        return Err(why("E_WATCH_COMPANY_GUILD", "行会轨上公司不经提案开案:这一单由买家开(条款须许买家开案)"));
    }
    let instance = d.derived.instance.clone();
    let deploy = if a1::chain::has_code(w.t, w.table, &instance).map_err(a1w)? {
        a1::chain::code_matches(w.t, w.table, &instance, w.logic)
            .map_err(|f| why("E_WATCH_COMPANY_CODE", format!("实例地址 {instance} 上不是本标准的码:{}", f.detail())))?;
        let (phase, _) = crate::poll::phase(w, &instance)?;
        if phase != 0 {
            return Err(why(crate::poll::E_OPEN_OPENED, format!("{instance} 的案已开(阶段 {phase}),不必再开")));
        }
        None
    } else {
        Some(a1::call::deploy_instance(&d.derived.salt, w.logic).map_err(a1w)?)
    };
    let (sig_seller, sig_buyer) = crate::act::opening_slots(w, &d.o, &d.sig_seller, &d.sig_buyer).map_err(a1w)?;
    crate::act::opening(w, &d.o, &instance, &sig_seller, &sig_buyer, now, from).map_err(a1w)?;
    let passed = crate::act::check(w, &d.o).map_err(|r| crate::act::why_of(&r))?;
    let init = a1::call::init(&d.o, &instance, &passed.quote, &sig_buyer, &sig_seller, &[], &[]).map_err(a1w)?;
    let listing = order::part(data, me, &o.id, &order::part_listing())?;
    let receipt = order::part(data, me, &o.id, &order::part_receipt())?;
    let originals = vec![
        a1::proposal::abi_doc(&[a1::sol::INIT], a1::proposal::INSTANCE_ABI_NAME).map_err(a1w)?,
        a1::proposal::Original { kind: eravon_literals::literals::DOC_LISTING.into(), name: "listing.json".into(), bytes: listing },
        a1::proposal::Original { kind: eravon_literals::literals::DOC_RECEIPT.into(), name: "receipt.json".into(), bytes: receipt },
    ];
    let plan = a1::proposal::Plan {
        verb: "init",
        calls: deploy.iter().chain(std::iter::once(&init)).map(|c| a1::proposal::of_call(c).map_err(a1w)).collect::<Result<_, _>>()?,
        originals,
        items: vec![a1::company::Item::Deadline { name: "openBy", who: String::new(), at: crate::poll::open_until(&d.o) }],
        face: vec![format!("开案:{}init 由公司直接调,实例 {instance};原件是这一单的发布件加回执。", if deploy.is_some() { "部署与 " } else { "" })],
        instance,
    };
    Ok((plan, d.o.buyer_key.to_ascii_lowercase()))
}
