
#[derive(Clone)]
pub struct MarketDb {
    pub db: mongodb::Database,
}

#[derive(Clone)]
pub struct BrokerDb {
    pub db: mongodb::Database,
}