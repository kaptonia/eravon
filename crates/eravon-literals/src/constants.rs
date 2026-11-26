#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot<T: 'static> {
    Fixed(T),
    Undetermined {
        born_in: &'static str,
        why: &'static str,
    },
}

impl<T> Slot<T> {
    pub const fn get(&self) -> Option<&T> {
        match self {
            Slot::Fixed(v) => Some(v),
            Slot::Undetermined { .. } => None,
        }
    }
    pub const fn is_undetermined(&self) -> bool {
        matches!(self, Slot::Undetermined { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rung {
    pub buyer_bps: u16,
    pub seller_bps: u16,
    pub fee_bearer: u8,
}

pub const LADDER_MIN_RUNGS: usize = 2;
pub const LADDER_MAX_RUNGS: usize = 16;
pub const BPS_TOTAL: u16 = 10000;
pub const FEE_BEARER_BUYER: u8 = 0;
pub const FEE_BEARER_SELLER: u8 = 1;
pub const FEE_BEARER_HALF: u8 = 2;

pub const REFUND_LADDER_TEMPLATE: &[Rung] = FULL_LADDER_TEMPLATE;

const FULL_LADDER_TEMPLATE: &[Rung] = &[
    Rung { buyer_bps: 0, seller_bps: BPS_TOTAL, fee_bearer: FEE_BEARER_BUYER },
    Rung { buyer_bps: BPS_TOTAL, seller_bps: 0, fee_bearer: FEE_BEARER_SELLER },
];

pub const fn fee_bearer_of_rung(index: usize) -> u8 {
    if index == 0 { FEE_BEARER_BUYER } else { FEE_BEARER_SELLER }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub symbol: &'static str,
    pub address: &'static str,
    pub decimals: u8,
}

pub const SETTLEMENT_TOKENS_MAINNET: &[Token] = &[
    Token { symbol: "USDT", address: "0xdAC17F958D2ee523a2206206994597C13D831ec7", decimals: 6 },
    Token { symbol: "USDC", address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", decimals: 6 },
    Token { symbol: "USDS", address: "0xdC035D45d973E3EC169d2276DDab16f1e407384F", decimals: 18 },
    Token { symbol: "DAI", address: "0x6B175474E89094C44Da98b954EedeAC495271d0F", decimals: 18 },
    Token { symbol: "PYUSD", address: "0x6c3ea9036406852006290770BEdFcAbA0e23A0e8", decimals: 6 },
    Token { symbol: "FDUSD", address: "0xc5f0f7b66764F6ec8C8Dff7BA683102295E16409", decimals: 18 },
    Token { symbol: "EURC", address: "0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c", decimals: 6 },
    Token { symbol: "USDe", address: "0x4c9EDD5852cd905f086C759E8383e09bff1E68B3", decimals: 18 },
    Token { symbol: "WETH", address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", decimals: 18 },
    Token { symbol: "WBTC", address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", decimals: 8 },
];


pub const SETTLEMENT_TOKENS_SEPOLIA: &[Token] = &[
    Token { symbol: "USDC", address: "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238", decimals: 6 },
    Token { symbol: "EURC", address: "0x08210F9170F89Ab7658F0B5E3fF39b0E03C594D4", decimals: 6 },
    Token { symbol: "USDT", address: "0xaA8E23Fb1079EA71e0a56F48a2aA51851D8433D0", decimals: 6 },
    Token { symbol: "WETH", address: "0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14", decimals: 18 },
    Token { symbol: "RPT", address: "0xc204f101fc22ffe8a4d372b7bb68d23926b0a1d1", decimals: 18 },
];

pub fn settlement_token_addresses() -> Slot<&'static [Token]> {
    Slot::Fixed(default_network().tokens)
}

pub fn settlement_tokens() -> &'static [Token] {
    current_network().tokens
}

pub const LINK_VALIDITY_DAYS: u32 = 90;
pub const RESIGN_LEAD_DAYS: u32 = 7;
pub const CANDIDATE_CAP: usize = 8;
pub const QR_BYTE_BUDGET: usize = 2953;

pub const PUBLIC_DEPLOYER: &str = "0x4e59b44847b379578588920cA78FbF26c0B4956C";

pub const MULTICALL3: Slot<&str> = Slot::Fixed("0xcA11bde05977b3631167028862bE2a173976CA11");

pub const WINDOW_ONE_TIME_KEY_PATH: &str = "m/44'/60'/2'/1/0";

#[derive(Clone, Copy, Debug)]
pub struct StandardTable {
    pub chain_id: u64,
    pub logic: &'static str,
    pub proxy_code_hash: &'static str,
    pub laws: &'static [&'static str],
    pub views: &'static [&'static str],
    pub first_block: u64,
    pub shape: Shape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Current,
    Trial,
}

#[derive(Clone, Copy, Debug)]
pub struct PastIdentity {
    pub logic_initcode_hash: &'static str,
    pub panel_initcode_hash: &'static str,
    pub panel_code_hash: &'static str,
    pub verifier_initcode_hash: &'static str,
    pub verifier: &'static str,
    pub verifier_code_hash: &'static str,
    pub board_initcode_hash: &'static str,
    pub board: &'static str,
    pub board_code_hash: &'static str,
}

pub const TRIAL_IDENTITY: PastIdentity = PastIdentity {
    logic_initcode_hash: "0xce856742d2e7d00a3b4fa13714cdbfbeff4b9109e5c3d0a674efded7bb8d5f16",
    panel_initcode_hash: "0x2ee04925e7c8f7486c1beaee6713000dd1029c16ce83fc91844b3dbe84ec247e",
    panel_code_hash: "0xf8bf651eb20dc84c1919d953880d1992b56757d9f285f411a896f25399703fcd",
    verifier_initcode_hash: "0x8d8f99b377394ce91adede1502e2a8019ec49195bfe271936d87bb4df5ea6630",
    verifier: "0x1447a7B4D7150A3f4362788f9C490207EC713AFE",
    verifier_code_hash: "0x488b4598d7515fefdd5a34791ed9b75c848de5ee78d233c4c4e17a8f73e94cd3",
    board_initcode_hash: "0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7",
    board: "0x62D4040839c4d4b9480f0Fb60028b257acEf6420",
    board_code_hash: "0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353",
};

impl Shape {
    pub fn past_identity(self) -> Option<&'static PastIdentity> {
        match self {
            Shape::Current => None,
            Shape::Trial => Some(&TRIAL_IDENTITY),
        }
    }
}

pub const STANDARD_TABLE: StandardTable = StandardTable {
    chain_id: 1,
    logic: "0x63c2C2a4b21Ad15d68F2A42e746feDB1D8470875",
    proxy_code_hash: "0x9fcd87aa6d3f7b07234d4eba0cd8277506802498b41c377764ef05b3afb5583f",
    laws: &[crate::literals::DOC_TERMS, crate::literals::DOC_LINK, crate::literals::DOC_DOSSIER],
    views: &["feeQuoted()"],
    first_block: MAINNET_FIRST_BLOCK,
    shape: Shape::Current,
};

pub const MAINNET_FIRST_BLOCK: u64 = 26_496_124;

pub const STANDARD_TABLE_SEPOLIA: StandardTable = StandardTable { chain_id: 11_155_111, first_block: SEPOLIA_FIRST_BLOCK, ..STANDARD_TABLE };

pub const SEPOLIA_FIRST_BLOCK: u64 = 12_010_682;

pub const TRIAL_SEPOLIA: StandardTable = StandardTable {
    chain_id: 11_155_111,
    logic: "0x092d6f5B8078441958cD165919041e3AeEA01Bca",
    proxy_code_hash: "0xabf13145f75f09139e0ff2b1ad9f379869ca80fa93451e22ea6dde467938ed47",
    laws: &[crate::literals::DOC_TERMS, crate::literals::DOC_LINK, crate::literals::DOC_DOSSIER],
    views: &["feeQuoted()"],
    first_block: SEPOLIA_FIRST_BLOCK,
    shape: Shape::Trial,
};

#[derive(Clone, Copy, Debug)]
pub struct Network {
    pub name: &'static str,
    pub word: &'static str,
    pub standard: StandardTable,
    pub past: &'static [StandardTable],
    pub factory: &'static [&'static str],
    pub registries: Slot<&'static [Registry]>,
    pub tokens: &'static [Token],
    pub block_secs: u64,
    pub consensus_hashes: bool,
    pub records_first_block: u64,
}

pub const NETWORK_MAINNET: Network = Network {
    name: "mainnet",
    word: "set.chainMain",
    standard: STANDARD_TABLE,
    past: &[],
    factory: &["https://mainnet.gateway.tenderly.co", "https://rpc.mevblocker.io"],
    registries: JUDGE_REGISTRIES,
    tokens: SETTLEMENT_TOKENS_MAINNET,
    block_secs: 12,
    consensus_hashes: true,
    records_first_block: 0,
};

pub const NETWORK_SEPOLIA: Network = Network {
    name: "sepolia",
    word: "set.chainSepolia",
    standard: STANDARD_TABLE_SEPOLIA,
    past: &[TRIAL_SEPOLIA],
    factory: &["https://rpc.sepolia.ethpandaops.io", "https://sepolia.gateway.tenderly.co"],
    registries: Slot::Fixed(JUDGE_REGISTRIES_SEPOLIA),
    tokens: SETTLEMENT_TOKENS_SEPOLIA,
    block_secs: 12,
    consensus_hashes: true,
    records_first_block: SEPOLIA_RECORDS_FIRST_BLOCK,
};

pub const SEPOLIA_RECORDS_FIRST_BLOCK: u64 = 11_998_635;

pub const NETWORKS: [Network; 2] = [NETWORK_MAINNET, NETWORK_SEPOLIA];

pub const fn default_network() -> Network {
    NETWORKS[0]
}

pub const STANDARD_ROWS: [StandardTable; 2] = [NETWORK_MAINNET.standard, NETWORK_SEPOLIA.standard];

pub fn network_of(chain_id: u64) -> Option<Network> {
    NETWORKS.iter().copied().find(|n| n.standard.chain_id == chain_id)
}

pub fn current_network() -> Network {
    let pretended = PRETENDED_CHAIN.with(std::cell::Cell::get);
    let chain = if pretended != 0 { pretended } else { ACTIVE_CHAIN.load(std::sync::atomic::Ordering::SeqCst) };
    network_of(chain).unwrap_or_else(default_network)
}

std::thread_local! {
    static PRETENDED_CHAIN: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub fn pretend_chain(c: Option<u64>) {
    PRETENDED_CHAIN.with(|p| p.set(c.unwrap_or(0)));
}

pub fn past_rows() -> &'static [StandardTable] {
    current_network().past
}

static PAST_SWITCH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_past_switch(on: bool) {
    PAST_SWITCH.store(on, std::sync::atomic::Ordering::SeqCst);
}

pub fn past_switch() -> bool {
    PAST_SWITCH.load(std::sync::atomic::Ordering::SeqCst)
}

#[derive(Clone, Copy, Debug)]
pub enum Generation {
    Current,
    Past(StandardTable),
}

impl Generation {
    pub fn is_past(&self) -> bool {
        matches!(self, Generation::Past(_))
    }

    pub fn row(&self) -> StandardTable {
        match self {
            Generation::Current => active_standard(),
            Generation::Past(r) => *r,
        }
    }

    pub fn shape(&self) -> Shape {
        self.row().shape
    }
}

pub fn generation_among(logic: &str, current: &StandardTable, past: &[StandardTable]) -> Option<Generation> {
    let logic = logic.trim();
    if logic.eq_ignore_ascii_case(current.logic) {
        return Some(Generation::Current);
    }
    past.iter().find(|r| r.chain_id == current.chain_id && logic.eq_ignore_ascii_case(r.logic)).copied().map(Generation::Past)
}

pub fn generation_of_logic(logic: &str) -> Option<Generation> {
    generation_among(logic, &active_standard(), past_rows())
}

pub fn records_floor() -> u64 {
    current_network().records_first_block
}

static ACTIVE_CHAIN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn current_for_chain(current: &[StandardTable], chain_id: u64) -> Option<StandardTable> {
    current.iter().copied().find(|r| r.chain_id == chain_id)
}

pub fn select_chain(chain_id: u64) -> bool {
    match current_for_chain(&STANDARD_ROWS, chain_id) {
        Some(r) => {
            ACTIVE_CHAIN.store(r.chain_id, std::sync::atomic::Ordering::SeqCst);
            true
        }
        None => false,
    }
}

static RUN_CHAIN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn pin_run_chain(chain_id: u64) {
    RUN_CHAIN.store(chain_id, std::sync::atomic::Ordering::SeqCst);
}

pub fn run_chain() -> Option<u64> {
    Some(RUN_CHAIN.load(std::sync::atomic::Ordering::SeqCst)).filter(|c| *c != 0)
}

pub fn active_standard() -> StandardTable {
    current_network().standard
}

pub const LOGIC_INITCODE_HASH: &str =
    "0xc20db87ece6f54bbb9bdea43994aeeb7e8ff48dfaf24852626b16d2529f4aa14";
pub const PANEL_INITCODE_HASH: &str =
    "0x7a8e34943181637c413e9889d06e945dfd2ed5496eac89f445e9f099f401e864";
pub const PANEL: &str = "0x4eD1d231cD27a84517F183f1455188713dbD06d1";
pub const PANEL_CODE_HASH: &str =
    "0x91dc1b8a8092f9a24232897224f2cef9234c75b42e625d3b162092b49f5073d8";

pub const VERIFIER_INITCODE_HASH: &str =
    "0x1b71b7e340dd6e1882635ae2b80b0dd4726e604608169e90a40eb091291b1596";
pub const VERIFIER: &str = "0x86e73507BB79c888C968Fe7eF733688957af18aF";
pub const VERIFIER_CODE_HASH: &str =
    "0x2bbc633e66fd4b81e7660302716aec28dc978145093b7cb3cd2f399ad1c4991d";

pub const BOARD_INITCODE_HASH: &str =
    "0x2fdb06cfcf689779f3148cee2bfb0662266ac99f7b9763e3df145e3408c457d7";
pub const BOARD: &str = "0x62D4040839c4d4b9480f0Fb60028b257acEf6420";
pub const BOARD_CODE_HASH: &str =
    "0xac601fba7fc2b8f38d9116c746dd485944f06d5306c4ed6930321b82b6035353";

pub const SUCCESSION_BOOK: &str = "0x3dB7c505dd25361F1A66f6c5Bd96b48df27f668c";
pub const BOOK_CODE_HASH: &str = "0xdb3293b42187b8b7270e62d5eaafee45af02c9caf3a0c8fe376cb4195f2f6aca";
pub const ROUTE_GAS: u64 = 1_000_000;

pub fn endpoint_table_factory_sample() -> Slot<&'static [&'static str]> {
    Slot::Fixed(default_network().factory)
}

pub fn factory_of(n: &Network) -> &'static [&'static str] {
    if n.name == default_network().name { endpoint_table_factory_sample().get().copied().unwrap_or(&[]) } else { n.factory }
}


pub const NATIVE_DECIMALS: u8 = 18;
pub const NATIVE_SYMBOL: &str = "ETH";

pub const READ_REFRESH_SECS: u64 = 60;
pub const READ_STALE_SECS: u64 = 600;
pub const SCAN_BACKOFF_BLOCKS: u64 = 64;

pub const TIP_HISTORY_BLOCKS: u64 = 20;
pub const TIP_CAP_WEI: u128 = 1_000_000_000;
pub const TIP_FALLBACK_WEI: u128 = 1_000_000_000;

pub const FEE_CAP_FALLBACK_WEI: u128 = 3_000_000_000;

pub const PASSCODE_LEN: usize = 8;
pub const PASSCODE_MAX_ATTEMPTS: u32 = 5;

pub const FORM_CONSTANTS_HOME: &str = "eravon-ui::form::Form";

pub const FORM_CONSTANTS_PRINT: &str = "eravon-app form";

pub const AUTO_LOCK_DEFAULT_ON: bool = true;
pub const AUTO_LOCK_DEFAULT_MINUTES: u32 = 15;
pub const IDLE_MINUTES: [u32; 5] = [1, 5, 15, 30, 60];
pub const ZIKARON_LINK_DEFAULT_ON: bool = true;

pub const JUDGE_REGISTRIES: Slot<&[Registry]> = Slot::Fixed(JUDGE_REGISTRIES_MAINNET);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registry {
    pub address: &'static str,
    pub from_block: u64,
}

pub const JUDGE_REGISTRIES_MAINNET: &[Registry] = &[Registry { address: "0x5da2894fc5260832330305f50f948401b3c904d8", from_block: 26_496_122 }];

pub const JUDGE_REGISTRIES_SEPOLIA: &[Registry] = &[Registry { address: "0x5da2894fc5260832330305f50f948401b3c904d8", from_block: 11_936_067 }];

pub fn judge_registries() -> Slot<&'static [Registry]> {
    current_network().registries
}

pub fn registry_from(addr: &str) -> u64 {
    from_in(judge_registries().get().copied().unwrap_or(&[]), addr)
}

fn from_in(rs: &[Registry], addr: &str) -> u64 {
    rs.iter().find(|r| r.address.eq_ignore_ascii_case(addr)).map_or(0, |r| r.from_block)
}

pub const COMPANY_MODE_DEFAULT_ON: bool = false;

pub const TIME_CHECK_DEFAULT_ON: bool = true;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChelekIdentity {
    pub proxy_code_hash: &'static str,
    pub wallet_code_hash: &'static str,
    pub ledger_code_hash: &'static str,
}

pub const CHELEK_IDENTITY: ChelekIdentity = ChelekIdentity {
    proxy_code_hash: crate::chelek::PROXY_CODE_HASH,
    wallet_code_hash: crate::chelek::WALLET_CODE_HASH,
    ledger_code_hash: crate::chelek::LEDGER_CODE_HASH,
};

pub const CHELEK_SOURCE_COMMIT: &str = crate::chelek::SOURCE_COMMIT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChelekAbi {
    pub wallet: &'static str,
    pub ledger: &'static str,
    pub company_views: &'static str,
}

pub const CHELEK_ABI: ChelekAbi = ChelekAbi {
    wallet: crate::chelek::WALLET_ABI,
    ledger: crate::chelek::LEDGER_ABI,
    company_views: crate::chelek::COMPANY_VIEWS_ABI,
};

pub fn abi_signatures(abi: &str) -> Result<Vec<String>, &'static str> {
    use crate::json::In;
    let v = crate::json::read(abi.as_bytes()).map_err(|_| "E_CHELEK_ABI")?;
    let types = |x: Option<&In>| -> Result<String, &'static str> {
        let mut out = Vec::new();
        for p in x.and_then(In::as_arr).ok_or("E_CHELEK_ABI")? {
            let t = p.get("type").and_then(In::as_str).ok_or("E_CHELEK_ABI")?;
            if t.starts_with("tuple") {
                return Err("E_CHELEK_ABI");
            }
            out.push(t.to_string());
        }
        Ok(out.join(","))
    };
    let mut rows = Vec::new();
    for item in v.as_arr().ok_or("E_CHELEK_ABI")? {
        let kind = item.get("type").and_then(In::as_str).ok_or("E_CHELEK_ABI")?;
        let name = item.get("name").and_then(In::as_str).ok_or("E_CHELEK_ABI")?;
        let ins = types(item.get("inputs"))?;
        let row = match kind {
            "function" => {
                let outs = types(item.get("outputs"))?;
                if outs.is_empty() {
                    format!("function {name}({ins})")
                } else {
                    format!("function {name}({ins}) returns ({outs})")
                }
            }
            "event" | "error" => format!("{kind} {name}({ins})"),
            _ => return Err("E_CHELEK_ABI"),
        };
        rows.push(row);
    }
    Ok(rows)
}

#[derive(Clone, Copy, Debug)]
pub struct UnfilledSlot {
    pub name: &'static str,
    pub born_in: &'static str,
    pub why: &'static str,
}

const fn slot_row<T>(name: &'static str, s: &Slot<T>) -> Option<UnfilledSlot> {
    match s {
        Slot::Fixed(_) => None,
        Slot::Undetermined { born_in, why } => Some(UnfilledSlot { name, born_in, why }),
    }
}

pub fn unfilled_slots() -> Vec<UnfilledSlot> {
    [
        slot_row("SETTLEMENT_TOKEN_ADDRESSES", &settlement_token_addresses()),
        slot_row("MULTICALL3", &MULTICALL3),
        slot_row("ENDPOINT_TABLE_FACTORY_SAMPLE", &endpoint_table_factory_sample()),
        slot_row("JUDGE_REGISTRIES", &JUDGE_REGISTRIES),
    ]
    .into_iter()
    .flatten()
    .collect()
}

pub fn self_check() -> Result<(), &'static str> {
    for abi in [CHELEK_ABI.wallet, CHELEK_ABI.ledger, CHELEK_ABI.company_views] {
        abi_signatures(abi)?;
    }
    if !IDLE_MINUTES.windows(2).all(|w| w[0] < w[1]) || !IDLE_MINUTES.contains(&AUTO_LOCK_DEFAULT_MINUTES) {
        return Err("E_CONSTANTS_INCOMPLETE");
    }
    let tokens = settlement_token_addresses().get().copied().unwrap_or(&[]);
    if tokens.is_empty()
        || tokens.iter().any(|t| t.decimals > crate::units::DECIMALS_MAX)
        || STANDARD_TABLE.laws.len() != 3
        || REFUND_LADDER_TEMPLATE.len() < LADDER_MIN_RUNGS
        || REFUND_LADDER_TEMPLATE.len() > LADDER_MAX_RUNGS
    {
        return Err("E_CONSTANTS_INCOMPLETE");
    }
    let last = REFUND_LADDER_TEMPLATE.len() - 1;
    let mut prev = 0u16;
    for (i, r) in REFUND_LADDER_TEMPLATE.iter().enumerate() {
        if r.buyer_bps > BPS_TOTAL || r.buyer_bps + r.seller_bps != BPS_TOTAL {
            return Err("E_CONSTANTS_LADDER");
        }
        if i == 0 && r.buyer_bps != 0 {
            return Err("E_CONSTANTS_LADDER");
        }
        if i == last && r.buyer_bps != BPS_TOTAL {
            return Err("E_CONSTANTS_LADDER");
        }
        if r.buyer_bps < prev {
            return Err("E_CONSTANTS_LADDER");
        }
        prev = r.buyer_bps;
        if r.fee_bearer != fee_bearer_of_rung(i) {
            return Err("E_CONSTANTS_BEARER");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_pass_their_own_three_questions() {
        assert_eq!(self_check(), Ok(()));
    }

    static NET: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn past_on(n: Network) -> StandardTable {
        StandardTable { logic: "0x00000000000000000000000000000000000F1A57", first_block: 7, ..n.standard }
    }

    #[test]
    fn one_current_row_and_past_rows_told_apart_by_logic() {
        let _held = NET.lock().unwrap_or_else(|e| e.into_inner());
        for n in NETWORKS {
            assert!(n.past.iter().all(|p| !p.logic.eq_ignore_ascii_case(n.standard.logic) && p.shape != Shape::Current), "往代行不许是现行行");
            assert!(n.past.iter().all(|p| p.chain_id == n.standard.chain_id), "往代行同链");
        }
        for n in NETWORKS {
            let cur = n.standard;
            let past = past_on(n);
            assert!(matches!(generation_among(cur.logic, &cur, &[past]), Some(Generation::Current)));
            assert!(matches!(generation_among(&cur.logic.to_ascii_lowercase(), &cur, &[past]), Some(Generation::Current)));
            match generation_among(past.logic, &cur, &[past]) {
                Some(Generation::Past(r)) => assert_eq!(r.logic, past.logic),
                other => panic!("往代行没认出:{other:?}"),
            }
            assert!(generation_among("0x0000000000000000000000000000000000000001", &cur, &[past]).is_none());
            let elsewhere = StandardTable { chain_id: cur.chain_id + 1, ..past };
            assert!(generation_among(past.logic, &cur, &[elsewhere]).is_none(), "别链的往代行不算");
            assert!(generation_among(past.logic, &cur, &[]).is_none());
            assert_eq!(current_for_chain(&STANDARD_ROWS, cur.chain_id).map(|r| r.logic), Some(cur.logic));
            let sep = NETWORK_SEPOLIA.standard;
            assert!(matches!(generation_among(TRIAL_SEPOLIA.logic, &sep, NETWORK_SEPOLIA.past), Some(Generation::Past(r)) if r.shape == Shape::Trial));
            assert!(generation_among(TRIAL_SEPOLIA.logic, &NETWORK_MAINNET.standard, NETWORK_MAINNET.past).is_none());
            assert!(Shape::Trial.past_identity().is_some() && Shape::Current.past_identity().is_none());
            assert!(Generation::Past(past).is_past() && !Generation::Current.is_past());
        }
    }

    #[test]
    fn the_network_table_checks_itself() {
        let mut names = std::collections::BTreeSet::new();
        let mut chains = std::collections::BTreeSet::new();
        let mut nodes = std::collections::BTreeSet::new();
        for n in NETWORKS {
            assert!(names.insert(n.name), "行名 {} 重了", n.name);
            assert!(chains.insert(n.standard.chain_id), "链号 {} 重了", n.standard.chain_id);
            assert!(n.factory.len() >= 2, "{} 至少两家出厂节点", n.name);
            for e in n.factory {
                assert!(e.starts_with("https://"), "{e} 不是 https");
                assert!(nodes.insert(*e), "{e} 在表里出现两次");
            }
            assert!(!n.word.is_empty());
            assert!(n.standard.first_block > 0, "{} 的起扫块不许是 0(未部署即填部署前的安全下界)", n.name);
            if n.name == NETWORK_MAINNET.name {
                assert_eq!(n.records_first_block, 0, "主网照法文自第 0 块读 §6.1 的记录");
            }
            for r in std::iter::once(&n.standard).chain(n.past) {
                assert!(n.records_first_block <= r.first_block, "{} 的记录下界 {} 高过标准行首块 {}:那一行的记录会读漏", n.name, n.records_first_block, r.first_block);
            }
            let (mut syms, mut addrs) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
            assert!(!n.tokens.is_empty(), "{} 没有结算币", n.name);
            for t in n.tokens {
                assert!(syms.insert(t.symbol), "{} 的结算币 {} 重了", n.name, t.symbol);
                assert!(addrs.insert(t.address.to_ascii_lowercase()), "{} 的结算币地址 {} 重了", n.name, t.address);
                assert!(t.address.len() == 42 && t.address.starts_with("0x") && t.address[2..].bytes().all(|c| c.is_ascii_hexdigit()), "{}", t.address);
                assert!(t.decimals <= crate::units::DECIMALS_MAX, "{} 的小数位 {} 出界", t.symbol, t.decimals);
            }
        }
        let default = NETWORKS.iter().filter(|n| n.name == default_network().name).count();
        assert_eq!(default, 1, "默认那一行在表里恰一行");
        assert_eq!(default_network().name, NETWORKS[0].name, "默认那一行是表上头一行");
        assert_eq!(settlement_token_addresses().get().copied().map(<[Token]>::len), Some(default_network().tokens.len()));
        assert_eq!(endpoint_table_factory_sample().get().copied(), Some(default_network().factory));
        assert_eq!(factory_of(&default_network()), default_network().factory);
        assert_eq!(STANDARD_ROWS.len(), NETWORKS.len());
        for (r, n) in STANDARD_ROWS.iter().zip(NETWORKS) {
            assert_eq!(r.chain_id, n.standard.chain_id);
            assert_eq!(current_for_chain(&STANDARD_ROWS, n.standard.chain_id).map(|x| x.chain_id), Some(n.standard.chain_id));
        }
    }

    #[test]
    fn every_network_reader_follows_the_switch() {
        let _held = NET.lock().unwrap_or_else(|e| e.into_inner());
        for n in NETWORKS {
            assert!(select_chain(n.standard.chain_id));
            assert_eq!(current_network().name, n.name);
            assert_eq!(active_standard().chain_id, n.standard.chain_id);
            assert_eq!(judge_registries(), n.registries);
            assert_eq!(settlement_tokens().len(), n.tokens.len());
            assert_eq!(past_rows().len(), n.past.len());
            assert!(!select_chain(31_337), "本地台不改网");
            assert_eq!(current_network().name, n.name);
        }
        assert!(select_chain(default_network().standard.chain_id));
        assert_eq!(current_network().name, default_network().name);
    }

    #[test]
    fn a_home_with_no_choice_is_on_the_head_row_and_every_row_is_read_alike() {
        let _held = NET.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(default_network().name, NETWORK_MAINNET.name);
        ACTIVE_CHAIN.store(0, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(current_network().name, NETWORKS[0].name);
        for n in NETWORKS {
            assert!(select_chain(n.standard.chain_id));
            assert_eq!(current_network().name, n.name, "the home's choice");
        }
        ACTIVE_CHAIN.store(0, std::sync::atomic::Ordering::SeqCst);
    }

    #[test]
    fn four_slots_are_declared_and_every_one_is_filled() {
        assert!(unfilled_slots().is_empty());
        let open: Slot<u8> = Slot::Undetermined { born_in: "a later release", why: "no value is published yet" };
        let row = slot_row("OPEN", &open).expect("an undetermined slot is listed");
        assert_eq!((row.name, row.born_in, row.why), ("OPEN", "a later release", "no value is published yet"));
    }

    #[test]
    fn settlement_tokens_are_the_ten_the_judge_fixed() {
        let want = [("USDT", 6), ("USDC", 6), ("USDS", 18), ("DAI", 18), ("PYUSD", 6), ("FDUSD", 18), ("EURC", 6), ("USDe", 18), ("WETH", 18), ("WBTC", 8)];
        let rows = SETTLEMENT_TOKENS_MAINNET;
        assert_eq!(rows.len(), 10);
        for (i, t) in rows.iter().enumerate() {
            assert_eq!((t.symbol, t.decimals), want[i]);
            assert!(t.address.len() == 42 && t.address.starts_with("0x") && t.address[2..].bytes().all(|c| c.is_ascii_hexdigit()));
        }
        assert!(rows.iter().all(|t| t.symbol != "FRAX" && t.symbol != "GHO"));
        assert_eq!(rows[0].address, "0xdAC17F958D2ee523a2206206994597C13D831ec7");
        assert_eq!(rows[9].address, "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599");
        let sepolia: Vec<(&str, u8)> = SETTLEMENT_TOKENS_SEPOLIA.iter().map(|t| (t.symbol, t.decimals)).collect();
        assert_eq!(sepolia, [("USDC", 6), ("EURC", 6), ("USDT", 6), ("WETH", 18), ("RPT", 18)]);
    }

    #[test]
    fn auto_lock_and_zikaron_factory_values_are_the_volumes() {
        assert!(AUTO_LOCK_DEFAULT_ON);
        assert_eq!(AUTO_LOCK_DEFAULT_MINUTES, 15);
        assert_eq!(IDLE_MINUTES, [1, 5, 15, 30, 60]);
        assert!(ZIKARON_LINK_DEFAULT_ON);
    }

    #[test]
    fn the_refresh_period_is_born_a_minute() {
        assert_eq!(READ_REFRESH_SECS, 60);
    }

    #[test]
    fn company_mode_ships_off_and_time_check_ships_on() {
        assert!(!COMPANY_MODE_DEFAULT_ON);
        assert!(TIME_CHECK_DEFAULT_ON);
    }

    #[test]
    fn the_wallet_pays_and_the_four_views_read() {
        let wallet = abi_signatures(CHELEK_ABI.wallet).expect("钱包 ABI 读得成");
        assert!(wallet.iter().any(|r| r == "function pay(address,address,uint256)"));
        let ledger = abi_signatures(CHELEK_ABI.ledger).expect("账本 ABI 读得成");
        assert!(ledger.iter().any(|r| r == "function company() returns (address)"));
        assert!(!ledger.iter().any(|r| r.starts_with("function sweep(")));
        let views = abi_signatures(CHELEK_ABI.company_views).expect("四视图片段读得成");
        assert_eq!(
            views,
            [
                "function calendarBound() returns (uint256)",
                "function wallet() returns (address)",
                "function ledger() returns (address)",
                "function holds(address) returns (bool)"
            ]
        );
        for (row, lit) in views.iter().zip(crate::literals::COMPANY_VIEWS) {
            assert!(row.starts_with(&format!("function {lit}")));
        }
    }

    #[test]
    fn a_registry_scan_starts_at_its_declared_deployment_block() {
        let sepolia = JUDGE_REGISTRIES_SEPOLIA[0];
        assert_eq!(from_in(JUDGE_REGISTRIES_SEPOLIA, &sepolia.address.to_ascii_uppercase().replacen("0X", "0x", 1)), 11_936_067);
        assert_eq!(from_in(JUDGE_REGISTRIES_SEPOLIA, "0x118ace67048091c8a811404a00a97ed67c3a402d"), 0, "旧址不再登记");
        assert_eq!(from_in(&[], sepolia.address), 0);
        let mainnet = JUDGE_REGISTRIES_MAINNET[0];
        assert_eq!(mainnet.address, sepolia.address, "the registry stands at one address on both networks");
        assert_eq!(from_in(JUDGE_REGISTRIES_MAINNET, mainnet.address), 26_496_122);
        assert_eq!(JUDGE_REGISTRIES.get().copied(), Some(JUDGE_REGISTRIES_MAINNET));
    }

    #[test]
    fn the_chelek_identity_is_three_distinct_hashes() {
        let id = CHELEK_IDENTITY;
        let all = [id.proxy_code_hash, id.wallet_code_hash, id.ledger_code_hash];
        for h in all {
            assert!(h.len() == 66 && h.starts_with("0x") && h[2..].bytes().all(|b| b.is_ascii_hexdigit()));
        }
        assert!(all[0] != all[1] && all[1] != all[2] && all[0] != all[2]);
    }

    #[test]
    fn an_undetermined_slot_hands_out_no_value() {
        let open: Slot<&[Registry]> = Slot::Undetermined { born_in: "a later release", why: "no value is published yet" };
        assert!(open.get().is_none());
        assert!(open.is_undetermined());
        assert!(!JUDGE_REGISTRIES.is_undetermined());
    }

    #[test]
    fn the_filled_slot_hands_out_the_judges_address() {
        assert!(!MULTICALL3.is_undetermined());
        assert_eq!(MULTICALL3.get(), Some(&"0xcA11bde05977b3631167028862bE2a173976CA11"));
    }
}
