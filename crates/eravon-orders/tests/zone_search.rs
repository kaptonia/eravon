use eravon_orders::table::{search, Row};
use eravon_orders::when;

const AT: u64 = 1_790_293_572;

fn row(number: &str, placed: u64) -> Row {
    Row {
        instance: String::new(),
        case_id: String::new(),
        record: String::new(),
        number: number.into(),
        buyer: String::new(),
        label: String::new(),
        product: String::new(),
        amount: String::new(),
        token: String::new(),
        status: Err(eravon_orders::why("E_ORDERS_NONE", "-")),
        deadline: 0,
        placed,
        block: None,
        chain: None,
        local: None,
        handed: false,
        note: String::new(),
        terms_hash: String::new(),
        decimals: None,
        past: false,
        logic: String::new(),
        series: None,
    }
}

#[test]
fn a_date_range_files_an_order_under_the_day_of_the_zone_chosen() {
    let rows = [row("R-1", AT)];
    let on = |d: &str| search(&rows, "", d, d).map(|v| v.len()).unwrap_or(99);
    assert!(when::inject(when::Probe::Offset(8 * 3600)), "the stand-in zone is given before any read");
    when::set(when::Zone::Utc);
    assert_eq!((on("2026-09-24"), on("2026-09-25")), (1, 0), "UTC: the 24th");
    assert_eq!(when::day(AT), "2026-09-24");
    when::set(when::Zone::System);
    assert_eq!((on("2026-09-24"), on("2026-09-25")), (0, 1), "eight hours east: the 25th");
    assert_eq!(when::when(AT), "2026-09-25 07:46:12");
    assert_eq!(eravon_orders::day::of(AT), "2026-09-24", "the machine's date stays UTC");
}
