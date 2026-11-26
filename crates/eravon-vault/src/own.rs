use crate::deploy::Deployment;
use crate::lang::Key;

pub const DEPLOYMENTS: [Deployment; 4] = [
    Deployment {
        name: "mainnet",
        label: Key::DeployMainnet,
        chain_id: 1,
        registry: "0x36Ea8A857a5FE813429d4D9947000C644A88809A",
        from_block: 26_087_229,
        nodes: ["https://mainnet.gateway.tenderly.co", "https://rpc.mevblocker.io"],
        own: (),
    },
    Deployment {
        name: "sepolia",
        label: Key::SetChainSepolia,
        chain_id: 11_155_111,
        registry: "0xC29410B882c4C3b77e33659d2f06ac563e7B08a3",
        from_block: 11_715_660,
        nodes: ["https://sepolia.gateway.tenderly.co", "https://rpc.sepolia.ethpandaops.io"],
        own: (),
    },
    Deployment {
        name: "arbitrum-one",
        label: Key::ChainArbitrumOne,
        chain_id: 42_161,
        registry: "0x36Ea8A857a5FE813429d4D9947000C644A88809A",
        from_block: 511_445_184,
        nodes: ["https://arb1.arbitrum.io/rpc", "https://arbitrum.gateway.tenderly.co"],
        own: (),
    },
    Deployment {
        name: "op-mainnet",
        label: Key::ChainOpMainnet,
        chain_id: 10,
        registry: "0x36Ea8A857a5FE813429d4D9947000C644A88809A",
        from_block: 157_735_914,
        nodes: ["https://mainnet.optimism.io", "https://optimism.gateway.tenderly.co"],
        own: (),
    },
];

pub type DeployOwn = ();

pub const DEPLOY_DEFAULT: &str = "mainnet";

pub const DEPLOY_CODE_HASH: &str = "0xfa97a1d9b22fab2b52f4e27c9a965b32734c40001b565ab365d05c887118f57d";

#[cfg(test)]
mod tests {
    #[test]
    fn every_network_has_its_deployment_row_on_its_chain() {
        for n in eravon_literals::constants::NETWORKS {
            let row = super::DEPLOYMENTS.iter().find(|d| d.name == n.name).unwrap_or_else(|| panic!("{}: no deployment row", n.name));
            assert_eq!(row.chain_id, n.standard.chain_id, "{}", n.name);
        }
        assert!(eravon_literals::constants::NETWORKS.iter().any(|n| n.name == super::DEPLOY_DEFAULT), "the default row is a network's");
    }
}
