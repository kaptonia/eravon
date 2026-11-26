pub const TX_CAP: u64 = 1 << 24;

pub fn init_cost(n: u64) -> u64 {
    820_000u64.saturating_add(n.saturating_mul(2_470)).saturating_add(n.saturating_mul(n).saturating_mul(111))
}

pub fn draw_cost(n: u64) -> u64 {
    200_000u64.saturating_add(n.saturating_mul(4_300)).saturating_add(n.saturating_mul(n).saturating_mul(226))
}

pub fn routed_draw_cost(route_gas: u64) -> u64 {
    1_000_000u64.saturating_add(route_gas)
}

pub fn fits(n: u64, routed: bool) -> Result<(), String> {
    let init = init_cost(n);
    if init > TX_CAP {
        return Err(format!("名单 {n} 人:init 约 {init} gas,装不进一笔({TX_CAP})"));
    }
    let draw = if routed { routed_draw_cost(u64::from(eravon_literals::constants::ROUTE_GAS)) } else { draw_cost(n) };
    if draw > TX_CAP {
        return Err(format!("名单 {n} 人:最贵一抽约 {draw} gas,装不进一笔({TX_CAP})"));
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bound_covers_every_measured_point() {
        let init = [(200, 5_725_818), (250, 8_329_311), (300, 11_494_036), (350, 15_205_789), (400, 19_470_470), (450, 24_288_081)];
        let draw = [(200, 9_952_906), (250, 15_200_893), (300, 21_597_805), (350, 29_114_455), (400, 37_812_467), (425, 42_158_851), (450, 47_629_341)];
        for (n, g) in init {
            assert!(init_cost(n) >= g, "init {n}");
        }
        for (n, g) in draw {
            assert!(draw_cost(n) >= g, "draw {n}");
        }
    }

    #[test]
    fn a_panel_fits_or_is_named() {
        assert!(fits(5, false).is_ok());
        assert!(fits(250, false).is_ok());
        assert!(fits(300, false).unwrap_err().contains("最贵一抽"));
        assert!(fits(300, true).is_ok());
        assert!(fits(400, true).unwrap_err().contains("init"));
    }
}
