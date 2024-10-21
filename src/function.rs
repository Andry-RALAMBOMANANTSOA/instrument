use shared_structs::*;
use rand::Rng;
use chrono::{DateTime, Utc,SecondsFormat};
use std::fs::File;
use std::io::BufReader;
use serde_json::from_reader;
use serde::de::DeserializeOwned;
use rmp_serde::{Deserializer, Serializer};
use dashmap::DashMap;
use std::sync::mpsc;
use mongodb::{bson::doc, Database,bson,bson::Document, error::Result as MongoResult};
use std::fmt::Debug;
use serde::{Deserialize, Serialize};
use crate::shared::*;
use std::error::Error;
//use std::error::Error;
use std::collections::BTreeMap;
use serde_json::to_string;
use std::io::Write;
use futures::stream::StreamExt;

use hex;
use hmac::{Hmac, Mac};

pub fn read_marketconf(file_path: &str) -> Result<MarketConf, String> {
    let file = File::open(file_path).map_err(|e| format!("Error opening file: {}", e))?;
    let reader = BufReader::new(file);
    let market_config: MarketConf = serde_json::from_reader(reader).map_err(|e| format!("Error reading JSON: {}", e))?;
    Ok(market_config)
}

pub fn read_market_config(file_path: &str) -> Result<MarketConfig, String> {
    let file = File::open(file_path).map_err(|e| format!("Error opening file: {}", e))?;
    let reader = BufReader::new(file);
    let market_config: MarketConfig = serde_json::from_reader(reader).map_err(|e| format!("Error reading JSON: {}", e))?;
    Ok(market_config)
}

pub fn read_broker_config(path: &str) -> Result<BrokerConfigDedicaced, String> {
    let file = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    let reader = BufReader::new(file);
    let broker_config: BrokerConfigDedicaced = serde_json::from_reader(reader).map_err(|e| format!("Error reading JSON: {}", e))?;
    Ok(broker_config)
}

pub fn read_market_spec_config(file_path: &str) -> Result<MarketSpecConfig, String> {
    let file = File::open(file_path).map_err(|e| format!("Error opening file: {}", e))?;
    let reader = BufReader::new(file);
    let market_spec_config: MarketSpecConfig = serde_json::from_reader(reader)
        .map_err(|e| format!("Error reading JSON: {}", e))?;
    Ok(market_spec_config)
}

pub fn ms_res(message: &str, success: bool) -> MSResponse {
    MSResponse {
        message: message.to_string(),
        success,
    }
}

pub fn verify_hmac(serialized_body: &Vec<u8>, received_hmac: &str, secret_key_name: &str) -> Result<bool, Box<dyn Error>> {
    // Get the secret key from the environment variable
    let secret_key = std::env::var(secret_key_name)?;
    let secret_key_bytes = secret_key.as_bytes();

    let mut mac = <HmacSha256 as hmac::Mac>::new_from_slice(secret_key_bytes)
    .map_err(|_| "Invalid key length")?;
    mac.update(&serialized_body);
    let result = mac.finalize();
    let expected_hmac = hex::encode(result.into_bytes());

    // Compare the HMAC sent by the client with the one we generate
    Ok(expected_hmac == received_hmac)
}
pub fn verify_hmac_json(json_body: &str, received_hmac: &str, secret_key_name: &str) -> Result<bool, Box<dyn Error>> {
    // Get the secret key from the environment variable
    let secret_key = std::env::var(secret_key_name)?;
    let secret_key_bytes = secret_key.as_bytes();

    let mut mac = HmacSha256::new_from_slice(secret_key_bytes)
        .map_err(|_| "Invalid key length")?;
    mac.update(json_body.as_bytes());
    let result = mac.finalize();
    let expected_hmac = hex::encode(result.into_bytes());

    // Compare the HMAC sent by the client with the one we generate
    Ok(expected_hmac == received_hmac)
}

pub fn validate_limit_order(order: &LimitOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }
    if let Some(order_id) = order.order_identifier {
        if order_id == 0 {
            return Err("Order identifier cannot be zero".into());
        }
    }
    if order.order_quantity == 0 {
        return Err("Order quantity cannot be zero".into());
    }
    if order.price == 0 {
        return Err("Price cannot be zero".into());
    }

    // Check if order_side is valid (not Unspecified)
    if order.order_side == OrderSide::Unspecified {
        return Err("Invalid order side".into());
    }

    // Check if expiration is valid (not Unspecified)
    if order.expiration == OrderExpiration::Unspecified {
        return Err("Invalid order expiration".into());
    }

    Ok(())
}

pub fn validate_market_order(order: &MarketOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }
    if let Some(order_id) = order.order_identifier {
        if order_id == 0 {
            return Err("Order identifier cannot be zero".into());
        }
    }
    if order.order_quantity == 0 {
        return Err("Order quantity cannot be zero".into());
    }
   

    // Check if order_side is valid (not Unspecified)
    if order.order_side == OrderSide::Unspecified {
        return Err("Invalid order side".into());
    }

    // Check if expiration is valid (not Unspecified)
    if order.expiration == OrderExpiration::Unspecified {
        return Err("Invalid order expiration".into());
    }

    Ok(())
}

pub fn validate_stop_order(order: &StopOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }
  
    if order.order_quantity == 0 {
        return Err("Order quantity cannot be zero".into());
    }
   
    if order.trigger_price == 0 {
        return Err("Price cannot be zero".into());
    }
    // Check if order_side is valid (not Unspecified)
    if order.order_side == OrderSide::Unspecified {
        return Err("Invalid order side".into());
    }

    // Check if expiration is valid (not Unspecified)
    if order.expiration == OrderExpiration::Unspecified {
        return Err("Invalid order expiration".into());
    }

    Ok(())
}

pub fn validate_stoplimit_order(order: &StopLimitOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }
  
    if order.order_quantity == 0 {
        return Err("Order quantity cannot be zero".into());
    }
   
    if order.trigger_price == 0 {
        return Err("Price cannot be zero".into());
    }
    if order.price == 0 {
        return Err("Price cannot be zero".into());
    }
    // Check if order_side is valid (not Unspecified)
    if order.order_side == OrderSide::Unspecified {
        return Err("Invalid order side".into());
    }

    // Check if expiration is valid (not Unspecified)
    if order.expiration == OrderExpiration::Unspecified {
        return Err("Invalid order expiration".into());
    }

    Ok(())
}
pub fn validate_iceberg_order(order: &IcebergOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
    
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }
  
    if order.total_quantity == 0 {
        return Err("Total quantity cannot be zero".into());
    }
    
    if order.visible_quantity == 0 {
        return Err("Visible quantity cannot be zero".into());
    }
    if order.visible_quantity >= order.total_quantity {
        return Err("Visible quantity cannot be superior or equal to the total quantity".into());
    }
    if order.visible_quantity > (order.total_quantity / 4) {
        return Err("Visible quantity cannot exceed 25% of the total quantity".into());
    }
   
    if order.price == 0 {
        return Err("Price cannot be zero".into());
    }
    if order.price == 0 {
        return Err("Price cannot be zero".into());
    }
    // Check if order_side is valid (not Unspecified)
    if order.order_side == OrderSide::Unspecified {
        return Err("Invalid order side".into());
    }

    // Check if expiration is valid (not Unspecified)
    if order.expiration == OrderExpiration::Unspecified {
        return Err("Invalid order expiration".into());
    }

    Ok(())
}

pub fn validate_modify_order(order: &ModifyOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }

    if order.order_identifier == 0 {
        return Err("Price cannot be zero".into());
    }
  
    if order.new_quantity == 0 {
        return Err("Order quantity cannot be zero".into());
    }
   
    Ok(())
}
pub fn validate_modify_iceberg_order(order: &ModifyIcebergOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
   
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }
  
    if order.new_quantity == 0 {
        return Err("Total quantity cannot be zero".into());
    }
    
    if order.new_visible_quantity == 0 {
        return Err("Visible quantity cannot be zero".into());
    }
    if order.new_visible_quantity > order.new_quantity {
        return Err("Visible quantity cannot be superior than total quantity".into());
    }
    if order.new_visible_quantity > (order.new_quantity / 4) {
        return Err("Visible quantity cannot exceed 25% of the total quantity".into());
    }
   
    if order.iceberg_identifier == 0 {
        return Err("Price cannot be zero".into());
    }

    Ok(())
}

pub fn validate_delete_order(order: &DeleteOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }

    if order.order_identifier == 0 {
        return Err("Price cannot be zero".into());
    }
  
    Ok(())
}
pub fn validate_delete_iceberg_order(order: &DeleteIcebergOrder) -> Result<(), String> {
    // Check if numeric fields are non-zero
    if order.trader_identifier == 0 {
        return Err("Trader identifier cannot be zero".into());
    }

    if order.iceberg_identifier == 0 {
        return Err("Price cannot be zero".into());
    }
  
    Ok(())
}
pub fn validate_limit_specs(limit_order: &LimitOrder, market_spec: &MarketSpec) -> Result<(), String> {
    // Check if the price is a valid step of tick_size
    if limit_order.price % market_spec.tick_size  != 0 {
        return Err(format!("Price must be a multiple of {}", market_spec.tick_size));
    }

    // Check if order quantity is within the allowed range
    if limit_order.order_quantity < market_spec.min_contract {
        return Err(format!(
            "Order quantity cannot be less than {}",
            market_spec.min_contract
        ));
    }

    if limit_order.order_quantity > market_spec.max_contract {
        return Err(format!(
            "Order quantity cannot be greater than {}",
            market_spec.max_contract
        ));
    }

    Ok(())
}
pub fn validate_iceberg_specs(iceberg_order: &IcebergOrder, market_spec: &MarketSpec) -> Result<(), String> {
    // Check if the price is a valid step of tick_size
    if iceberg_order.price % market_spec.tick_size  != 0 {
        return Err(format!("Price must be a multiple of {}", market_spec.tick_size));
    }

    // Check if order quantity is within the allowed range
    if iceberg_order.total_quantity < market_spec.min_contract {
        return Err(format!(
            "Iceberg quantity cannot be less than {} contracts",
            market_spec.min_contract
        ));
    }

    if iceberg_order.total_quantity > market_spec.max_contract {
        return Err(format!(
            "Iceberg quantity cannot be greater than {} contracts",
            market_spec.max_contract
        ));
    }
    if iceberg_order.total_quantity < market_spec.iceberg_min_contract {
        return Err(format!("Total iceberg quantity cannot be inferior than {}",market_spec.iceberg_min_contract));
    }

    Ok(())
}
pub fn validate_market_specs(market_order: &MarketOrder, market_spec: &MarketSpec) -> Result<(), String> {
   
    // Check if order quantity is within the allowed range
    if market_order.order_quantity < market_spec.min_contract {
        return Err(format!(
            "Order quantity cannot be less than {}",
            market_spec.min_contract
        ));
    }

    if market_order.order_quantity > market_spec.max_contract {
        return Err(format!(
            "Order quantity cannot be greater than {}",
            market_spec.max_contract
        ));
    }

    Ok(())
}

pub fn validate_stop_specs(stop_order: &StopOrder, market_spec: &MarketSpec) -> Result<(), String> {
    // Check if the price is a valid step of tick_size
    if stop_order.trigger_price % market_spec.tick_size  != 0 {
        return Err(format!("Price must be a multiple of {}", market_spec.tick_size));
    }

    // Check if order quantity is within the allowed range
    if stop_order.order_quantity < market_spec.min_contract {
        return Err(format!(
            "Order quantity cannot be less than {}",
            market_spec.min_contract
        ));
    }

    if stop_order.order_quantity > market_spec.max_contract {
        return Err(format!(
            "Order quantity cannot be greater than {}",
            market_spec.max_contract
        ));
    }

    Ok(())
}

pub fn validate_stoplimit_specs(stoplimit_order: &StopLimitOrder, market_spec: &MarketSpec) -> Result<(), String> {
    // Check if the price is a valid step of tick_size
    if stoplimit_order.trigger_price % market_spec.tick_size  != 0 {
        return Err(format!("Price must be a multiple of {}", market_spec.tick_size));
    }
    if stoplimit_order.price % market_spec.tick_size  != 0 {
        return Err(format!("Price must be a multiple of {}", market_spec.tick_size));
    }

    // Check if order quantity is within the allowed range
    if stoplimit_order.order_quantity < market_spec.min_contract {
        return Err(format!(
            "Order quantity cannot be less than {}",
            market_spec.min_contract
        ));
    }

    if stoplimit_order.order_quantity > market_spec.max_contract {
        return Err(format!(
            "Order quantity cannot be greater than {}",
            market_spec.max_contract
        ));
    }

    Ok(())
}

pub fn validate_modify_specs(modify_order: &ModifyOrder, market_spec: &MarketSpec) -> Result<(), String> {
    // Check if the price is a valid step of tick_size
    
    // Check if order quantity is within the allowed range
    if modify_order.new_quantity < market_spec.min_contract {
        return Err(format!(
            "Order quantity cannot be less than {}",
            market_spec.min_contract
        ));
    }

    if modify_order.new_quantity > market_spec.max_contract {
        return Err(format!(
            "Order quantity cannot be greater than {}",
            market_spec.max_contract
        ));
    }
    

    Ok(())
}
pub fn validate_modify_iceberg_specs(modify_iceberg_order: &ModifyIcebergOrder, market_spec: &MarketSpec) -> Result<(), String> {
    // Check if the price is a valid step of tick_size
    
    // Check if order quantity is within the allowed range
    if modify_iceberg_order.new_quantity < market_spec.min_contract {
        return Err(format!(
            "Order quantity cannot be less than {}",
            market_spec.min_contract
        ));
    }

    if modify_iceberg_order.new_quantity > market_spec.max_contract {
        return Err(format!(
            "Order quantity cannot be greater than {}",
            market_spec.max_contract
        ));
    }

    if modify_iceberg_order.new_quantity < market_spec.iceberg_min_contract {
        return Err(format!(
            "Order quantity cannot be less than {}",
            market_spec.iceberg_min_contract
        ));
    }

    Ok(())
}

pub fn id_i64() -> i64 {
    let mut rng = rand::thread_rng();
    let id = rng.gen::<i64>();
    if id == i64::MIN { i64::MAX } else if id < 0 { -id } else { id }
}

pub fn lowest_ask(ask_mbp: &MBPData) -> Option<i32> {
    if let Some((lowest_price,_)) = ask_mbp.mbp.iter().next() {
        Some(*lowest_price)
    } else {
        None
    }
}

pub fn highest_bid(bid_mbp: &MBPData) -> Option<i32> {
    if let Some((highest_price, _)) = bid_mbp.mbp.iter().next_back() {
        Some(*highest_price)
    } else {
        None
    }
}
pub fn lowest_ask_quant(ask_mbp: &MBPData) -> Option<i32> {
    if let Some((_,askquantity)) = ask_mbp.mbp.iter().next() {
        Some(*askquantity)
    } else {
        None
    }
}

pub fn highest_bid_quant(bid_mbp: &MBPData) -> Option<i32> {
    if let Some((_, bidquantity)) = bid_mbp.mbp.iter().next_back() {
        Some(*bidquantity)
    } else {
        None
    }
}

pub fn match_struct_limit(arrival: &LimitOrder, trader_order: &TraderOrderStruct, match_identifier: i64,taker_orderid:i64,time:i64,quantity:i32) -> MatchStruct {

    MatchStruct {
        market: arrival.market.clone(),
        broker_identifier_taker: arrival.broker_identifier.clone(),
        broker_identifier_maker: trader_order.broker_identifier.clone(),
        unix_time:time,
        match_identifier,
        trader_identifier_taker: arrival.trader_identifier,
        order_identifier_taker: taker_orderid,
        trader_identifier_maker: trader_order.trader_identifier,
        order_identifier_maker: trader_order.order_identifier,
        maker_position_side:trader_order.order_side.clone(),
        taker_position_side:arrival.order_side.clone(),
        taker_type:"limit".to_string(),
        expiration_taker: arrival.expiration.clone(),
        expiration_maker: trader_order.expiration.clone(),
        order_quantity: quantity,
        order_side: arrival.order_side.clone(),
        price: trader_order.price,
        pointing_at_maker:trader_order.pointing_at,
        pointing_at_taker:arrival.pointing_at,
    }
}
pub fn match_struct_market(arrival: &MarketOrder, trader_order: &TraderOrderStruct, match_identifier: i64,taker_orderid:i64,time:i64,quantity:i32) -> MatchStruct {

    MatchStruct {
        market: arrival.market.clone(),
        broker_identifier_taker: arrival.broker_identifier.clone(),
        broker_identifier_maker: trader_order.broker_identifier.clone(),
        unix_time:time,
        match_identifier,
        trader_identifier_taker: arrival.trader_identifier,
        order_identifier_taker: taker_orderid,
        trader_identifier_maker: trader_order.trader_identifier,
        order_identifier_maker: trader_order.order_identifier,
        maker_position_side:trader_order.order_side.clone(),
        taker_position_side:arrival.order_side.clone(),
        taker_type:"market".to_string(),
        expiration_taker: arrival.expiration.clone(),
        expiration_maker: trader_order.expiration.clone(),
        order_quantity: quantity,
        order_side: arrival.order_side.clone(),
        price: trader_order.price,
        pointing_at_maker:trader_order.pointing_at,
        pointing_at_taker:arrival.pointing_at,
    }
}

pub fn trader_info(position: &ClosePositionStruct, config: &MarketConf,tx_broker: &mpsc::Sender<Structs>)  {
    let mut trader_calcbalance: i32 = 0;
    let tick_size = config.tick_size; // Tick size from the market configuration
    let tick_value = config.tick_value;

    match position.position_side {
        OrderSide::Long => {
            // For long positions, profit is made when closing_price > initial_price
            let price_diff = position.closing_price  - position.initial_price;
            let tick_diff = price_diff / tick_size;
            trader_calcbalance = tick_diff * tick_value * position.position_quantity;
        }
        OrderSide::Short => {
            // For short positions, profit is made when closing_price < initial_price
            let price_diff = position.initial_price  - position.closing_price;
            let tick_diff = price_diff / tick_size;
            trader_calcbalance = tick_diff * tick_value * position.position_quantity;
        }
        OrderSide::Unspecified => {
            // No action for unspecified order side
        }
    }

    // Create PostTraderInf struct with the calculated balance
    let info = PostTraderInf {
        unix_time: position.unix_time,
        market: config.market_name.clone(),
        broker_identifier: position.broker_identifier.clone(),
        trader_identifier: position.trader_identifier,
        position_identifier: position.position_identifier,
        position_identifier_closer: position.position_identifier_closer,
        position_quantity:position.position_quantity,
        trader_calcbalance, // Positive if profit, negative if loss
    };

    let info_message = Structs::PostTraderInf(info);
    if let Err(e) = tx_broker.send(info_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}

pub fn time_sale(config: &MarketConf, unix_time: i64, order_quantity: i32, order_side:OrderSide, price: i32) -> TimeSale {
    TimeSale {
        market: config.market_name.clone(),
        exchange: config.exchange.clone(),
        unix_time,
        order_quantity,
        order_side: order_side.clone(),
        price,
    }
}

pub fn tp_last(unix_time:i64,price: i32,config:&MarketConf) -> Last {
    Last { unix_time,market:config.market_name.clone(),price }
}

pub fn struct_bbo(unix_time: i64, ask_mbp: &MBPData, bid_mbp: &MBPData,config:&MarketConf) -> BBO {
    let ask_price = lowest_ask(ask_mbp);
    let bid_price = highest_bid(bid_mbp);
    let ask_size = lowest_ask_quant(ask_mbp); 
    let bid_size = highest_bid_quant(bid_mbp); 

    BBO {
        unix_time,
        market:config.market_name.clone(),
        ask_price,
        bid_price,
        ask_size,
        bid_size,
    }
}
pub fn full_ob(unix_time: i64, ask_mbp: &MBPData, bid_mbp: &MBPData,config:&MarketConf,tx_market: &mpsc::Sender<Structs>) {
    let bid_levels: Vec<PriceLevel> = bid_mbp
    .mbp
    .iter()
    .map(|(price, quantity)| PriceLevel {
        price: *price,
        quantity: *quantity,
    })
    .collect();

let ask_levels: Vec<PriceLevel> = ask_mbp
    .mbp
    .iter()
    .map(|(price, quantity)| PriceLevel {
        price: *price,
        quantity: *quantity,
    })
    .collect();

let event = FullOB {
    unix_time,
    market: config.market_name.clone(),
    bid: bid_levels,
    ask: ask_levels,
};
let event_message = Structs::FullOB(event);
if let Err(e) = tx_market.send(event_message) {
    eprintln!("Failed to send fullob via channel: {:?}", e);
}
}
pub fn full_interest(unix_time: i64, long: &InterestTree, short: &InterestTree,config:&MarketConf,tx_market: &mpsc::Sender<Structs>) {
    let long_levels = long.to_vec();
    let short_levels = short.to_vec();
    let event = FullInterest {
        unix_time,
        market:config.market_name.clone(),
        long: long_levels,
        short: short_levels,
    };
    let event_message = Structs::FullInterest(event);
    if let Err(e) = tx_market.send(event_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn struct_ob( ask_mbp: &MBPData, bid_mbp: &MBPData,config:&MarketConf,depth:usize) -> OB {

    let ask: BTreeMap<i32, i32> = ask_mbp
    .mbp
    .iter()
    .take(depth) // Take the first `depth` asks
    .map(|(&price, &quantity)| (price, quantity))
    .collect();

// Get the top `depth` highest bids
let bid: BTreeMap<i32, i32> = bid_mbp
    .mbp
    .iter()
    .rev() // Reverse iterator for highest bids
    .take(depth) // Take the first `depth` bids
    .map(|(&price, &quantity)| (price, quantity))
    .collect();

    OB {
        market:config.market_name.clone(),
        bid,
        ask,
    }
}

pub fn volume_struct(time:i64,quantity:i32,side:&OrderSide,price:i32,config:&MarketConf)-> Volume {
    let (buy_volume, sell_volume) = match side {
        OrderSide::Long => (quantity, 0),
        OrderSide::Short => (0, quantity),
        _ => panic!("Invalid side: {}", side),
    };
    
    Volume {
        unix_time: time,
        market: config.market_name.clone(),
        buy_volume,
        sell_volume,
        price,
    }
}

pub fn modify_struct (config: &MarketConf,trader_order_struct: &TraderOrderStruct,unix_time: i64,old_quant:i32,new_quant:i32 ) -> ModifiedOrderStruct {

    ModifiedOrderStruct {
    market: config.market_name.clone(),
    broker_identifier: trader_order_struct.broker_identifier.clone(),
    unix_time,
    trader_identifier: trader_order_struct.trader_identifier,
    order_identifier: trader_order_struct.order_identifier,
    older_order_quantity: old_quant,
    new_order_quantity: new_quant,
    order_side: trader_order_struct.order_side.clone(),
    expiration:trader_order_struct.expiration.clone(),
    price: trader_order_struct.price,
    pointing_at:trader_order_struct.pointing_at,
    }
}
pub fn modify_iceberg_struct (config: &MarketConf,iceberg_order_struct: &IcebergOrderStruct,unix_time: i64,new_quant:i32,new_v_quant:i32,new_r_quant:i32 ) -> ModifiedIcebergStruct {

    ModifiedIcebergStruct {
    market: config.market_name.clone(),
    broker_identifier: iceberg_order_struct.broker_identifier.clone(),
    unix_time,
    trader_identifier: iceberg_order_struct.trader_identifier,
    iceberg_identifier: iceberg_order_struct.iceberg_identifier,
    older_quantity: iceberg_order_struct.total_quantity,
    new_quantity: new_quant,
    older_visible_quantity: iceberg_order_struct.visible_quantity,
    new_visible_quantity: new_v_quant,
    older_resting_quantity:iceberg_order_struct.resting_quantity,
    new_resting_quantity:new_r_quant,
    order_side: iceberg_order_struct.order_side.clone(),
    expiration:iceberg_order_struct.expiration.clone(),
    price: iceberg_order_struct.price,
    }
}

pub fn modify_stop_struct (config: &MarketConf,trader_stop_order_struct: &TraderStopOrderStruct,unix_time: i64,old_quant:i32,new_quant:i32 ) -> ModifiedStopOrderStruct {

    ModifiedStopOrderStruct {
    market: config.market_name.clone(),
    broker_identifier: trader_stop_order_struct.broker_identifier.clone(),
    unix_time,
    trader_identifier: trader_stop_order_struct.trader_identifier,
    order_identifier: trader_stop_order_struct.order_identifier,
    older_order_quantity: old_quant,
    new_order_quantity: new_quant,
    order_side: trader_stop_order_struct.order_side.clone(),
    expiration:trader_stop_order_struct.expiration.clone(),
    trigger_price: trader_stop_order_struct.trigger_price,
    pointing_at:trader_stop_order_struct.pointing_at,
    }
}

pub fn modify_stop_limit_struct (config: &MarketConf,trader_stop_limit_order_struct: &TraderStopLimitOrderStruct,unix_time: i64,old_quant:i32,new_quant:i32 ) -> ModifiedStopLimitOrderStruct {

    ModifiedStopLimitOrderStruct {
    market: config.market_name.clone(),
    broker_identifier: trader_stop_limit_order_struct.broker_identifier.clone(),
    unix_time,
    trader_identifier: trader_stop_limit_order_struct.trader_identifier,
    order_identifier: trader_stop_limit_order_struct.order_identifier,
    older_order_quantity: old_quant,
    new_order_quantity: new_quant,
    order_side: trader_stop_limit_order_struct.order_side.clone(),
    expiration:trader_stop_limit_order_struct.expiration.clone(),
    trigger_price: trader_stop_limit_order_struct.trigger_price,
    price: trader_stop_limit_order_struct.price,
    pointing_at:trader_stop_limit_order_struct.pointing_at,
    
    }
}
pub fn delete_struct (trader_order_struct: &TraderOrderStruct,unix_time: i64)-> DeletedOrderStruct {

    DeletedOrderStruct {
     market: trader_order_struct.market.clone(),
    broker_identifier: trader_order_struct.broker_identifier.clone(),
     unix_time,
     trader_identifier: trader_order_struct.trader_identifier,
     order_identifier:trader_order_struct.order_identifier,
     order_quantity: trader_order_struct.order_quantity,
     order_side: trader_order_struct.order_side.clone(),
     expiration:trader_order_struct.expiration.clone(),
     price: trader_order_struct.price,
     pointing_at:trader_order_struct.pointing_at,
    }

}
pub fn delete_iceberg_struct (iceberg_order_struct: &IcebergOrderStruct,unix_time: i64)-> DeletedIcebergStruct {

    DeletedIcebergStruct {
     market:iceberg_order_struct.market.clone(),
    broker_identifier: iceberg_order_struct.broker_identifier.clone(),
     unix_time,
     trader_identifier: iceberg_order_struct.trader_identifier,
     iceberg_identifier:iceberg_order_struct.iceberg_identifier,
     total_quantity: iceberg_order_struct.total_quantity,
     visible_quantity: iceberg_order_struct.visible_quantity,
     resting_quantity: iceberg_order_struct.resting_quantity,
     order_side: iceberg_order_struct.order_side.clone(),
     expiration:iceberg_order_struct.expiration.clone(),
     price: iceberg_order_struct.price,
    }

}
pub fn delete_stop_struct (trader_stop_order_struct: &TraderStopOrderStruct,unix_time: i64)-> DeletedStopOrderStruct {

    DeletedStopOrderStruct {
     market: trader_stop_order_struct.market.clone(),
    broker_identifier: trader_stop_order_struct.broker_identifier.clone(),
     unix_time,
     trader_identifier: trader_stop_order_struct.trader_identifier,
     order_identifier:trader_stop_order_struct.order_identifier,
     order_quantity: trader_stop_order_struct.order_quantity,
     order_side: trader_stop_order_struct.order_side.clone(),
     expiration:trader_stop_order_struct.expiration.clone(),
     trigger_price: trader_stop_order_struct.trigger_price,
     pointing_at:trader_stop_order_struct.pointing_at,
    }
}
pub fn delete_stop_limit_struct (trader_stop_limit_order_struct: &TraderStopLimitOrderStruct,unix_time: i64)-> DeletedStopLimitOrderStruct {

        DeletedStopLimitOrderStruct {
         market: trader_stop_limit_order_struct.market.clone(),
        broker_identifier:trader_stop_limit_order_struct.broker_identifier.clone(),
         unix_time,
         trader_identifier: trader_stop_limit_order_struct.trader_identifier,
         order_identifier:trader_stop_limit_order_struct.order_identifier,
         order_quantity: trader_stop_limit_order_struct.order_quantity,
         order_side: trader_stop_limit_order_struct.order_side.clone(),
         expiration:trader_stop_limit_order_struct.expiration.clone(),
         trigger_price: trader_stop_limit_order_struct.trigger_price,
         price: trader_stop_limit_order_struct.price,
         pointing_at:trader_stop_limit_order_struct.pointing_at,
        }
    
    }


pub fn mbp_event (unix_time:i64,side:String,event_value:i32,event_price:i32,calc:i32,tx_market: &mpsc::Sender<Structs>,config:&MarketConf)  {
    
    let calculated_value = calc *event_value;
    let event = MBPEvents {
        unix_time,
        market:config.market_name.clone(),
        side:side.to_string(),
        event_value: calculated_value,
        event_price,
    };
    let event_message = Structs::MBPEvents(event);
    if let Err(e) = tx_market.send(event_message) {
        eprintln!("Failed to send message: {:?}", e);
    }

}
pub fn interest_event (unix_time:i64,side:String,event_value:i32,event_price:i32,calc:i32,tx_market: &mpsc::Sender<Structs>,config:&MarketConf)  {
    
    let calculated_value = calc *event_value;
    let event = InterestEvents {
        unix_time,
        market:config.market_name.clone(),
        side:side.to_string(),
        event_value: calculated_value,
        event_price,
    };
    let event_message = Structs::InterestEvents(event);
    if let Err(e) = tx_market.send(event_message) {
        eprintln!("Failed to send message: {:?}", e);
    }

}

pub fn ex_stop(
    stop_struct: &DashMap<i64, TraderStopOrderStruct>,
    stop_map: &mut MAPStopData,
    last_dyn: &Last,
    tx: &mpsc::Sender<Structs>,//tx internal
    tx_broker: &mpsc::Sender<Structs>,//tx broker
) {
    let mut exist = false;
    // Check if there are any order ids at the last_dyn price
    if let Some(order_ids) = stop_map.map.get(&last_dyn.price) {
        // Iterate over each order id
        exist = true;
        for &order_id in order_ids {
            // Get the TraderStopOrderStruct associated with the order id
            if let Some((_,order_struct)) = stop_struct.remove(&order_id) {
                // Create a MarketOrder from the TraderStopOrderStruct
                let market_order = MarketOrder {
                    market: order_struct.market.clone(),
                    broker_identifier: order_struct.broker_identifier.clone(),
                    trader_identifier: order_struct.trader_identifier,
                    order_identifier: Some(order_struct.order_identifier),
                    order_quantity: order_struct.order_quantity,
                    order_side: order_struct.order_side.clone(),
                    expiration: order_struct.expiration.clone(),
                    pointing_at:order_struct.pointing_at,
                };
                let order_message = Structs::MarketOrder(market_order);
               
                if let Err(e) = tx.send(order_message) {
                    eprintln!("Failed to send message: {:?}", e);
                }

                let string_m = format! ("{} {}, {} {} {} Stop order id {} entered to market", Utc::now().timestamp_micros(),order_struct.market.clone(),order_struct.order_quantity,order_struct.order_side.clone(),order_struct.expiration.clone(),order_struct.order_identifier );
                let message = Messaging {
                    unix_time :  Utc::now().timestamp_micros(),
                    market : order_struct.market.clone(),
                    broker_identifier : order_struct.broker_identifier.clone(),
                    trader_identifier : order_struct.trader_identifier,
                    message : string_m,
                };
                let m_message = Structs::Messaging(message);
                if let Err(e) = tx_broker.send(m_message) {
                    eprintln!("Failed to send message: {:?}", e);
                }

                let ex_stop = ExecutedStop {
                    unix_time :  Utc::now().timestamp_micros(),
                    market : order_struct.market.clone(),
                    broker_identifier : order_struct.broker_identifier.clone(),
                    trader_identifier : order_struct.trader_identifier,
                    order_identifier : order_struct.order_identifier,
                    order_quantity : order_struct.order_quantity,
                    order_side : order_struct.order_side.clone(),
                    expiration : order_struct.expiration.clone(),
                    trigger_price : order_struct.trigger_price,
                    pointing_at:order_struct.pointing_at,
                };
                let ex_message = Structs::ExecutedStop(ex_stop);
                if let Err(e) = tx_broker.send(ex_message) {
                    eprintln!("Failed to send message: {:?}", e);
                }

            }
        }
        
    }
    if exist {
        stop_map.map.remove(&last_dyn.price);
        exist = false;
    }
}
pub fn ex_stop2(
    stop_struct: &DashMap<i64, TraderStopOrderStruct>,
    stop_map: &mut MAPStopData,
    last_dyn: &Last,
    tx: &mpsc::Sender<Structs>,//tx internal
   
) {
    let mut exist = false;
    // Check if there are any order ids at the last_dyn price
    if let Some(order_ids) = stop_map.map.get(&last_dyn.price) {
        // Iterate over each order id
        exist = true;
        for &order_id in order_ids {
            // Get the TraderStopOrderStruct associated with the order id
            if let Some((_,order_struct)) = stop_struct.remove(&order_id) {
                // Create a MarketOrder from the TraderStopOrderStruct
                let market_order = MarketOrder {
                    market: order_struct.market.clone(),
                    broker_identifier: order_struct.broker_identifier.clone(),
                    trader_identifier: order_struct.trader_identifier,
                    order_identifier: Some(order_struct.order_identifier),
                    order_quantity: order_struct.order_quantity,
                    order_side: order_struct.order_side.clone(),
                    expiration: order_struct.expiration.clone(),
                    pointing_at:order_struct.pointing_at,
                };
                let order_message = Structs::MarketOrder(market_order);
              
                if let Err(e) = tx.send(order_message) {
                    eprintln!("Failed to send message: {:?}", e);
                }

               

            }
        }
        
    }
    if exist {
        stop_map.map.remove(&last_dyn.price);
        exist = false;
    }
}
pub fn ex_stop_limit(
    stop_limit_struct: &DashMap<i64, TraderStopLimitOrderStruct>,
    stop_limit_map: &mut MAPStopLimitData,
    last_dyn: &Last,
    tx: &mpsc::Sender<Structs>,
    tx_broker: &mpsc::Sender<Structs>,//tx broker
) {
    let mut exist = false;
    // Check if there are any order ids at the last_dyn price
    if let Some(order_ids) = stop_limit_map.map.get(&last_dyn.price) {
        // Iterate over each order id
        exist = true;
        for &order_id in order_ids {
            // Get the TraderStopOrderStruct associated with the order id
            if let Some((_,order_struct)) = stop_limit_struct.remove(&order_id) {
                // Create a MarketOrder from the TraderStopOrderStruct
                let limit_order = LimitOrder {
                    market: order_struct.market.clone(),
                    broker_identifier: order_struct.broker_identifier.clone(),
                    trader_identifier: order_struct.trader_identifier,
                    order_identifier: Some(order_struct.order_identifier),
                    order_quantity: order_struct.order_quantity,
                    order_side: order_struct.order_side.clone(),
                    expiration: order_struct.expiration.clone(),
                    price: order_struct.price,
                    pointing_at:order_struct.pointing_at,
                    
                };
                let order_message = Structs::LimitOrder(limit_order);
                 
                if let Err(e) = tx.send(order_message) {
                    eprintln!("Failed to send message: {:?}", e);
                }

                let string_m = format! ("{} {}, {} {} at {} price level {} Stop limit order id {} entered to market", Utc::now().timestamp_micros(),order_struct.market,order_struct.order_quantity,order_struct.order_side,order_struct.price,order_struct.expiration,order_struct.order_identifier );
                let message = Messaging {
                    unix_time :  Utc::now().timestamp_micros(),
                    market : order_struct.market.clone(),
                    broker_identifier : order_struct.broker_identifier.clone(),
                    trader_identifier : order_struct.trader_identifier,
                    message : string_m,
                };
                let m_message = Structs::Messaging(message);
                if let Err(e) = tx_broker.send(m_message) {
                    eprintln!("Failed to send message: {:?}", e);
                }

                let ex_stop_limit = ExecutedStopLimit {
                    unix_time :  Utc::now().timestamp_micros(),
                    market : order_struct.market.clone(),
                    broker_identifier : order_struct.broker_identifier.clone(),
                    trader_identifier : order_struct.trader_identifier,
                    order_identifier : order_struct.order_identifier,
                    order_quantity : order_struct.order_quantity,
                    order_side : order_struct.order_side.clone(),
                    expiration : order_struct.expiration.clone(),
                    trigger_price : order_struct.trigger_price,
                    price : order_struct.price,
                    pointing_at: order_struct.pointing_at,
                };
                let ex_message = Structs::ExecutedStopLimit(ex_stop_limit);
                if let Err(e) = tx_broker.send(ex_message) {
                    eprintln!("Failed to send message: {:?}", e);
                }
            }
        }
        
    }
    if exist {
        stop_limit_map.map.remove(&last_dyn.price);
        exist = false;
    }
}

pub fn ex_stop_limit2(
    stop_limit_struct: &DashMap<i64, TraderStopLimitOrderStruct>,
    stop_limit_map: &mut MAPStopLimitData,
    last_dyn: &Last,
    tx: &mpsc::Sender<Structs>,
    
) {
    let mut exist = false;
    // Check if there are any order ids at the last_dyn price
    if let Some(order_ids) = stop_limit_map.map.get(&last_dyn.price) {
        // Iterate over each order id
        exist = true;
        for &order_id in order_ids {
            // Get the TraderStopOrderStruct associated with the order id
            if let Some((_,order_struct)) = stop_limit_struct.remove(&order_id) {
                // Create a MarketOrder from the TraderStopOrderStruct
                let limit_order = LimitOrder {
                    market: order_struct.market.clone(),
                    broker_identifier: order_struct.broker_identifier.clone(),
                    trader_identifier: order_struct.trader_identifier,
                    order_identifier: Some(order_struct.order_identifier),
                    order_quantity: order_struct.order_quantity,
                    order_side: order_struct.order_side.clone(),
                    expiration: order_struct.expiration.clone(),
                    price: order_struct.price,
                    pointing_at:order_struct.pointing_at,
                   
                };
                let order_message = Structs::LimitOrder(limit_order);
                
                if let Err(e) = tx.send(order_message) {
                    eprintln!("Failed to send message: {:?}", e);
                }

              
            }
        }
        
    }
    if exist {
        stop_limit_map.map.remove(&last_dyn.price);
        exist = false;
    }
}

pub fn message_limit_taker(arrival: &LimitOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: arrival.market.clone(),
        broker_identifier: arrival.broker_identifier.clone(),
        trader_identifier: arrival.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_position_maker(trade: &MatchStruct, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: trade.unix_time,
        market: trade.market.clone(),
        broker_identifier: trade.broker_identifier_maker.clone(),
        trader_identifier: trade.trader_identifier_maker,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_position_taker(trade: &MatchStruct, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: trade.unix_time,
        market: trade.market.clone(),
        broker_identifier: trade.broker_identifier_taker.clone(),
        trader_identifier: trade.trader_identifier_taker,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_market_taker(arrival: &MarketOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: arrival.market.clone(),
        broker_identifier: arrival.broker_identifier.clone(),
        trader_identifier: arrival.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_arrival_iceberg(arrival: &IcebergOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: arrival.market.clone(),
        broker_identifier: arrival.broker_identifier.clone(),
        trader_identifier: arrival.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}

pub fn message_limit_maker(limit: &TraderOrderStruct, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: limit.market.clone(),
        broker_identifier: limit.broker_identifier.clone(),
        trader_identifier: limit.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}

pub fn message_modify(modify: &ModifyOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: modify.market.clone(),
        broker_identifier: modify.broker_identifier.clone(),
        trader_identifier: modify.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_modify_iceberg(modify: &ModifyIcebergOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: modify.market.clone(),
        broker_identifier: modify.broker_identifier.clone(),
        trader_identifier: modify.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_delete_iceberg(delete: &DeleteIcebergOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: delete.market.clone(),
        broker_identifier: delete.broker_identifier.clone(),
        trader_identifier: delete.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_delete(delete: &DeleteOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: delete.market.clone(),
        broker_identifier: delete.broker_identifier.clone(),
        trader_identifier: delete.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_stop(stop: &StopOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: stop.market.clone(),
        broker_identifier: stop.broker_identifier.clone(),
        trader_identifier: stop.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn message_stoplimit(stoplimit: &StopLimitOrder, tx_broker:&mpsc::Sender<Structs>, message_string: String) {
    // Format the message string
   
    // Create the Messaging struct
    let message_m = Messaging {
        unix_time: Utc::now().timestamp_micros(),
        market: stoplimit.market.clone(),
        broker_identifier: stoplimit.broker_identifier.clone(),
        trader_identifier: stoplimit.trader_identifier,
        message:message_string,
    };

    // Wrap the message into your defined enum/struct
    let m_message = Structs::Messaging(message_m);

    // Send the message via tx_sender and handle potential errors
    if let Err(e) = tx_broker.send(m_message) {
        eprintln!("Failed to send message: {:?}", e);
    }
}
pub fn ex_iceberg(iceberg_struct: &mut DashMap<i64, IcebergOrderStruct>,order_id:i64, tx: &mpsc::Sender<Structs>,
    tx_broker: &mpsc::Sender<Structs>) {

        let mut resting_empty = false;
        if let Some(mut ice_struct) = iceberg_struct.get_mut(&order_id) {
            if ice_struct.resting_quantity > ice_struct.visible_quantity {
                ice_struct.resting_quantity -= ice_struct.visible_quantity;
            let limit_order = LimitOrder {
                market: ice_struct.market.clone(),
                broker_identifier: ice_struct.broker_identifier.clone(),
                trader_identifier: ice_struct.trader_identifier,
                order_identifier: Some(ice_struct.iceberg_identifier),
                order_quantity: ice_struct.visible_quantity,
                order_side: ice_struct.order_side.clone(),
                expiration: ice_struct.expiration.clone(),
                price: ice_struct.price,
                pointing_at:None, 
            };
            let order_message = Structs::LimitOrder(limit_order);
             
            if let Err(e) = tx.send(order_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
    
            let string_m = format! ("{} {}, {} {} at {} price level {} visible iceberg limit order id {} entered to market.", Utc::now().timestamp_micros(),ice_struct.market,ice_struct.visible_quantity,ice_struct.order_side,ice_struct.price,ice_struct.expiration,ice_struct.iceberg_identifier );
            let message = Messaging {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier :ice_struct.broker_identifier.clone(),
                trader_identifier : ice_struct.trader_identifier,
                message : string_m,
            };
            let m_message = Structs::Messaging(message);
            if let Err(e) = tx_broker.send(m_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
            let string_m = format! ("{} {}, Iceberg order with id {} and total quantity of {} contracts, {} executed, resting {}. ", Utc::now().timestamp_micros(),ice_struct.market,ice_struct.iceberg_identifier ,ice_struct.total_quantity,ice_struct.visible_quantity,ice_struct.resting_quantity);
            let message = Messaging {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier :ice_struct.broker_identifier.clone(),
                trader_identifier : ice_struct.trader_identifier,
                message : string_m,
            };
            let m_message = Structs::Messaging(message);
            if let Err(e) = tx_broker.send(m_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
            let ex_iceberg = ExecutedIceberg {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier : ice_struct.broker_identifier.clone(),
                trader_identifier :ice_struct.trader_identifier,
                iceberg_identifier : ice_struct.iceberg_identifier,
                executed_quantity : ice_struct.visible_quantity,
                order_side : ice_struct.order_side.clone(),
                expiration : ice_struct.expiration.clone(),
                price : ice_struct.price,
            };
            let ex_message = Structs::ExecutedIceberg(ex_iceberg);
            if let Err(e) = tx_broker.send(ex_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
        } else  if ice_struct.resting_quantity == ice_struct.visible_quantity {
            resting_empty = true;

            let limit_order = LimitOrder {
                market: ice_struct.market.clone(),
                broker_identifier: ice_struct.broker_identifier.clone(),
                trader_identifier: ice_struct.trader_identifier,
                order_identifier: Some(ice_struct.iceberg_identifier),
                order_quantity: ice_struct.visible_quantity,
                order_side: ice_struct.order_side.clone(),
                expiration: ice_struct.expiration.clone(),
                price: ice_struct.price,
                pointing_at:None, 
            };
            let order_message = Structs::LimitOrder(limit_order);
             
            if let Err(e) = tx.send(order_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
    
            let string_m = format! ("{} {}, {} {} at {} price level {} visible iceberg limit order id {} entered to market.", Utc::now().timestamp_micros(),ice_struct.market,ice_struct.visible_quantity,ice_struct.order_side,ice_struct.price,ice_struct.expiration,ice_struct.iceberg_identifier );
            let message = Messaging {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier :ice_struct.broker_identifier.clone(),
                trader_identifier : ice_struct.trader_identifier,
                message : string_m,
            };
            let m_message = Structs::Messaging(message);
            if let Err(e) = tx_broker.send(m_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
            let string_m = format! ("{} {}, Iceberg order with id {} and total quantity of {} contracts fully executed. ", Utc::now().timestamp_micros(),ice_struct.market,ice_struct.iceberg_identifier ,ice_struct.total_quantity);
            let message = Messaging {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier :ice_struct.broker_identifier.clone(),
                trader_identifier : ice_struct.trader_identifier,
                message : string_m,
            };
            let m_message = Structs::Messaging(message);
            if let Err(e) = tx_broker.send(m_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
            let ex_iceberg = ExecutedIceberg {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier : ice_struct.broker_identifier.clone(),
                trader_identifier :ice_struct.trader_identifier,
                iceberg_identifier : ice_struct.iceberg_identifier,
                executed_quantity : ice_struct.visible_quantity,
                order_side : ice_struct.order_side.clone(),
                expiration : ice_struct.expiration.clone(),
                price : ice_struct.price,
            };
            let ex_message = Structs::ExecutedIceberg(ex_iceberg);
            if let Err(e) = tx_broker.send(ex_message) {
                eprintln!("Failed to send message: {:?}", e);
            }

        } else if ice_struct.resting_quantity < ice_struct.visible_quantity {
            resting_empty = true;

            let limit_order = LimitOrder {
                market: ice_struct.market.clone(),
                broker_identifier: ice_struct.broker_identifier.clone(),
                trader_identifier: ice_struct.trader_identifier,
                order_identifier: Some(ice_struct.iceberg_identifier),
                order_quantity: ice_struct.resting_quantity,
                order_side: ice_struct.order_side.clone(),
                expiration: ice_struct.expiration.clone(),
                price: ice_struct.price,
                pointing_at:None, 
            };
            let order_message = Structs::LimitOrder(limit_order);
             
            if let Err(e) = tx.send(order_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
    
            let string_m = format! ("{} {}, {} {} at {} price level {} visible iceberg limit order id {} entered to market.", Utc::now().timestamp_micros(),ice_struct.market,ice_struct.resting_quantity,ice_struct.order_side,ice_struct.price,ice_struct.expiration,ice_struct.iceberg_identifier );
            let message = Messaging {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier :ice_struct.broker_identifier.clone(),
                trader_identifier : ice_struct.trader_identifier,
                message : string_m,
            };
            let m_message = Structs::Messaging(message);
            if let Err(e) = tx_broker.send(m_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
            let string_m = format! ("{} {}, Iceberg order with id {} and total quantity of {} contracts fully executed. ", Utc::now().timestamp_micros(),ice_struct.market,ice_struct.iceberg_identifier ,ice_struct.total_quantity);
            let message = Messaging {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier :ice_struct.broker_identifier.clone(),
                trader_identifier : ice_struct.trader_identifier,
                message : string_m,
            };
            let m_message = Structs::Messaging(message);
            if let Err(e) = tx_broker.send(m_message) {
                eprintln!("Failed to send message: {:?}", e);
            }
            let ex_iceberg = ExecutedIceberg {
                unix_time :  Utc::now().timestamp_micros(),
                market : ice_struct.market.clone(),
                broker_identifier : ice_struct.broker_identifier.clone(),
                trader_identifier :ice_struct.trader_identifier,
                iceberg_identifier : ice_struct.iceberg_identifier,
                executed_quantity : ice_struct.resting_quantity,
                order_side : ice_struct.order_side.clone(),
                expiration : ice_struct.expiration.clone(),
                price : ice_struct.price,
            };
            let ex_message = Structs::ExecutedIceberg(ex_iceberg);
            if let Err(e) = tx_broker.send(ex_message) {
                eprintln!("Failed to send message: {:?}", e);
            }

        }
        if resting_empty {
            iceberg_struct.remove(&order_id);
        }

        }//fin
    } 

    pub fn save_to_json_file<T: serde::Serialize>(data: &T, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Convert the data to a JSON string
        let json_data = to_string(data)?;
    
        // Create or overwrite the file at the specified path
        let mut file = File::create(file_path)?;
    
        // Write the JSON data to the file
        file.write_all(json_data.as_bytes())?;
    
        Ok(())
    }

    pub fn load_from_json_file<T: serde::de::DeserializeOwned>(file_path: &str) -> Result<T, Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let data = from_reader(file)?;
        Ok(data)
    }

    pub async fn insert_document_collection<T>(
        db: &Database,
        collection_name: &str,
        item: &T,
    ) -> Result<(), mongodb::error::Error>
    where
        T: Serialize,
    {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
        let doc = bson::to_document(item)?;
        collection.insert_one(doc).await?;
        Ok(())
    }
    
    pub async fn delete_document_by_orderid(
        db: &Database,
        collection_name: &str,
        order_id: i64,
    ) -> Result<(), mongodb::error::Error> {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
    
        // Create a filter to match the document by the `order_id` field
        let filter = doc! { "order_identifier": order_id };
    
        // Delete the document
        collection.delete_one(filter).await?;
    
        Ok(())
    }
    pub async fn delete_iceberg_by_orderid(
        db: &Database,
        collection_name: &str,
        order_id: i64,
    ) -> Result<(), mongodb::error::Error> {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
    
        // Create a filter to match the document by the `order_id` field
        let filter = doc! { "iceberg_identifier": order_id };
    
        // Delete the document
        collection.delete_one(filter).await?;
    
        Ok(())
    }
    pub async fn delete_document_by_posid(
        db: &Database,
        collection_name: &str,
        position_id: i64,
    ) -> Result<(), mongodb::error::Error> {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
    
        // Create a filter to match the document by the `order_id` field
        let filter = doc! { "position_identifier": position_id };
    
        // Delete the document
        collection.delete_one(filter).await?;
    
        Ok(())
    }
    pub async fn modify_document_by_orderid(
        db: &Database,
        collection_name: &str,
        order_id: i64,
        new_quantity: i32,
    ) -> Result<(), mongodb::error::Error> {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
    
        // Create a filter to match the document by the `order_id` field
        let filter = doc! { "order_identifier": order_id };
    
        // Create the update to set the new order_quantity
        let update = doc! {
            "$set": { "order_quantity": new_quantity }
        };
    
        // Update the document
        collection.update_one(filter, update).await?;
    
        Ok(())
    }
    pub async fn modify_iceberg_by_orderid(
        db: &Database,
        collection_name: &str,
        order_id: i64,
        new_quantity: i32,
        new_visible_quantity:i32,
    ) -> Result<(), mongodb::error::Error> {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
    
        // Create a filter to match the document by the `order_id` field
        let filter = doc! { "iceberg_identifier": order_id };
    
        // Create the update to set the new order_quantity
        let update = doc! {
            "$set": {
                "total_quantity": new_quantity,
                "visible_quantity": new_visible_quantity,
                "resting_quantity": new_quantity
            }
        };
    
        // Update the document
        collection.update_one(filter, update).await?;
    
        Ok(())
    }
    pub async fn update_iceberg_resting_by_orderid(
        db: &Database,
        collection_name: &str,
        order_id: i64,
        new_quantity: i32,
    ) -> Result<(), mongodb::error::Error> {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
    
        // Create a filter to match the document by the `order_id` field
        let filter = doc! { "iceberg_identifier": order_id };
    
        // Create the update to set the new order_quantity
        let update = doc! {
            "$set": {
               
                "resting_quantity": new_quantity
            }
        };
    
        // Update the document
        collection.update_one(filter, update).await?;
    
        Ok(())
    }
    pub async fn modify_document_by_posid(
        db: &Database,
        collection_name: &str,
        position_id: i64,
        new_quantity: i32,
    ) -> Result<(), mongodb::error::Error> {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
    
        // Create a filter to match the document by the `order_id` field
        let filter = doc! { "position_identifier": position_id };
    
        // Create the update to set the new order_quantity
        let update = doc! {
            "$set": { "position_quantity": new_quantity }
        };
    
        // Update the document
        collection.update_one(filter, update).await?;
    
        Ok(())
    }
    pub async fn fetch_document_byorderid<T>(
        db: &Database,
        collection_name: &str,
        order_id: i64,
    ) -> Result<T, String>
    where 
    T: DeserializeOwned + Unpin + Debug + Send + Sync,
     {
        let collection = db.collection::<T>(collection_name);
    
        // Find the user by id_user
        let filter = doc! { "order_identifier": order_id };
        let document = collection
            .find_one(filter)
            .await
            .map_err(|err| format!("Failed to query the database: {}", err))?;
    
        match document {
            Some(doc) => Ok(doc),
            None => Err(format!("Document not found in collection '{}'", collection_name)),
        }
    }
    pub async fn fetch_iceberg<T>(
        db: &Database,
        collection_name: &str,
        order_id: i64,
    ) -> Result<T, String>
    where 
    T: DeserializeOwned + Unpin + Debug + Send + Sync,
     {
        let collection = db.collection::<T>(collection_name);
    
        // Find the user by id_user
        let filter = doc! { "iceberg_identifier": order_id };
        let document = collection
            .find_one(filter)
            .await
            .map_err(|err| format!("Failed to query the database: {}", err))?;
    
        match document {
            Some(doc) => Ok(doc),
            None => Err(format!("Document not found in collection '{}'", collection_name)),
        }
    }
    pub async fn fetch_document_position<T>(
        db: &Database,
        collection_name: &str,
        position_id: i64,
    ) -> Result<T, String>
    where 
    T: DeserializeOwned + Unpin + Debug + Send + Sync,
     {
        let collection = db.collection::<T>(collection_name);
    
        // Find the user by id_user
        let filter = doc! { "position_identifier": position_id };
        let document = collection
            .find_one(filter)
            .await
            .map_err(|err| format!("Failed to query the database: {}", err))?;
    
        match document {
            Some(doc) => Ok(doc),
            None => Err(format!("Document not found in collection '{}'", collection_name)),
        }
    }
    pub async fn fetch_document_traderid<T>(
        db: &Database,
        collection_name: &str,
        trader_id: i64,
    ) -> Result<T, String>
    where 
    T: DeserializeOwned + Unpin + Debug + Send + Sync,
     {
        let collection = db.collection::<T>(collection_name);
    
        // Find the user by id_user
        let filter = doc! { "trader_identifier": trader_id };
        let document = collection
            .find_one(filter)
            .await
            .map_err(|err| format!("Failed to query the database: {}", err))?;
    
        match document {
            Some(doc) => Ok(doc),
            None => Err(format!("Document not found in collection '{}'", collection_name)),
        }
    }
    pub async fn fetch_all_documents_by_trader_id<T>(
        db: &Database,
        collection_name: &str,
        trader_id: i64,
    ) -> Result<Vec<T>, String>
    where
        T: DeserializeOwned + Unpin + Debug + Send + Sync,
    {
        let collection = db.collection::<T>(collection_name);
    
        // Create a filter to match all documents with the specified trader_identifier
        let filter = doc! { "trader_identifier": trader_id };
    
        // Use the find method to retrieve a cursor to the matching documents
        let mut cursor = collection
            .find(filter)
            .await
            .map_err(|err| format!("Failed to query the database: {}", err))?;
    
        // Collect the documents into a vector
        let mut documents = Vec::new();
        while let Some(result) = cursor.next().await {
            match result {
                Ok(doc) => documents.push(doc),
                Err(err) => {
                    return Err(format!("Error reading document from cursor: {}", err));
                }
            }
        }
    
        if documents.is_empty() {
           Ok( vec![])
        } else {
            Ok(documents)
        }
    }
    pub async fn update_balance_by_traderid(
        db: &Database,
        collection_name: &str,
        trader_id: i64,
        new_balance: i32,
    ) -> Result<(), mongodb::error::Error> {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
    
        // Create a filter to match the document by the `order_id` field
        let filter = doc! { "trader_identifier": trader_id };
    
        // Create the update to set the new order_quantity
        let update = doc! {
            "$set": { "balance": new_balance }
        };
    
        // Update the document
        collection.update_one(filter, update).await?;
    
        Ok(())
    }

    pub async fn overwrite_document<T>(
        db: &Database,
        collection_name: &str,
        item: &T,
    ) -> Result<(), String>
    where
        T: Serialize + DeserializeOwned + Unpin + Send + Sync + Debug,
    {
        let collection: mongodb::Collection<Document> = db.collection(collection_name);
        
        // Check if the collection already has one document
        let existing_doc = collection
            .find_one(doc! {})
            .await
            .map_err(|err| format!("Failed to query the database: {}", err))?;
    
        let doc = bson::to_document(item)
            .map_err(|err| format!("Failed to serialize item: {}", err))?;
    
        // If a document exists, overwrite it, otherwise insert a new document
        if existing_doc.is_some() {
            // Replace the existing document
            collection
                .replace_one(doc! {}, doc) // {} filter matches any document
                .await
                .map_err(|err| format!("Failed to replace the document: {}", err))?;
        } else {
            // Insert a new document if none exists
            collection
                .insert_one(doc)
                .await
                .map_err(|err| format!("Failed to insert a new document: {}", err))?;
        }
    
        Ok(())
    }

    pub async fn fetch_number_documents<T>(
        db: &Database,
        collection_name: &str,
        number:u32,
    ) -> Result<Vec<T>, String>
    where
        T: DeserializeOwned + Unpin + Debug + Send + Sync,
    {
        let collection = db.collection::<T>(collection_name);
    
        // Sort by unix_time in descending order and limit to 1000 documents
        let pipeline = vec![
            doc! { "$sort": { "unix_time": -1 } }, // Sort by unix_time in descending order
            doc! { "$limit": number } // Limit to 1000 documents
        ];
    
        // Query the collection without a filter (i.e., fetch all documents)
        let cursor= collection
            .aggregate(pipeline) // Pass the aggregation pipeline
            .await
            .map_err(|err| format!("Failed to query the database: {}", err))?;
    
        let mut documents = Vec::new();
        let mut cursor_stream = cursor;
    
        // Iterate through the results and collect them into the Vec
        while let Some(result) = cursor_stream.next().await {
            match result {
                Ok(doc) => {
                    match bson::from_document::<T>(doc) {
                        Ok(deserialized) => documents.push(deserialized),
                        Err(err) => return Err(format!("Failed to deserialize document: {}", err)),
                    }
                },
                Err(err) => return Err(format!("Error fetching document: {}", err)),
            }
        }
    
        Ok(documents)
    }

    pub async fn fetch_one_document<T>(
        db: &Database,
        collection_name: &str,
    ) -> Result<Option<T>, String>
    where
        T: DeserializeOwned + Unpin + Debug + Send + Sync,
    {
        let collection = db.collection::<T>(collection_name);
    
        let document = collection
            .find_one(doc! {})
            .await
            .map_err(|err| format!("Failed to query the database: {}", err))?;
    
        Ok(document)
    }