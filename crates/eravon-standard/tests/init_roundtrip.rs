use eravon_standard::{call, order};
use eravon_terms as k2;

fn order_of(buyer_may_open: bool) -> order::Order {
    let a = |c: char| format!("0x{}", c.to_string().repeat(40));
    order::Order {
        offer: k2::derive::Offer {
            seller_key: a('1'),
            judge: a('2'),
            guild: a('0'),
            seller_payout: a('3'),
            seller_fund: a('4'),
            token: a('5'),
            amount: k2::u256::U256::from_dec("1000000000").expect("十进制"),
            delivery_window: 604_800,
            confirm_window: 259_200,
            open_window: 86_400,
            buyer_may_open,
            ladder: vec![k2::doc::Rung { buyer_bps: 0, seller_bps: 10_000, fee_bearer: 0 }, k2::doc::Rung { buyer_bps: 10_000, seller_bps: 0, fee_bearer: 1 }],
            terms_hash: [0x11; 32],
            amanah_hash: [0x22; 32],
            quote_hash: [0x33; 32],
            nonce: 7,
            expiry: 1_800_000_000,
        },
        buyer_key: a('6'),
        buyer_payout: a('7'),
        open_by: 1_700_086_400,
        prev: order::NO_PREV.to_string(),
    }
}

#[test]
fn init_calldata_reads_back_field_for_field() {
    for bmo in [false, true] {
        let o = order_of(bmo);
        let c = call::init(&o, &format!("0x{}", "9".repeat(40)), &[0xab; 480], &[0xcd; 65], &[], &[], &[]).expect("写得出");
        let back = order::decode_init(&c.data).expect("读得回");
        assert_eq!(back.order.offer.buyer_may_open, bmo);
        assert_eq!(back.order.offer.ladder.len(), 2);
        assert_eq!((back.order.offer.nonce, back.order.offer.expiry, back.order.open_by), (7, 1_800_000_000, 1_700_086_400));
        assert_eq!(back.order.offer.amount.to_dec(), "1000000000");
        assert_eq!((back.quote.len(), back.sig_buyer.len(), back.sig_seller.len()), (480, 65, 0));
        assert_eq!(back.order.offer.amanah_hash, [0x22; 32]);
    }
}
