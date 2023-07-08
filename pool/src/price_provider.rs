use price_feed_interface::PriceFeedClient;
use soroban_sdk::{Address, Env};

#[allow(dead_code)]
pub struct PriceProvider<'a> {
    feed: PriceFeedClient<'a>,
}

pub struct AssetPrice {
    pub price: i128,
    pub decimals: u32,
}

#[allow(dead_code)]
impl PriceProvider<'_> {
    pub fn new(env: &Env, feed_address: Address) -> Self {
        let feed = price_feed_interface::PriceFeedClient::new(env, &feed_address);
        Self { feed }
    }

    pub fn get_price(&self, asset: Address) -> Option<AssetPrice> {
        let last_price = self.feed.lastprice(&asset);
        let decimals = self.feed.decimals();

        Some(AssetPrice {
            price: last_price?.price,
            decimals,
        })
    }
}
