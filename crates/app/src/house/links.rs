use crate::house::desk::Site;
use crate::house::seller::{Said, Tone};
use crate::house::shell::{Seat, Toast};
use crate::shell::Shell;
use crate::task::Kind;

impl Shell {
    pub(crate) fn take_url(&mut self, u: &str) {
        use crate::house::action::Action as Step;
        if self.house_in_flight(Kind::PayLink) || self.page_in_flight(Kind::Bo) || self.covered() {
            self.desk.pending_url.push_back(u.to_string());
            self.desk.sh.toast(Toast { tone: Tone::Note, head: crate::house::i18n::t(crate::house::i18n::Key::ToastLinkQueued), next: String::new(), detail: None });
            self.desk.flush();
            return;
        }
        let d = crate::house::link::dispatch(u);
        self.desk.sh.links.push((d.kind, d.bytes));
        let known = d.kind != crate::house::seam::LINK_UNKNOWN;
        let head = format!("{}:{}", crate::house::i18n::t(if known { crate::house::i18n::Key::ToastLinkGot } else { crate::house::i18n::Key::ToastLinkUnknown }), d.kind);
        let data = self.desk.data.clone();
        let key = crate::house::paylink::land_link(&mut self.desk.sh, &data, u, &d);
        let switching = self.desk.seat() != Seat::of_store(eravon_vault::vault::seat_of(self.settings.role));
        let refused = switching && matches!(crate::house::act::family_said_started(self, crate::action::Action::SwitchRole), crate::action::Applied::Trouble(_));
        if self.desk.sh.paylink.as_ref().is_some_and(|p| p.wants_open) {
            self.spawn_paylink_open();
        }
        if self.desk.seat() == Seat::Buyer && self.desk.sh.seller.page() == Some(crate::house::seller::Page::PayLink) {
            if switching {
                if !refused {
                    self.desk.goto_landed = Some(crate::house::seller::Page::PayLink);
                }
            } else {
                self.desk.goto = Some(Some(crate::house::seller::Page::PayLink));
            }
        }
        if !switching {
            self.refresh_addr();
        }
        let next = crate::house::i18n::get_named(crate::lang::lang(), key).0;
        let head = if key == "toast.linkPay" { crate::house::i18n::t(crate::house::i18n::Key::ToastLinkPayGot) } else { head };
        self.desk.sh.toast(Toast { tone: if known { Tone::Note } else { Tone::Bad }, head, next, detail: None });
        self.desk.tr.push(Step::Url, crate::house::mark::FEATURE, format!("{} · {} 字节", d.kind, d.bytes));
        self.desk.flush();
        let _: Option<Said> = None;
    }

    pub(crate) fn settle_from_link(&mut self, text: String) {
        match crate::house::asked(crate::house::buyer_orders::BOrders::settle_plan(&text)) {
            Err(e) => {
                self.desk.sh.toast(Toast { tone: Tone::Bad, head: e.detail.clone(), next: String::new(), detail: Some(e.name.clone()) });
                if let Some(b) = self.desk.sh.buyer_orders.as_mut() {
                    b.tell(e);
                }
            }
            Ok(()) => {
                self.spawn_bo_at(Site::Page, crate::house::flow::Ask::Asked, move |s, data, me, _| {
                    let mut b = crate::house::buyer_orders::BOrders::load(s, data, me);
                    b.land_settle(data, me, &text);
                    b
                });
            }
        }
    }
}
