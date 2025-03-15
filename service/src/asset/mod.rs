pub mod price;

#[derive(Debug, Clone)]
pub enum Chain {
    Svm(u64), // Solana
    Evm(u64), // EVM L1/L2/L3
}

#[derive(Debug, Clone, buildstructor::Builder)]
pub struct Asset {
    pub address: String, 
    pub symbol: String,
    pub chain: Chain,
    pub name: Option<String>,
    pub decimals: u8,
}
