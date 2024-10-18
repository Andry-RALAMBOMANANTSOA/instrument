use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use mongodb::{Client, options::ClientOptions};
use dotenv::dotenv;
use std::env;
use dashmap::DashMap;
use tokio::time::{sleep, Duration};
use std::sync::mpsc as sync_mpsc;
use shared_structs::*;
mod websocket;
use crate::websocket::*;
use std::sync::{Arc,Mutex};
use std::collections::BTreeMap;
mod function;
use std::time::Instant;
mod handler;
mod shared;
mod dedic_structs;

use crate::handler::*;
use crate::dedic_structs::*;
use crate::function::*;
use std::thread;

mod env_coll_decl;
use env_coll_decl::CollConfig;
use crate::shared::*;
use chrono::Utc;
use std::collections::btree_map::Entry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let broker_config = read_broker_config("./broker-hmac.json")
    .expect("Failed to load broker.json");
    let market_config = read_market_config("./market.json")
    .expect("Failed to load market.json");
let market_spec_config = read_market_spec_config("./marketspec.json")
        .expect("Failed to load marketspec.json");
let server_url = env::var("SERVER_URL")
.expect("Failed to get SERVER URL");
let config =  read_marketconf( "./marketcfg.json") 
    .expect("Failed to load marketcfg.json");

    //DB market 
    let mongo_uri_market = env::var("MONGO_URI_MARKET").expect("MONGO_URI must be set");
let db_name_market = env::var("DB_DATA_MARKET").expect("DB_DATA must be set");
let client_options_market = ClientOptions::parse(&mongo_uri_market).await.expect("Failed to parse MongoDB URI");
let client_market = Client::with_options(client_options_market).expect("Failed to initialize MongoDB client");
let db_m = client_market.database(&db_name_market);
let db_market = MarketDb { db: db_m };
 //DB Broker
 let mongo_uri_broker = env::var("MONGO_URI_BROKER").expect("MONGO_URI must be set");
let db_name_broker = env::var("DB_DATA_BROKER").expect("DB_DATA must be set");
let client_options_broker = ClientOptions::parse(&mongo_uri_broker).await.expect("Failed to parse MongoDB URI");
let client_broker = Client::with_options(client_options_broker).expect("Failed to initialize MongoDB client");
let db_b = client_broker.database(&db_name_broker);
let db_broker = BrokerDb { db: db_b };
//////////////////////////////////////////////////////////////////
let mut bid_struct = DashMap::<i64, TraderOrderStruct>::new();//key is order_identifier or maker id
    let mut bid_map: MAPData = MAPData { map:BTreeMap::new() };//key is price, value is vec of order identifier
    let mut bid_mbo: MBOData = MBOData { mbo: BTreeMap::new() };//key is price, value is vec of order quantity
    let mut bid_mbp: MBPData = MBPData { mbp: BTreeMap::new() };//key is price, value is is sum of quantity
    
    //ask data part
    let mut ask_struct = DashMap::<i64, TraderOrderStruct>::new();//key is order_identifier or maker id
    let mut ask_map: MAPData = MAPData { map:BTreeMap::new() };//key is price, value is vec of order identifier
    let mut ask_mbo: MBOData = MBOData { mbo: BTreeMap::new() };//key is price, value is vec of order quantity
    let mut ask_mbp: MBPData = MBPData { mbp: BTreeMap::new() };//key is price, value is is sum of quantity

    //stop data
    let mut stop_struct = DashMap::<i64, TraderStopOrderStruct>::new();
    let mut stop_map: MAPStopData = MAPStopData { map:BTreeMap::new()};

    //stop limit data
    let mut stop_limit_struct = DashMap::<i64, TraderStopLimitOrderStruct>::new();
    let mut stop_limit_map: MAPStopLimitData = MAPStopLimitData { map:BTreeMap::new()};

    //Interest data
    let mut long_tree: InterestTree = InterestTree { interest:BTreeMap::new() };
    let mut short_tree: InterestTree = InterestTree { interest:BTreeMap::new() };
    let mut long_map = DashMap::<i64, Vec<PositionStruct>>::new();
    let mut short_map = DashMap::<i64, Vec<PositionStruct>>::new();

    //Iceberg
    let mut iceberg_struct = DashMap::<i64,IcebergOrderStruct>::new();
   
    // Load data from JSON files
    if let Err(e) = load_from_json_file::<BTreeMap<i64, TraderOrderStruct>>("bid_struct.json").map(|data| { 
        for (key, value) in data {
            bid_struct.insert(key, value);
        }
    }) {
        //eprintln!("Failed to load bid_struct: {}", e);
    }
if let Err(e) = load_from_json_file("bid_map.json").map(|data: MAPData| { bid_map = data }) {
   //eprintln!("Failed to load bid_map or still inexistant: {}", e);
}
if let Err(e) = load_from_json_file("bid_mbo.json").map(|data: MBOData| { bid_mbo = data }) {
    //eprintln!("Failed to load bid_mbo or still inexistant: {}", e);
}
if let Err(e) = load_from_json_file("bid_mbp.json").map(|data: MBPData| { bid_mbp = data }) {
    //eprintln!("Failed to load bid_mbp or still inexistant: {}", e);
}

// Similar loading for ask data
if let Err(e) = load_from_json_file::<BTreeMap<i64, TraderOrderStruct>>("ask_struct.json").map(|data| { 
    for (key, value) in data {
        ask_struct.insert(key, value);
    }
}) {
    //eprintln!("Failed to load ask_struct: {}", e);
}
if let Err(e) = load_from_json_file("ask_map.json").map(|data: MAPData| { ask_map = data }) {
    //eprintln!("Failed to load ask_map or still inexistant: {}", e);
}
if let Err(e) = load_from_json_file("ask_mbo.json").map(|data: MBOData| { ask_mbo = data }) {
    //eprintln!("Failed to load ask_mbo or still inexistant: {}", e);
}
if let Err(e) = load_from_json_file("ask_mbp.json").map(|data: MBPData| { ask_mbp = data }) {
    //eprintln!("Failed to load ask_mbp or still inexistant: {}", e);
}

// Load stop data
if let Err(e) = load_from_json_file::<BTreeMap<i64, TraderStopOrderStruct>>("stop_struct.json").map(|data| { 
    for (key, value) in data {
        stop_struct.insert(key, value);
    }
}) {
    //eprintln!("Failed to load stop_struct or still inexistant: {}", e);
}
if let Err(e) = load_from_json_file("stop_map.json").map(|data: MAPStopData| { stop_map = data }) {
    //eprintln!("Failed to load stop_map or still inexistant: {}", e);
}

// Load stop limit data
if let Err(e) = load_from_json_file::<BTreeMap<i64, TraderStopLimitOrderStruct>>("stop_limit_struct.json").map(|data| { 
    for (key, value) in data {
        stop_limit_struct.insert(key, value);
    }
}) {
    //eprintln!("Failed to load stop_limit_struct or still inexistant: {}", e);
}
if let Err(e) = load_from_json_file("stop_limit_map.json").map(|data: MAPStopLimitData| { stop_limit_map = data }) {
    //eprintln!("Failed to load stop_limit_map or still inexistant: {}", e);
}

// Load interest data
if let Err(e) = load_from_json_file("long_tree.json").map(|data: InterestTree| { long_tree = data }) {
    //eprintln!("Failed to load long_tree or still inexistant: {}", e);
}
if let Err(e) = load_from_json_file("short_tree.json").map(|data: InterestTree| { short_tree = data }) {
    //eprintln!("Failed to load short_tree or still inexistant: {}", e);
}
if let Err(e) = load_from_json_file::<BTreeMap<i64, Vec<PositionStruct>>>("long_map.json").map(|data| { 
    for (key, value) in data {
        long_map.insert(key, value);
    }
}) {
    //eprintln!("Failed to load long_map: {}", e);
}
if let Err(e) = load_from_json_file::<BTreeMap<i64, Vec<PositionStruct>>>("short_map.json").map(|data| { 
    for (key, value) in data {
        short_map.insert(key, value);
    }
}) {
    //eprintln!("Failed to load short_map or still inexistant: {}", e);
}

// Load iceberg data
if let Err(e) = load_from_json_file::<BTreeMap<i64, IcebergOrderStruct>>("iceberg_struct.json").map(|data| { 
    for (key, value) in data {
        iceberg_struct.insert(key, value);
    }
}) {
    //eprintln!("Failed to load iceberg_struct or still inexistant: {}", e);
}
println!("Market {} is running!",config.market_name.clone());

let mut last_dyn = Last {
    unix_time:0,
    market:config.market_name.clone(),
    price: 0, //  initial value 
};
let mut tns =TimeSale {
    market: "".to_string(),
   exchange:"".to_string(),
   unix_time: 0,
   order_quantity: 0,
   order_side: OrderSide::Unspecified,
   price:0,
   };


let mut trade = MatchStruct {
    market:  "".to_string(),
broker_identifier_taker:  "".to_string(),
broker_identifier_maker:  "".to_string(),
unix_time: 0,
match_identifier:0,
trader_identifier_taker: 0,
order_identifier_taker: 0,
trader_identifier_maker: 0,
order_identifier_maker: 0,
maker_position_side:OrderSide::Unspecified,
taker_position_side:OrderSide::Unspecified,
taker_type:"".to_string(),
expiration_taker:  OrderExpiration::Unspecified,
expiration_maker:  OrderExpiration::Unspecified,
order_quantity: 0,
order_side:   OrderSide::Unspecified,
price: 0,
pointing_at_maker:Some(0),
pointing_at_taker:Some(0),
};

let mut bbo_http = Arc::new(Mutex::new(BBO {
    unix_time: 0,            // Example initialization
    market: config.market_name.clone(),
    ask_price: None,
    bid_price: None,
    ask_size: None,
    bid_size: None,
}));
let bbo_http_clone = Arc::clone(&bbo_http);  
    
//let (tx_tokio, mut rx_tokio) = mpsc::unbounded_channel::<Structs>();
    //let tx_tokio = Arc::new(tx_tokio);
    let (tx_order, rx_order) = sync_mpsc::channel::<Structs>();
    let (tx_market,rx_market) = sync_mpsc::channel::<Structs>();
let (tx_broker,rx_broker) = sync_mpsc::channel::<Structs>();
let (tx_position,rx_position) = sync_mpsc::channel::<Structs>();


let tx2 = tx_order.clone();
let txb2 = tx_broker.clone();
let txb3 = tx_broker.clone();

let txm2 = tx_market.clone();
let config_clone_http = config.clone();
let config_position = config.clone();
let db1_market =  Arc::new(db_market);
let db1_broker =  Arc::new(db_broker);
let ws_connections : WsConnections = Arc::new(DashMap::new());
let  connection_type_map : ConnectionTypeMap = Arc::new(DashMap::new());
let coll_config = Arc::new(CollConfig::new());

let mut last_save_time_position = Instant::now();

thread::spawn(move || { //positioning
    loop {
        if let Ok(match_a) = rx_position.try_recv() {
            match match_a {
                Structs::MatchStruct(trade) => {

                    // Handle Taker Side based on position side
                    match trade.taker_position_side {
                        OrderSide::Long => {
                            // Check inside short_map for the trader_identifier_taker.
                            let mut is_empty = false;
                            if let Some(mut positions) = short_map.get_mut(&trade.trader_identifier_taker) {
                                // Check pointing_at_taker if not None
                                let position_id_closer = id_i64();
                                if let Some(position_id) = trade.pointing_at_taker {
                                    // Find the specific position in the vector
                                    if let Some(position) = positions.iter_mut().find(|p| p.position_identifier == position_id) {
                                       
                                            let mut remaining_quantity = trade.order_quantity;
                
                                            // Subtract as much as possible from the specific position
                                            let subtract_quantity = remaining_quantity.min(position.position_quantity);
                                            remaining_quantity -= subtract_quantity;
                                            position.position_quantity -= subtract_quantity;

                                            if let Some(quantity) = short_tree.interest.get_mut(&position.price) {
                                                *quantity -= subtract_quantity;
                                                if *quantity == 0 {
                                                    short_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                }
                                            }
                            
                                            // Create a ClosePositionStruct for the closed portion
                                            let close_position = ClosePositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_taker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: position.position_identifier,
                                                position_identifier_closer: position_id_closer, // Identifier of the closer
                                                trader_identifier: trade.trader_identifier_taker,
                                                position_side: OrderSide::Short,
                                                position_quantity: subtract_quantity,
                                                initial_price: position.price,
                                                closing_price: trade.price,
                                            };
                            
                                            // Send the close position message
                                            let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                            if let Err(e) = txb3.send(close_position_message) {
                                                eprintln!("Failed to send close position message: {:?}", e);
                                            }
                            
                                            let string_m = format!(
                                                "{} {}, {} {} position from price level {} to {}, closed",
                                                trade.unix_time, trade.market, subtract_quantity, "Short", position.price, trade.price
                                            );
                                            message_position_taker(&trade, &txb3, string_m.clone());
                                            trader_info(&close_position,&config_position, &txb3 );
                                            interest_event(Utc::now().timestamp_micros(),"Short".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                            // Remove the specific position if the quantity is zero
                                            if position.position_quantity == 0 {
                                                positions.retain(|p| p.position_identifier != position_id);
                                            }
                            
                                            // If the remaining quantity is still greater than zero, proceed with the rest of the positions
                                            while remaining_quantity > 0 && !positions.is_empty() {
                                                // Take the first position in the vector
                                                if let Some(mut position) = positions.first().cloned() {
                                                    let subtract_quantity2 = remaining_quantity.min(position.position_quantity);
                                                    remaining_quantity -= subtract_quantity2;
                                                    position.position_quantity -= subtract_quantity2;

                                                    if let Some(quantity) = short_tree.interest.get_mut(&position.price) {
                                                        *quantity -= subtract_quantity2;
                                                        if *quantity == 0 {
                                                            short_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                        }
                                                    }
                            
                                                    // Create and send the ClosePositionStruct as above
                                                    let close_position = ClosePositionStruct {
                                                        market: trade.market.clone(),
                                                        broker_identifier: trade.broker_identifier_taker.clone(),
                                                        unix_time: trade.unix_time,
                                                        position_identifier: position.position_identifier,
                                                        position_identifier_closer: position_id_closer,
                                                        trader_identifier: trade.trader_identifier_taker,
                                                        position_side: OrderSide::Short,
                                                        position_quantity: subtract_quantity2,
                                                        initial_price: position.price,
                                                        closing_price: trade.price,
                                                    };
                            
                                                    let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                                    if let Err(e) = txb3.send(close_position_message) {
                                                        eprintln!("Failed to send close position message: {:?}", e);
                                                    }
                            
                                                    let string_m = format!(
                                                        "{} {}, {} {} position from price level {} to {}, closed",
                                                        trade.unix_time, trade.market, subtract_quantity2, "Short", position.price, trade.price
                                                    );
                                                    message_position_taker(&trade, &txb3, string_m.clone());
                                                    trader_info(&close_position,&config_position, &txb3 );
                                                    interest_event(Utc::now().timestamp_micros(),"Short".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                                    // Remove the position if the quantity is zero, or update it in the vector
                                                    if position.position_quantity == 0 {
                                                        positions.remove(0);
                                                    } else {
                                                        positions[0] = position;
                                                    }
                                                }
                                            }
                                               // If there's still remaining quantity, add it to the long_map as a new position
                                        if remaining_quantity > 0 {
                                            let new_position = PositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_taker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: id_i64(),
                                                trader_identifier: trade.trader_identifier_taker,
                                                position_side: OrderSide::Long,
                                                position_quantity: remaining_quantity,
                                                price: trade.price,
                                            };

                                            long_map
                                                .entry(trade.trader_identifier_taker)
                                                .or_default()
                                                .push(new_position.clone());

                                            let position_message = Structs::PositionStruct(new_position);
                                            if let Err(e) = txb3.send(position_message) {
                                                eprintln!("Failed to send new position message: {:?}", e);
                                            }

                                            let string_m = format!(
                                                "{} {}, {} {} position at price level {}, opened.",
                                                trade.unix_time, trade.market, remaining_quantity, "Long", trade.price
                                            );
                                            message_position_taker(&trade, &txb3, string_m.clone());
                                            long_tree
                                            .interest
                                            .entry(trade.price)
                                            .and_modify(|q| *q += remaining_quantity)
                                            .or_insert(remaining_quantity);
                                            interest_event(Utc::now().timestamp_micros(),"Long".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                        }

                                        // Remove the key from short_map if the positions vector is empty
                                        if positions.is_empty() {
                                            is_empty = true;
                                        }
            
        
                                        
                                    } else { // Tsy hita le position identifier
                                        let mut remaining_quantity = trade.order_quantity;

                                        // Iterate through positions to close
                                        while remaining_quantity > 0 && !positions.is_empty() {
                                            // Take the first position in the vector
                                            if let Some(mut position) = positions.first().cloned() {
                                                // Determine how much to subtract from the current position
                                                let subtract_quantity = remaining_quantity.min(position.position_quantity);
                            
                                                // Update the remaining quantity in MatchStruct
                                                remaining_quantity -= subtract_quantity;
                            
                                                // Update the position's quantity
                                                position.position_quantity -= subtract_quantity;

                                                if let Some(quantity) = short_tree.interest.get_mut(&position.price) {
                                                    *quantity -= subtract_quantity;
                                                    if *quantity == 0 {
                                                        short_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                    }
                                                }
                            
                                                // Create a ClosePositionStruct for the closed portion
                                                let close_position = ClosePositionStruct {
                                                    market: trade.market.clone(),
                                                    broker_identifier: trade.broker_identifier_taker.clone(),
                                                    unix_time: trade.unix_time,
                                                    position_identifier: position.position_identifier,
                                                    position_identifier_closer: position_id_closer, // Identifier of the closer
                                                    trader_identifier: trade.trader_identifier_taker,
                                                    position_side: OrderSide::Short,
                                                    position_quantity: subtract_quantity,
                                                    initial_price: position.price,
                                                    closing_price: trade.price,
                                                };
                            
                                                // Send the close position message
                                                let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                                if let Err(e) = txb3.send(close_position_message) {
                                                    eprintln!("Failed to send close position message: {:?}", e);
                                                }
                                                let string_m = format! ("{} {}, {} {} position from price level {} to {}, closed", trade.unix_time,trade.market,subtract_quantity,"Short",position.price, trade.price );
                                                message_position_taker(&trade,&txb3,string_m.clone());
                                                trader_info(&close_position,&config_position, &txb3 );
                                                interest_event(Utc::now().timestamp_micros(),"Short".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                                // If the position quantity is zero, remove it from the vector
                                                if position.position_quantity == 0 {
                                                    positions.remove(0);
                                                } else {
                                                    // Update the first position in the vector with the reduced quantity
                                                    positions[0] = position;
                                                }
                                            }
                                        }
                            
                                        // Check if there is still remaining quantity after exhausting all positions
                                        if remaining_quantity > 0 {
                                            // Add the remaining quantity as a new position in the long_map
                                            let new_position = PositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_taker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: id_i64(), // Generate a new unique identifier
                                                trader_identifier: trade.trader_identifier_taker,
                                                position_side: OrderSide::Long,
                                                position_quantity: remaining_quantity,
                                                price: trade.price,
                                            };
                            
                                            long_map
                                                .entry(trade.trader_identifier_taker)
                                                .or_default()
                                                .push(new_position.clone());
                            
                                            let position_message = Structs::PositionStruct(new_position);
                                            if let Err(e) = txb3.send(position_message) {
                                                eprintln!("Failed to send new position message: {:?}", e);
                                            }
                                            let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,remaining_quantity,"Long",trade.price );
                                            message_position_taker(&trade,&txb3,string_m.clone());

                                            long_tree
                                            .interest
                                            .entry(trade.price)
                                            .and_modify(|q| *q += remaining_quantity)
                                            .or_insert(remaining_quantity);

                                            interest_event(Utc::now().timestamp_micros(),"Long".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                        }
                            
                                        // Remove the key from the short_map if the positions vector is empty
                                        if positions.is_empty() {
                                            is_empty=true;
                                           
                                        }
                                    }
                                } else { // Raha None ny position identifier aker
                                    let mut remaining_quantity = trade.order_quantity;

                                    // Iterate through positions to close
                                    while remaining_quantity > 0 && !positions.is_empty() {
                                        // Take the first position in the vector
                                        if let Some(mut position) = positions.first().cloned() {
                                            // Determine how much to subtract from the current position
                                            let subtract_quantity = remaining_quantity.min(position.position_quantity);
                        
                                            // Update the remaining quantity in MatchStruct
                                            remaining_quantity -= subtract_quantity;
                        
                                            // Update the position's quantity
                                            position.position_quantity -= subtract_quantity;

                                            if let Some(quantity) = short_tree.interest.get_mut(&position.price) {
                                                *quantity -= subtract_quantity;
                                                if *quantity == 0 {
                                                    short_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                }
                                            }
                        
                                            // Create a ClosePositionStruct for the closed portion
                                            let close_position = ClosePositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_taker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: position.position_identifier,
                                                position_identifier_closer: position_id_closer, // Identifier of the closer
                                                trader_identifier: trade.trader_identifier_taker,
                                                position_side: OrderSide::Short,
                                                position_quantity: subtract_quantity,
                                                initial_price: position.price,
                                                closing_price: trade.price,
                                            };
                        
                                            // Send the close position message
                                            let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                            if let Err(e) = txb3.send(close_position_message) {
                                                eprintln!("Failed to send close position message: {:?}", e);
                                            }
                                            let string_m = format! ("{} {}, {} {} position from price level {} to {}, closed", trade.unix_time,trade.market,subtract_quantity,"Short",position.price, trade.price );
                                            message_position_taker(&trade,&txb3,string_m.clone());
                                            trader_info(&close_position,&config_position, &txb3 );
                                            interest_event(Utc::now().timestamp_micros(),"Short".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                            // If the position quantity is zero, remove it from the vector
                                            if position.position_quantity == 0 {
                                                positions.remove(0);
                                            } else {
                                                // Update the first position in the vector with the reduced quantity
                                                positions[0] = position;
                                            }
                                        }
                                    }
                        
                                    // Check if there is still remaining quantity after exhausting all positions
                                    if remaining_quantity > 0 {
                                        // Add the remaining quantity as a new position in the long_map
                                        let new_position = PositionStruct {
                                            market: trade.market.clone(),
                                            broker_identifier: trade.broker_identifier_taker.clone(),
                                            unix_time: trade.unix_time,
                                            position_identifier: id_i64(), // Generate a new unique identifier
                                            trader_identifier: trade.trader_identifier_taker,
                                            position_side: OrderSide::Long,
                                            position_quantity: remaining_quantity,
                                            price: trade.price,
                                        };
                        
                                        long_map
                                            .entry(trade.trader_identifier_taker)
                                            .or_default()
                                            .push(new_position.clone());
                        
                                        let position_message = Structs::PositionStruct(new_position);
                                        if let Err(e) = txb3.send(position_message) {
                                            eprintln!("Failed to send new position message: {:?}", e);
                                        }
                                        let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,remaining_quantity,"Long",trade.price );
                                    message_position_taker(&trade,&txb3,string_m.clone());

                                    long_tree
                                    .interest
                                    .entry(trade.price)
                                    .and_modify(|q| *q += remaining_quantity)
                                    .or_insert(remaining_quantity);

                                    interest_event(Utc::now().timestamp_micros(),"Long".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                    }
                        
                                    // Remove the key from the short_map if the positions vector is empty
                                    if positions.is_empty() {
                                        is_empty=true;
                                       
                                    }
                                }
                            }
                           
                             else { // Mbola tsy misy position open
                                // Trader identifier doesn't exist, create new in long_map
                                let new_position = PositionStruct {
                                    market: trade.market.clone(),
                                    broker_identifier: trade.broker_identifier_taker.clone(),
                                    unix_time: trade.unix_time,
                                    position_identifier: id_i64(),
                                    trader_identifier: trade.trader_identifier_taker,
                                    position_side: OrderSide::Long,
                                    position_quantity: trade.order_quantity,
                                    price: trade.price,
                                };
                                long_map.entry(trade.trader_identifier_taker).or_default().push(new_position.clone());
                                let position_message = Structs::PositionStruct(new_position);
                                    if let Err(e) = txb3.send(position_message) {
                                        eprintln!("Failed to send message: {:?}", e);
                                    }
                                    let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,trade.order_quantity,"Long",trade.price );
                                    message_position_taker(&trade,&txb3,string_m.clone());

                                    long_tree
                                    .interest
                                    .entry(trade.price)
                                    .and_modify(|q| *q += trade.order_quantity)
                                    .or_insert(trade.order_quantity);
                                
                                    interest_event(Utc::now().timestamp_micros(),"Long".to_string(),trade.order_quantity,trade.price,1,&txm2,&config_position);
                            }
                            if is_empty {
                            short_map.remove(&trade.trader_identifier_taker);
                            is_empty = false;
                            }
                        }
                        OrderSide::Short => {
                            let mut is_empty = false;
                            if let Some(mut positions) = long_map.get_mut(&trade.trader_identifier_taker) {
                                // Check pointing_at_taker if not None
                                let position_id_closer = id_i64();
                                if let Some(position_id) = trade.pointing_at_taker {
                                    // Find the specific position in the vector
                                    if let Some(position) = positions.iter_mut().find(|p| p.position_identifier == position_id) {
                                       
                                            let mut remaining_quantity = trade.order_quantity;
                
                                            // Subtract as much as possible from the specific position
                                            let subtract_quantity = remaining_quantity.min(position.position_quantity);
                                            remaining_quantity -= subtract_quantity;
                                            position.position_quantity -= subtract_quantity;

                                            if let Some(quantity) = long_tree.interest.get_mut(&position.price) {
                                                *quantity -= subtract_quantity;
                                                if *quantity == 0 {
                                                    long_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                }
                                            }
                            
                                            // Create a ClosePositionStruct for the closed portion
                                            let close_position = ClosePositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_taker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: position.position_identifier,
                                                position_identifier_closer: position_id_closer, // Identifier of the closer
                                                trader_identifier: trade.trader_identifier_taker,
                                                position_side: OrderSide::Long,
                                                position_quantity: subtract_quantity,
                                                initial_price: position.price,
                                                closing_price: trade.price,
                                            };
                            
                                            // Send the close position message
                                            let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                            if let Err(e) = txb3.send(close_position_message) {
                                                eprintln!("Failed to send close position message: {:?}", e);
                                            }
                            
                                            let string_m = format!(
                                                "{} {}, {} {} position from price level {} to {}, closed",
                                                trade.unix_time, trade.market, subtract_quantity, "Long", position.price, trade.price
                                            );
                                            message_position_taker(&trade, &txb3, string_m.clone());
                                            trader_info(&close_position,&config_position, &txb3 );
                                            interest_event(Utc::now().timestamp_micros(),"Long".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                            // Remove the specific position if the quantity is zero
                                            if position.position_quantity == 0 {
                                                positions.retain(|p| p.position_identifier != position_id);
                                            }
                            
                                            // If the remaining quantity is still greater than zero, proceed with the rest of the positions
                                            while remaining_quantity > 0 && !positions.is_empty() {
                                                // Take the first position in the vector
                                                if let Some(mut position) = positions.first().cloned() {
                                                    let subtract_quantity2 = remaining_quantity.min(position.position_quantity);
                                                    remaining_quantity -= subtract_quantity2;
                                                    position.position_quantity -= subtract_quantity2;

                                                    if let Some(quantity) = long_tree.interest.get_mut(&position.price) {
                                                        *quantity -= subtract_quantity2;
                                                        if *quantity == 0 {
                                                            long_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                        }
                                                    }
                            
                                                    // Create and send the ClosePositionStruct as above
                                                    let close_position = ClosePositionStruct {
                                                        market: trade.market.clone(),
                                                        broker_identifier: trade.broker_identifier_taker.clone(),
                                                        unix_time: trade.unix_time,
                                                        position_identifier: position.position_identifier,
                                                        position_identifier_closer: position_id_closer,
                                                        trader_identifier: trade.trader_identifier_taker,
                                                        position_side: OrderSide::Long,
                                                        position_quantity: subtract_quantity2,
                                                        initial_price: position.price,
                                                        closing_price: trade.price,
                                                    };
                            
                                                    let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                                    if let Err(e) = txb3.send(close_position_message) {
                                                        eprintln!("Failed to send close position message: {:?}", e);
                                                    }
                            
                                                    let string_m = format!(
                                                        "{} {}, {} {} position from price level {} to {}, closed",
                                                        trade.unix_time, trade.market, subtract_quantity2, "Long", position.price, trade.price
                                                    );
                                                    message_position_taker(&trade, &txb3, string_m.clone());
                                                    trader_info(&close_position,&config_position, &txb3 );
                                                    interest_event(Utc::now().timestamp_micros(),"Long".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                                    // Remove the position if the quantity is zero, or update it in the vector
                                                    if position.position_quantity == 0 {
                                                        positions.remove(0);
                                                    } else {
                                                        positions[0] = position;
                                                    }
                                                }
                                            }
                                               // If there's still remaining quantity, add it to the long_map as a new position
                                        if remaining_quantity > 0 {
                                            let new_position = PositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_taker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: id_i64(),
                                                trader_identifier: trade.trader_identifier_taker,
                                                position_side: OrderSide::Short,
                                                position_quantity: remaining_quantity,
                                                price: trade.price,
                                            };

                                            short_map
                                                .entry(trade.trader_identifier_taker)
                                                .or_default()
                                                .push(new_position.clone());

                                            let position_message = Structs::PositionStruct(new_position);
                                            if let Err(e) = txb3.send(position_message) {
                                                eprintln!("Failed to send new position message: {:?}", e);
                                            }

                                            let string_m = format!(
                                                "{} {}, {} {} position at price level {}, opened.",
                                                trade.unix_time, trade.market, remaining_quantity, "Short", trade.price
                                            );
                                            message_position_taker(&trade, &txb3, string_m.clone());
                                            short_tree
                                            .interest
                                            .entry(trade.price)
                                            .and_modify(|q| *q += remaining_quantity)
                                            .or_insert(remaining_quantity);
                                            interest_event(Utc::now().timestamp_micros(),"Short".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                        }

                                        // Remove the key from short_map if the positions vector is empty
                                        if positions.is_empty() {
                                            is_empty = true;
                                        }
            
        
                                        
                                    } else { // Tsy hita le position identifier
                                        let mut remaining_quantity = trade.order_quantity;

                                        // Iterate through positions to close
                                        while remaining_quantity > 0 && !positions.is_empty() {
                                            // Take the first position in the vector
                                            if let Some(mut position) = positions.first().cloned() {
                                                // Determine how much to subtract from the current position
                                                let subtract_quantity = remaining_quantity.min(position.position_quantity);
                            
                                                // Update the remaining quantity in MatchStruct
                                                remaining_quantity -= subtract_quantity;
                            
                                                // Update the position's quantity
                                                position.position_quantity -= subtract_quantity;

                                                if let Some(quantity) = long_tree.interest.get_mut(&position.price) {
                                                    *quantity -= subtract_quantity;
                                                    if *quantity == 0 {
                                                        long_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                    }
                                                }
                            
                                                // Create a ClosePositionStruct for the closed portion
                                                let close_position = ClosePositionStruct {
                                                    market: trade.market.clone(),
                                                    broker_identifier: trade.broker_identifier_taker.clone(),
                                                    unix_time: trade.unix_time,
                                                    position_identifier: position.position_identifier,
                                                    position_identifier_closer: position_id_closer, // Identifier of the closer
                                                    trader_identifier: trade.trader_identifier_taker,
                                                    position_side: OrderSide::Long,
                                                    position_quantity: subtract_quantity,
                                                    initial_price: position.price,
                                                    closing_price: trade.price,
                                                };
                            
                                                // Send the close position message
                                                let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                                if let Err(e) = txb3.send(close_position_message) {
                                                    eprintln!("Failed to send close position message: {:?}", e);
                                                }
                                                let string_m = format! ("{} {}, {} {} position from price level {} to {}, closed", trade.unix_time,trade.market,subtract_quantity,"Long",position.price, trade.price );
                                                message_position_taker(&trade,&txb3,string_m.clone());
                                                trader_info(&close_position,&config_position, &txb3 );
                                                interest_event(Utc::now().timestamp_micros(),"Long".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                                // If the position quantity is zero, remove it from the vector
                                                if position.position_quantity == 0 {
                                                    positions.remove(0);
                                                } else {
                                                    // Update the first position in the vector with the reduced quantity
                                                    positions[0] = position;
                                                }
                                            }
                                        }
                            
                                        // Check if there is still remaining quantity after exhausting all positions
                                        if remaining_quantity > 0 {
                                            // Add the remaining quantity as a new position in the long_map
                                            let new_position = PositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_taker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: id_i64(), // Generate a new unique identifier
                                                trader_identifier: trade.trader_identifier_taker,
                                                position_side: OrderSide::Short,
                                                position_quantity: remaining_quantity,
                                                price: trade.price,
                                            };
                            
                                            short_map
                                                .entry(trade.trader_identifier_taker)
                                                .or_default()
                                                .push(new_position.clone());
                            
                                            let position_message = Structs::PositionStruct(new_position);
                                            if let Err(e) = txb3.send(position_message) {
                                                eprintln!("Failed to send new position message: {:?}", e);
                                            }
                                            let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,remaining_quantity,"Short",trade.price );
                                            message_position_taker(&trade,&txb3,string_m.clone());

                                            short_tree
                                            .interest
                                            .entry(trade.price)
                                            .and_modify(|q| *q += remaining_quantity)
                                            .or_insert(remaining_quantity);

                                            interest_event(Utc::now().timestamp_micros(),"Short".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                        }
                            
                                        // Remove the key from the short_map if the positions vector is empty
                                        if positions.is_empty() {
                                            is_empty=true;
                                           
                                        }
                                    }
                                } else { // Raha None ny position identifier aker
                                    let mut remaining_quantity = trade.order_quantity;

                                    // Iterate through positions to close
                                    while remaining_quantity > 0 && !positions.is_empty() {
                                        // Take the first position in the vector
                                        if let Some(mut position) = positions.first().cloned() {
                                            // Determine how much to subtract from the current position
                                            let subtract_quantity = remaining_quantity.min(position.position_quantity);
                        
                                            // Update the remaining quantity in MatchStruct
                                            remaining_quantity -= subtract_quantity;
                        
                                            // Update the position's quantity
                                            position.position_quantity -= subtract_quantity;

                                            if let Some(quantity) = long_tree.interest.get_mut(&position.price) {
                                                *quantity -= subtract_quantity;
                                                if *quantity == 0 {
                                                    long_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                }
                                            }
                        
                                            // Create a ClosePositionStruct for the closed portion
                                            let close_position = ClosePositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_taker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: position.position_identifier,
                                                position_identifier_closer: position_id_closer, // Identifier of the closer
                                                trader_identifier: trade.trader_identifier_taker,
                                                position_side: OrderSide::Long,
                                                position_quantity: subtract_quantity,
                                                initial_price: position.price,
                                                closing_price: trade.price,
                                            };
                        
                                            // Send the close position message
                                            let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                            if let Err(e) = txb3.send(close_position_message) {
                                                eprintln!("Failed to send close position message: {:?}", e);
                                            }
                                            let string_m = format! ("{} {}, {} {} position from price level {} to {}, closed", trade.unix_time,trade.market,subtract_quantity,"Long",position.price, trade.price );
                                            message_position_taker(&trade,&txb3,string_m.clone());
                                            trader_info(&close_position,&config_position, &txb3 );
                                            interest_event(Utc::now().timestamp_micros(),"Long".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                            // If the position quantity is zero, remove it from the vector
                                            if position.position_quantity == 0 {
                                                positions.remove(0);
                                            } else {
                                                // Update the first position in the vector with the reduced quantity
                                                positions[0] = position;
                                            }
                                        }
                                    }
                        
                                    // Check if there is still remaining quantity after exhausting all positions
                                    if remaining_quantity > 0 {
                                        // Add the remaining quantity as a new position in the long_map
                                        let new_position = PositionStruct {
                                            market: trade.market.clone(),
                                            broker_identifier: trade.broker_identifier_taker.clone(),
                                            unix_time: trade.unix_time,
                                            position_identifier: id_i64(), // Generate a new unique identifier
                                            trader_identifier: trade.trader_identifier_taker,
                                            position_side: OrderSide::Short,
                                            position_quantity: remaining_quantity,
                                            price: trade.price,
                                        };
                        
                                        short_map
                                            .entry(trade.trader_identifier_taker)
                                            .or_default()
                                            .push(new_position.clone());
                        
                                        let position_message = Structs::PositionStruct(new_position);
                                        if let Err(e) = txb3.send(position_message) {
                                            eprintln!("Failed to send new position message: {:?}", e);
                                        }
                                        let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,remaining_quantity,"Short",trade.price );
                                    message_position_taker(&trade,&txb3,string_m.clone());

                                    short_tree
                                    .interest
                                    .entry(trade.price)
                                    .and_modify(|q| *q += remaining_quantity)
                                    .or_insert(remaining_quantity);

                                    interest_event(Utc::now().timestamp_micros(),"Short".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                    }
                        
                                    // Remove the key from the short_map if the positions vector is empty
                                    if positions.is_empty() {
                                        is_empty=true;
                                       
                                    }
                                }
                            }
                           
                             else { // Mbola tsy misy position open
                                // Trader identifier doesn't exist, create new in long_map
                                let new_position = PositionStruct {
                                    market: trade.market.clone(),
                                    broker_identifier: trade.broker_identifier_taker.clone(),
                                    unix_time: trade.unix_time,
                                    position_identifier: id_i64(),
                                    trader_identifier: trade.trader_identifier_taker,
                                    position_side: OrderSide::Short,
                                    position_quantity: trade.order_quantity,
                                    price: trade.price,
                                };
                                short_map.entry(trade.trader_identifier_taker).or_default().push(new_position.clone());
                                let position_message = Structs::PositionStruct(new_position);
                                    if let Err(e) = txb3.send(position_message) {
                                        eprintln!("Failed to send message: {:?}", e);
                                    }
                                    let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,trade.order_quantity,"Short",trade.price );
                                    message_position_taker(&trade,&txb3,string_m.clone());

                                    short_tree
                                    .interest
                                    .entry(trade.price)
                                    .and_modify(|q| *q += trade.order_quantity)
                                    .or_insert(trade.order_quantity);
                                
                                    interest_event(Utc::now().timestamp_micros(),"Short".to_string(),trade.order_quantity,trade.price,1,&txm2,&config_position);
                            }
                            if is_empty {
                            long_map.remove(&trade.trader_identifier_taker);
                            is_empty = false;
                            }
                        }
                        OrderSide::Unspecified => {
                        }
                    }

                    // Handle Maker Side based on position side
                    match trade.maker_position_side {
                        OrderSide::Long => {
                            // Check inside short_map for the trader_identifier_maker.
                            let mut is_empty = false;
                            if let Some(mut positions) = short_map.get_mut(&trade.trader_identifier_maker) {
                                // Check pointing_at_maker if not None
                                let position_id_closer = id_i64();
                                if let Some(position_id) = trade.pointing_at_maker {
                                    // Find the specific position in the vector
                                    if let Some(position) = positions.iter_mut().find(|p| p.position_identifier == position_id) {
                                       
                                            let mut remaining_quantity = trade.order_quantity;
                
                                            // Subtract as much as possible from the specific position
                                            let subtract_quantity = remaining_quantity.min(position.position_quantity);
                                            remaining_quantity -= subtract_quantity;
                                            position.position_quantity -= subtract_quantity;

                                            if let Some(quantity) = short_tree.interest.get_mut(&position.price) {
                                                *quantity -= subtract_quantity;
                                                if *quantity == 0 {
                                                    short_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                }
                                            }
                            
                                            // Create a ClosePositionStruct for the closed portion
                                            let close_position = ClosePositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_maker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: position.position_identifier,
                                                position_identifier_closer: position_id_closer, // Identifier of the closer
                                                trader_identifier: trade.trader_identifier_maker,
                                                position_side: OrderSide::Short,
                                                position_quantity: subtract_quantity,
                                                initial_price: position.price,
                                                closing_price: trade.price,
                                            };
                            
                                            // Send the close position message
                                            let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                            if let Err(e) = txb3.send(close_position_message) {
                                                eprintln!("Failed to send close position message: {:?}", e);
                                            }
                            
                                            let string_m = format!(
                                                "{} {}, {} {} position from price level {} to {}, closed",
                                                trade.unix_time, trade.market, subtract_quantity, "Short", position.price, trade.price
                                            );
                                            message_position_maker(&trade, &txb3, string_m.clone());
                                            trader_info(&close_position,&config_position, &txb3 );
                                            interest_event(Utc::now().timestamp_micros(),"Short".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                            // Remove the specific position if the quantity is zero
                                            if position.position_quantity == 0 {
                                                positions.retain(|p| p.position_identifier != position_id);
                                            }
                            
                                            // If the remaining quantity is still greater than zero, proceed with the rest of the positions
                                            while remaining_quantity > 0 && !positions.is_empty() {
                                                // Take the first position in the vector
                                                if let Some(mut position) = positions.first().cloned() {
                                                    let subtract_quantity2 = remaining_quantity.min(position.position_quantity);
                                                    remaining_quantity -= subtract_quantity2;
                                                    position.position_quantity -= subtract_quantity2;

                                                    if let Some(quantity) = short_tree.interest.get_mut(&position.price) {
                                                        *quantity -= subtract_quantity2;
                                                        if *quantity == 0 {
                                                            short_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                        }
                                                    }
                            
                                                    // Create and send the ClosePositionStruct as above
                                                    let close_position = ClosePositionStruct {
                                                        market: trade.market.clone(),
                                                        broker_identifier: trade.broker_identifier_maker.clone(),
                                                        unix_time: trade.unix_time,
                                                        position_identifier: position.position_identifier,
                                                        position_identifier_closer: position_id_closer,
                                                        trader_identifier: trade.trader_identifier_maker,
                                                        position_side: OrderSide::Short,
                                                        position_quantity: subtract_quantity2,
                                                        initial_price: position.price,
                                                        closing_price: trade.price,
                                                    };
                            
                                                    let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                                    if let Err(e) = txb3.send(close_position_message) {
                                                        eprintln!("Failed to send close position message: {:?}", e);
                                                    }
                            
                                                    let string_m = format!(
                                                        "{} {}, {} {} position from price level {} to {}, closed",
                                                        trade.unix_time, trade.market, subtract_quantity2, "Short", position.price, trade.price
                                                    );
                                                    message_position_maker(&trade, &txb3, string_m.clone());
                                                    trader_info(&close_position,&config_position, &txb3 );
                                                    interest_event(Utc::now().timestamp_micros(),"Short".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                                    // Remove the position if the quantity is zero, or update it in the vector
                                                    if position.position_quantity == 0 {
                                                        positions.remove(0);
                                                    } else {
                                                        positions[0] = position;
                                                    }
                                                }
                                            }
                                               // If there's still remaining quantity, add it to the long_map as a new position
                                        if remaining_quantity > 0 {
                                            let new_position = PositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_maker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: id_i64(),
                                                trader_identifier: trade.trader_identifier_maker,
                                                position_side: OrderSide::Long,
                                                position_quantity: remaining_quantity,
                                                price: trade.price,
                                            };

                                            long_map
                                                .entry(trade.trader_identifier_maker)
                                                .or_default()
                                                .push(new_position.clone());

                                            let position_message = Structs::PositionStruct(new_position);
                                            if let Err(e) = txb3.send(position_message) {
                                                eprintln!("Failed to send new position message: {:?}", e);
                                            }

                                            let string_m = format!(
                                                "{} {}, {} {} position at price level {}, opened.",
                                                trade.unix_time, trade.market, remaining_quantity, "Long", trade.price
                                            );
                                            message_position_maker(&trade, &txb3, string_m.clone());
                                            long_tree
                                            .interest
                                            .entry(trade.price)
                                            .and_modify(|q| *q += remaining_quantity)
                                            .or_insert(remaining_quantity);
                                            interest_event(Utc::now().timestamp_micros(),"Long".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                        }

                                        // Remove the key from short_map if the positions vector is empty
                                        if positions.is_empty() {
                                            is_empty = true;
                                        }
            
        
                                        
                                    } else { // Tsy hita le position identifier
                                        let mut remaining_quantity = trade.order_quantity;

                                        // Iterate through positions to close
                                        while remaining_quantity > 0 && !positions.is_empty() {
                                            // Take the first position in the vector
                                            if let Some(mut position) = positions.first().cloned() {
                                                // Determine how much to subtract from the current position
                                                let subtract_quantity = remaining_quantity.min(position.position_quantity);
                            
                                                // Update the remaining quantity in MatchStruct
                                                remaining_quantity -= subtract_quantity;
                            
                                                // Update the position's quantity
                                                position.position_quantity -= subtract_quantity;

                                                if let Some(quantity) = short_tree.interest.get_mut(&position.price) {
                                                    *quantity -= subtract_quantity;
                                                    if *quantity == 0 {
                                                        short_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                    }
                                                }
                            
                                                // Create a ClosePositionStruct for the closed portion
                                                let close_position = ClosePositionStruct {
                                                    market: trade.market.clone(),
                                                    broker_identifier: trade.broker_identifier_maker.clone(),
                                                    unix_time: trade.unix_time,
                                                    position_identifier: position.position_identifier,
                                                    position_identifier_closer: position_id_closer, // Identifier of the closer
                                                    trader_identifier: trade.trader_identifier_maker,
                                                    position_side: OrderSide::Short,
                                                    position_quantity: subtract_quantity,
                                                    initial_price: position.price,
                                                    closing_price: trade.price,
                                                };
                            
                                                // Send the close position message
                                                let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                                if let Err(e) = txb3.send(close_position_message) {
                                                    eprintln!("Failed to send close position message: {:?}", e);
                                                }
                                                let string_m = format! ("{} {}, {} {} position from price level {} to {}, closed", trade.unix_time,trade.market,subtract_quantity,"Short",position.price, trade.price );
                                                message_position_maker(&trade,&txb3,string_m.clone());
                                                trader_info(&close_position,&config_position, &txb3 );
                                                interest_event(Utc::now().timestamp_micros(),"Short".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                                // If the position quantity is zero, remove it from the vector
                                                if position.position_quantity == 0 {
                                                    positions.remove(0);
                                                } else {
                                                    // Update the first position in the vector with the reduced quantity
                                                    positions[0] = position;
                                                }
                                            }
                                        }
                            
                                        // Check if there is still remaining quantity after exhausting all positions
                                        if remaining_quantity > 0 {
                                            // Add the remaining quantity as a new position in the long_map
                                            let new_position = PositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_maker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: id_i64(), // Generate a new unique identifier
                                                trader_identifier: trade.trader_identifier_maker,
                                                position_side: OrderSide::Long,
                                                position_quantity: remaining_quantity,
                                                price: trade.price,
                                            };
                            
                                            long_map
                                                .entry(trade.trader_identifier_maker)
                                                .or_default()
                                                .push(new_position.clone());
                            
                                            let position_message = Structs::PositionStruct(new_position);
                                            if let Err(e) = txb3.send(position_message) {
                                                eprintln!("Failed to send new position message: {:?}", e);
                                            }
                                            let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,remaining_quantity,"Long",trade.price );
                                            message_position_maker(&trade,&txb3,string_m.clone());

                                            long_tree
                                            .interest
                                            .entry(trade.price)
                                            .and_modify(|q| *q += remaining_quantity)
                                            .or_insert(remaining_quantity);

                                            interest_event(Utc::now().timestamp_micros(),"Long".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                        }
                            
                                        // Remove the key from the short_map if the positions vector is empty
                                        if positions.is_empty() {
                                            is_empty=true;
                                           
                                        }
                                    }
                                } else { // Raha None ny position identifier aker
                                    let mut remaining_quantity = trade.order_quantity;

                                    // Iterate through positions to close
                                    while remaining_quantity > 0 && !positions.is_empty() {
                                        // Take the first position in the vector
                                        if let Some(mut position) = positions.first().cloned() {
                                            // Determine how much to subtract from the current position
                                            let subtract_quantity = remaining_quantity.min(position.position_quantity);
                        
                                            // Update the remaining quantity in MatchStruct
                                            remaining_quantity -= subtract_quantity;
                        
                                            // Update the position's quantity
                                            position.position_quantity -= subtract_quantity;

                                            if let Some(quantity) = short_tree.interest.get_mut(&position.price) {
                                                *quantity -= subtract_quantity;
                                                if *quantity == 0 {
                                                    short_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                }
                                            }
                        
                                            // Create a ClosePositionStruct for the closed portion
                                            let close_position = ClosePositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_maker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: position.position_identifier,
                                                position_identifier_closer: position_id_closer, // Identifier of the closer
                                                trader_identifier: trade.trader_identifier_maker,
                                                position_side: OrderSide::Short,
                                                position_quantity: subtract_quantity,
                                                initial_price: position.price,
                                                closing_price: trade.price,
                                            };
                        
                                            // Send the close position message
                                            let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                            if let Err(e) = txb3.send(close_position_message) {
                                                eprintln!("Failed to send close position message: {:?}", e);
                                            }
                                            let string_m = format! ("{} {}, {} {} position from price level {} to {}, closed", trade.unix_time,trade.market,subtract_quantity,"Short",position.price, trade.price );
                                            message_position_maker(&trade,&txb3,string_m.clone());
                                            trader_info(&close_position,&config_position, &txb3 );
                                            interest_event(Utc::now().timestamp_micros(),"Short".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                            // If the position quantity is zero, remove it from the vector
                                            if position.position_quantity == 0 {
                                                positions.remove(0);
                                            } else {
                                                // Update the first position in the vector with the reduced quantity
                                                positions[0] = position;
                                            }
                                        }
                                    }
                        
                                    // Check if there is still remaining quantity after exhausting all positions
                                    if remaining_quantity > 0 {
                                        // Add the remaining quantity as a new position in the long_map
                                        let new_position = PositionStruct {
                                            market: trade.market.clone(),
                                            broker_identifier: trade.broker_identifier_maker.clone(),
                                            unix_time: trade.unix_time,
                                            position_identifier: id_i64(), // Generate a new unique identifier
                                            trader_identifier: trade.trader_identifier_maker,
                                            position_side: OrderSide::Long,
                                            position_quantity: remaining_quantity,
                                            price: trade.price,
                                        };
                        
                                        long_map
                                            .entry(trade.trader_identifier_maker)
                                            .or_default()
                                            .push(new_position.clone());
                        
                                        let position_message = Structs::PositionStruct(new_position);
                                        if let Err(e) = txb3.send(position_message) {
                                            eprintln!("Failed to send new position message: {:?}", e);
                                        }
                                        let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,remaining_quantity,"Long",trade.price );
                                    message_position_maker(&trade,&txb3,string_m.clone());

                                    long_tree
                                    .interest
                                    .entry(trade.price)
                                    .and_modify(|q| *q += remaining_quantity)
                                    .or_insert(remaining_quantity);

                                    interest_event(Utc::now().timestamp_micros(),"Long".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                    }
                        
                                    // Remove the key from the short_map if the positions vector is empty
                                    if positions.is_empty() {
                                        is_empty=true;
                                       
                                    }
                                }
                            }
                           
                             else { // Mbola tsy misy position open
                                // Trader identifier doesn't exist, create new in long_map
                                let new_position = PositionStruct {
                                    market: trade.market.clone(),
                                    broker_identifier: trade.broker_identifier_maker.clone(),
                                    unix_time: trade.unix_time,
                                    position_identifier: id_i64(),
                                    trader_identifier: trade.trader_identifier_maker,
                                    position_side: OrderSide::Long,
                                    position_quantity: trade.order_quantity,
                                    price: trade.price,
                                };
                                long_map.entry(trade.trader_identifier_maker).or_default().push(new_position.clone());
                                let position_message = Structs::PositionStruct(new_position);
                                    if let Err(e) = txb3.send(position_message) {
                                        eprintln!("Failed to send message: {:?}", e);
                                    }
                                    let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,trade.order_quantity,"Long",trade.price );
                                    message_position_maker(&trade,&txb3,string_m.clone());

                                    long_tree
                                    .interest
                                    .entry(trade.price)
                                    .and_modify(|q| *q += trade.order_quantity)
                                    .or_insert(trade.order_quantity);
                                
                                    interest_event(Utc::now().timestamp_micros(),"Long".to_string(),trade.order_quantity,trade.price,1,&txm2,&config_position);
                            }
                            if is_empty {
                            short_map.remove(&trade.trader_identifier_maker);
                            is_empty = false;
                            }
                        }
                        OrderSide::Short => {
                            let mut is_empty = false;
                            if let Some(mut positions) = long_map.get_mut(&trade.trader_identifier_maker) {
                                // Check pointing_at_maker if not None
                                let position_id_closer = id_i64();
                                if let Some(position_id) = trade.pointing_at_maker {
                                    // Find the specific position in the vector
                                    if let Some(position) = positions.iter_mut().find(|p| p.position_identifier == position_id) {
                                       
                                            let mut remaining_quantity = trade.order_quantity;
                
                                            // Subtract as much as possible from the specific position
                                            let subtract_quantity = remaining_quantity.min(position.position_quantity);
                                            remaining_quantity -= subtract_quantity;
                                            position.position_quantity -= subtract_quantity;

                                            if let Some(quantity) = long_tree.interest.get_mut(&position.price) {
                                                *quantity -= subtract_quantity;
                                                if *quantity == 0 {
                                                    long_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                }
                                            }
                            
                                            // Create a ClosePositionStruct for the closed portion
                                            let close_position = ClosePositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_maker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: position.position_identifier,
                                                position_identifier_closer: position_id_closer, // Identifier of the closer
                                                trader_identifier: trade.trader_identifier_maker,
                                                position_side: OrderSide::Long,
                                                position_quantity: subtract_quantity,
                                                initial_price: position.price,
                                                closing_price: trade.price,
                                            };
                            
                                            // Send the close position message
                                            let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                            if let Err(e) = txb3.send(close_position_message) {
                                                eprintln!("Failed to send close position message: {:?}", e);
                                            }
                            
                                            let string_m = format!(
                                                "{} {}, {} {} position from price level {} to {}, closed",
                                                trade.unix_time, trade.market, subtract_quantity, "Long", position.price, trade.price
                                            );
                                            message_position_maker(&trade, &txb3, string_m.clone());
                                            trader_info(&close_position,&config_position, &txb3 );
                                            interest_event(Utc::now().timestamp_micros(),"Long".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                            // Remove the specific position if the quantity is zero
                                            if position.position_quantity == 0 {
                                                positions.retain(|p| p.position_identifier != position_id);
                                            }
                            
                                            // If the remaining quantity is still greater than zero, proceed with the rest of the positions
                                            while remaining_quantity > 0 && !positions.is_empty() {
                                                // Take the first position in the vector
                                                if let Some(mut position) = positions.first().cloned() {
                                                    let subtract_quantity2 = remaining_quantity.min(position.position_quantity);
                                                    remaining_quantity -= subtract_quantity2;
                                                    position.position_quantity -= subtract_quantity2;

                                                    if let Some(quantity) = long_tree.interest.get_mut(&position.price) {
                                                        *quantity -= subtract_quantity2;
                                                        if *quantity == 0 {
                                                            long_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                        }
                                                    }
                            
                                                    // Create and send the ClosePositionStruct as above
                                                    let close_position = ClosePositionStruct {
                                                        market: trade.market.clone(),
                                                        broker_identifier: trade.broker_identifier_maker.clone(),
                                                        unix_time: trade.unix_time,
                                                        position_identifier: position.position_identifier,
                                                        position_identifier_closer: position_id_closer,
                                                        trader_identifier: trade.trader_identifier_maker,
                                                        position_side: OrderSide::Long,
                                                        position_quantity: subtract_quantity2,
                                                        initial_price: position.price,
                                                        closing_price: trade.price,
                                                    };
                            
                                                    let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                                    if let Err(e) = txb3.send(close_position_message) {
                                                        eprintln!("Failed to send close position message: {:?}", e);
                                                    }
                            
                                                    let string_m = format!(
                                                        "{} {}, {} {} position from price level {} to {}, closed",
                                                        trade.unix_time, trade.market, subtract_quantity2, "Long", position.price, trade.price
                                                    );
                                                    message_position_maker(&trade, &txb3, string_m.clone());
                                                    trader_info(&close_position,&config_position, &txb3 );
                                                    interest_event(Utc::now().timestamp_micros(),"Long".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                                    // Remove the position if the quantity is zero, or update it in the vector
                                                    if position.position_quantity == 0 {
                                                        positions.remove(0);
                                                    } else {
                                                        positions[0] = position;
                                                    }
                                                }
                                            }
                                               // If there's still remaining quantity, add it to the long_map as a new position
                                        if remaining_quantity > 0 {
                                            let new_position = PositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_maker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: id_i64(),
                                                trader_identifier: trade.trader_identifier_maker,
                                                position_side: OrderSide::Short,
                                                position_quantity: remaining_quantity,
                                                price: trade.price,
                                            };

                                            short_map
                                                .entry(trade.trader_identifier_maker)
                                                .or_default()
                                                .push(new_position.clone());

                                            let position_message = Structs::PositionStruct(new_position);
                                            if let Err(e) = txb3.send(position_message) {
                                                eprintln!("Failed to send new position message: {:?}", e);
                                            }

                                            let string_m = format!(
                                                "{} {}, {} {} position at price level {}, opened.",
                                                trade.unix_time, trade.market, remaining_quantity, "Short", trade.price
                                            );
                                            message_position_maker(&trade, &txb3, string_m.clone());
                                            short_tree
                                            .interest
                                            .entry(trade.price)
                                            .and_modify(|q| *q += remaining_quantity)
                                            .or_insert(remaining_quantity);
                                            interest_event(Utc::now().timestamp_micros(),"Short".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                        }

                                        // Remove the key from short_map if the positions vector is empty
                                        if positions.is_empty() {
                                            is_empty = true;
                                        }
            
        
                                        
                                    } else { // Tsy hita le position identifier
                                        let mut remaining_quantity = trade.order_quantity;

                                        // Iterate through positions to close
                                        while remaining_quantity > 0 && !positions.is_empty() {
                                            // Take the first position in the vector
                                            if let Some(mut position) = positions.first().cloned() {
                                                // Determine how much to subtract from the current position
                                                let subtract_quantity = remaining_quantity.min(position.position_quantity);
                            
                                                // Update the remaining quantity in MatchStruct
                                                remaining_quantity -= subtract_quantity;
                            
                                                // Update the position's quantity
                                                position.position_quantity -= subtract_quantity;

                                                if let Some(quantity) = long_tree.interest.get_mut(&position.price) {
                                                    *quantity -= subtract_quantity;
                                                    if *quantity == 0 {
                                                        long_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                    }
                                                }
                            
                                                // Create a ClosePositionStruct for the closed portion
                                                let close_position = ClosePositionStruct {
                                                    market: trade.market.clone(),
                                                    broker_identifier: trade.broker_identifier_maker.clone(),
                                                    unix_time: trade.unix_time,
                                                    position_identifier: position.position_identifier,
                                                    position_identifier_closer: position_id_closer, // Identifier of the closer
                                                    trader_identifier: trade.trader_identifier_maker,
                                                    position_side: OrderSide::Long,
                                                    position_quantity: subtract_quantity,
                                                    initial_price: position.price,
                                                    closing_price: trade.price,
                                                };
                            
                                                // Send the close position message
                                                let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                                if let Err(e) = txb3.send(close_position_message) {
                                                    eprintln!("Failed to send close position message: {:?}", e);
                                                }
                                                let string_m = format! ("{} {}, {} {} position from price level {} to {}, closed", trade.unix_time,trade.market,subtract_quantity,"Long",position.price, trade.price );
                                                message_position_maker(&trade,&txb3,string_m.clone());
                                                trader_info(&close_position,&config_position, &txb3 );
                                                interest_event(Utc::now().timestamp_micros(),"Long".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                                // If the position quantity is zero, remove it from the vector
                                                if position.position_quantity == 0 {
                                                    positions.remove(0);
                                                } else {
                                                    // Update the first position in the vector with the reduced quantity
                                                    positions[0] = position;
                                                }
                                            }
                                        }
                            
                                        // Check if there is still remaining quantity after exhausting all positions
                                        if remaining_quantity > 0 {
                                            // Add the remaining quantity as a new position in the long_map
                                            let new_position = PositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_maker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: id_i64(), // Generate a new unique identifier
                                                trader_identifier: trade.trader_identifier_maker,
                                                position_side: OrderSide::Short,
                                                position_quantity: remaining_quantity,
                                                price: trade.price,
                                            };
                            
                                            short_map
                                                .entry(trade.trader_identifier_maker)
                                                .or_default()
                                                .push(new_position.clone());
                            
                                            let position_message = Structs::PositionStruct(new_position);
                                            if let Err(e) = txb3.send(position_message) {
                                                eprintln!("Failed to send new position message: {:?}", e);
                                            }
                                            let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,remaining_quantity,"Short",trade.price );
                                            message_position_maker(&trade,&txb3,string_m.clone());

                                            short_tree
                                            .interest
                                            .entry(trade.price)
                                            .and_modify(|q| *q += remaining_quantity)
                                            .or_insert(remaining_quantity);

                                            interest_event(Utc::now().timestamp_micros(),"Short".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                        }
                            
                                        // Remove the key from the short_map if the positions vector is empty
                                        if positions.is_empty() {
                                            is_empty=true;
                                           
                                        }
                                    }
                                } else { // Raha None ny position identifier aker
                                    let mut remaining_quantity = trade.order_quantity;

                                    // Iterate through positions to close
                                    while remaining_quantity > 0 && !positions.is_empty() {
                                        // Take the first position in the vector
                                        if let Some(mut position) = positions.first().cloned() {
                                            // Determine how much to subtract from the current position
                                            let subtract_quantity = remaining_quantity.min(position.position_quantity);
                        
                                            // Update the remaining quantity in MatchStruct
                                            remaining_quantity -= subtract_quantity;
                        
                                            // Update the position's quantity
                                            position.position_quantity -= subtract_quantity;

                                            if let Some(quantity) = long_tree.interest.get_mut(&position.price) {
                                                *quantity -= subtract_quantity;
                                                if *quantity == 0 {
                                                    long_tree.interest.remove(&position.price);  // Remove the price level if quantity is zero
                                                }
                                            }
                        
                                            // Create a ClosePositionStruct for the closed portion
                                            let close_position = ClosePositionStruct {
                                                market: trade.market.clone(),
                                                broker_identifier: trade.broker_identifier_maker.clone(),
                                                unix_time: trade.unix_time,
                                                position_identifier: position.position_identifier,
                                                position_identifier_closer: position_id_closer, // Identifier of the closer
                                                trader_identifier: trade.trader_identifier_maker,
                                                position_side: OrderSide::Long,
                                                position_quantity: subtract_quantity,
                                                initial_price: position.price,
                                                closing_price: trade.price,
                                            };
                        
                                            // Send the close position message
                                            let close_position_message = Structs::ClosePositionStruct(close_position.clone());
                                            if let Err(e) = txb3.send(close_position_message) {
                                                eprintln!("Failed to send close position message: {:?}", e);
                                            }
                                            let string_m = format! ("{} {}, {} {} position from price level {} to {}, closed", trade.unix_time,trade.market,subtract_quantity,"Long",position.price, trade.price );
                                            message_position_maker(&trade,&txb3,string_m.clone());
                                            trader_info(&close_position,&config_position, &txb3 );
                                            interest_event(Utc::now().timestamp_micros(),"Long".to_string(),subtract_quantity,position.price,-1,&txm2,&config_position);
                                            // If the position quantity is zero, remove it from the vector
                                            if position.position_quantity == 0 {
                                                positions.remove(0);
                                            } else {
                                                // Update the first position in the vector with the reduced quantity
                                                positions[0] = position;
                                            }
                                        }
                                    }
                        
                                    // Check if there is still remaining quantity after exhausting all positions
                                    if remaining_quantity > 0 {
                                        // Add the remaining quantity as a new position in the long_map
                                        let new_position = PositionStruct {
                                            market: trade.market.clone(),
                                            broker_identifier: trade.broker_identifier_maker.clone(),
                                            unix_time: trade.unix_time,
                                            position_identifier: id_i64(), // Generate a new unique identifier
                                            trader_identifier: trade.trader_identifier_maker,
                                            position_side: OrderSide::Short,
                                            position_quantity: remaining_quantity,
                                            price: trade.price,
                                        };
                        
                                        short_map
                                            .entry(trade.trader_identifier_maker)
                                            .or_default()
                                            .push(new_position.clone());
                        
                                        let position_message = Structs::PositionStruct(new_position);
                                        if let Err(e) = txb3.send(position_message) {
                                            eprintln!("Failed to send new position message: {:?}", e);
                                        }
                                        let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,remaining_quantity,"Short",trade.price );
                                    message_position_maker(&trade,&txb3,string_m.clone());

                                    short_tree
                                    .interest
                                    .entry(trade.price)
                                    .and_modify(|q| *q += remaining_quantity)
                                    .or_insert(remaining_quantity);

                                    interest_event(Utc::now().timestamp_micros(),"Short".to_string(),remaining_quantity,trade.price,1,&txm2,&config_position);
                                    }
                        
                                    // Remove the key from the short_map if the positions vector is empty
                                    if positions.is_empty() {
                                        is_empty=true;
                                       
                                    }
                                }
                            }
                           
                             else { // Mbola tsy misy position open
                                // Trader identifier doesn't exist, create new in long_map
                                let new_position = PositionStruct {
                                    market: trade.market.clone(),
                                    broker_identifier: trade.broker_identifier_maker.clone(),
                                    unix_time: trade.unix_time,
                                    position_identifier: id_i64(),
                                    trader_identifier: trade.trader_identifier_maker,
                                    position_side: OrderSide::Short,
                                    position_quantity: trade.order_quantity,
                                    price: trade.price,
                                };
                                short_map.entry(trade.trader_identifier_maker).or_default().push(new_position.clone());
                                let position_message = Structs::PositionStruct(new_position);
                                    if let Err(e) = txb3.send(position_message) {
                                        eprintln!("Failed to send message: {:?}", e);
                                    }
                                    let string_m = format! ("{} {}, {} {} position at price level {}, opened.", trade.unix_time,trade.market,trade.order_quantity,"Short",trade.price );
                                    message_position_maker(&trade,&txb3,string_m.clone());

                                    short_tree
                                    .interest
                                    .entry(trade.price)
                                    .and_modify(|q| *q += trade.order_quantity)
                                    .or_insert(trade.order_quantity);
                                
                                    interest_event(Utc::now().timestamp_micros(),"Short".to_string(),trade.order_quantity,trade.price,1,&txm2,&config_position);
                            }
                            if is_empty {
                            long_map.remove(&trade.trader_identifier_maker);
                            is_empty = false;
                            }
                        }
                        OrderSide::Unspecified => {
                        }
                    }
                    
                }
                Structs::Save(_) => {
                    if let Err(e) = save_to_json_file(&long_tree, "long_tree.json") {
                        eprintln!("Failed to save long_tree: {}", e);
                    }
                    if let Err(e) = save_to_json_file(&short_tree, "short_tree.json") {
                        eprintln!("Failed to save short_tree: {}", e);
                    }
                    if let Err(e) = save_to_json_file(&long_map.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "long_map.json") {
                        eprintln!("Failed to save long_map: {}", e);
                    }
                    if let Err(e) = save_to_json_file(&short_map.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "short_map.json") {
                        eprintln!("Failed to save short_map: {}", e);
                    }
                    println!("Position map and tree manually saved");
                }
                _ => eprintln!("Unexpected message in tx_position"),
            }
            full_interest(Utc::now().timestamp_micros(),&long_tree,&short_tree,&config_position,&txm2 );
        }
            
            if last_save_time_position.elapsed().as_secs() >= 800 {
                if let Err(e) = save_to_json_file(&long_tree, "long_tree.json") {
                    eprintln!("Failed to save long_tree: {}", e);
                }
                if let Err(e) = save_to_json_file(&short_tree, "short_tree.json") {
                    eprintln!("Failed to save short_tree: {}", e);
                }
                if let Err(e) = save_to_json_file(&long_map.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "long_map.json") {
                    eprintln!("Failed to save long_map: {}", e);
                }
                if let Err(e) = save_to_json_file(&short_map.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "short_map.json") {
                    eprintln!("Failed to save short_map: {}", e);
                }
                println!("Position map and tree periodically saved");
                last_save_time_position = Instant::now();
            }   
    }
});

thread::spawn({//Broker
    let db1_broker = Arc::clone(&db1_broker);
    let coll_config = Arc::clone(&coll_config);

    move || { 
    
    loop {
       
        if let Ok(msg_a) = rx_broker.recv() {
            match msg_a {
                Structs::LimitOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_lmtorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::MarketOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_mktorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::StopOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_sorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::StopLimitOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_slorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::ModifyOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_modforder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::DeleteOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_dltorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::TraderOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_p_order, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::TraderStopOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_p_sorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::TraderStopLimitOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_p_slorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::DeletedOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let order_id = order.order_identifier;
                            match insert_document_collection(&db1_broker.db, &coll_config.coll_h_dltd_order, &order).await {
                                Ok(_) => {},
                                Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                            };
                            match delete_document_by_orderid(&db1_broker.db, &coll_config.coll_p_order, order_id).await {
                                Ok(_) => println!("Successfully deleted LimitOrder"),
                                Err(e) => eprintln!("Failed to delete LimitOrder: {:?}", e),
                            };
                    });
                }
                Structs::DeletedStopOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let order_id = order.order_identifier;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_dltd_sorder, &order).await {
                            Ok(_) =>{},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        match delete_document_by_orderid(&db1_broker.db, &coll_config.coll_p_sorder, order_id).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to delete StopOrder: {:?}", e),
                        };
                    });
                }
                Structs::DeletedStopLimitOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let order_id = order.order_identifier;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_dltd_slorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        match delete_document_by_orderid(&db1_broker.db, &coll_config.coll_p_slorder, order_id).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to delete StopOrder: {:?}", e),
                        };
                    });
                }
                Structs::ModifiedOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let order_id = order.order_identifier;
                        let order_quant = order.new_order_quantity;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_modfd_order, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        match modify_document_by_orderid(&db1_broker.db, &coll_config.coll_p_order, order_id,order_quant).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                        };
                    });
                }
                Structs::ModifiedStopOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let order_id = order.order_identifier;
                        let order_quant = order.new_order_quantity;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_modfd_sorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        match modify_document_by_orderid(&db1_broker.db, &coll_config.coll_p_sorder, order_id,order_quant).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                        };
                    });
                }
                Structs::ModifiedStopLimitOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let order_id = order.order_identifier;
                        let order_quant = order.new_order_quantity;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_modfd_slorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        match modify_document_by_orderid(&db1_broker.db, &coll_config.coll_p_slorder, order_id,order_quant).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                        };
                    });
                }
                Structs::Messaging(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_t_message, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::ExecutedStop(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let order_id = order.order_identifier;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_exctd_sorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        match delete_document_by_orderid(&db1_broker.db, &coll_config.coll_p_sorder, order_id).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to delete StopOrder: {:?}", e),
                        };
                    });
                }
                Structs::ExecutedStopLimit(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let order_id = order.order_identifier;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_exctd_slorder, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        match delete_document_by_orderid(&db1_broker.db, &coll_config.coll_p_slorder, order_id).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to delete StopOrder: {:?}", e),
                        };
                    });
                }
                Structs::MatchStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let oid_maker = order.order_identifier_maker;
                            let quant = order.order_quantity;
                            match insert_document_collection(&db1_broker.db, &coll_config.coll_h_match, &order).await {
                                Ok(_) => {},
                                Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                            };
                           
                            let p_order: Result<TraderOrderStruct, String> = fetch_document_byorderid(&db1_broker.db, &coll_config.coll_p_order, oid_maker).await;
                            let p_order_s = match p_order {
                            Ok(user) => user, // Extract the user if the result is Ok
                            Err(err) => {
                                eprintln!("{}", err);
                                return;
                                }
                            };
                            let remaining_quantity = p_order_s.order_quantity - quant;
                           

                            if remaining_quantity == 0 {
                                match delete_document_by_orderid(&db1_broker.db, &coll_config.coll_p_order, oid_maker).await {
                                    Ok(_) => {},
                                    Err(e) => eprintln!("Failed to delete limit Order: {:?}", e),
                                };
                            } else if remaining_quantity > 0 {
                                match modify_document_by_orderid(&db1_broker.db, &coll_config.coll_p_order, oid_maker,remaining_quantity).await {
                                    Ok(_) => {},
                                    Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                                };
                            } else {
                                match delete_document_by_orderid(&db1_broker.db, &coll_config.coll_p_order, oid_maker).await {
                                    Ok(_) => {},
                                    Err(e) => eprintln!("Failed to delete limit Order: {:?}", e),
                                };
                            }
                    });
                }
                Structs::DeleteIcebergOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_dlticeberg, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert DeleteIcebergOrder: {:?}", e),
                        };
                    });
                }
                Structs::DeletedIcebergStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let id = order.iceberg_identifier;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_dltd_iceberg, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert DeletedIcebergStruct: {:?}", e),
                        };
                        match delete_iceberg_by_orderid(&db1_broker.db, &coll_config.coll_p_iceberg, id).await {
                            Ok(_) => println!("Successfully deleted LimitOrder"),
                            Err(e) => eprintln!("Failed to delete LimitOrder: {:?}", e),
                        };
                    });
                }
                Structs::ExecutedIceberg(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_exctd_iceberg, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert ExecutedIceberg: {:?}", e),
                        };
                        let p_iceberg: Result<IcebergOrderStruct, String> = fetch_iceberg(&db1_broker.db, &coll_config.coll_p_iceberg, order.iceberg_identifier).await;
                            let p_iceberg_s = match p_iceberg {
                            Ok(user) => user, // Extract the user if the result is Ok
                            Err(err) => {
                                eprintln!("{}", err);
                                return;
                                }
                            };
                            let remaining_quantity = p_iceberg_s.resting_quantity - order.executed_quantity;
                           

                            if remaining_quantity == 0 {
                                match delete_iceberg_by_orderid(&db1_broker.db, &coll_config.coll_p_iceberg, order.iceberg_identifier).await {
                                    Ok(_) => {},
                                    Err(e) => eprintln!("Failed to delete iceberg Order: {:?}", e),
                                };
                            } else if remaining_quantity > 0 {
                                match update_iceberg_resting_by_orderid(&db1_broker.db, &coll_config.coll_p_iceberg, order.iceberg_identifier,remaining_quantity).await {
                                    Ok(_) => {},
                                    Err(e) => eprintln!("Failed to update iceberg Order: {:?}", e),
                                };
                            } else {
                                match delete_iceberg_by_orderid(&db1_broker.db, &coll_config.coll_p_iceberg, order.iceberg_identifier).await {
                                    Ok(_) => {},
                                    Err(e) => eprintln!("Failed to delete iceberg Order: {:?}", e),
                                };
                            }
                    });
                }
                Structs::IcebergOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_iceberg, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert IcebergOrder: {:?}", e),
                        };
                    });
                }
                Structs::ModifiedIcebergStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let id = order.iceberg_identifier;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_modfd_iceberg, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert ModifiedIcebergStruct: {:?}", e),
                        };
                        match modify_iceberg_by_orderid(&db1_broker.db, &coll_config.coll_p_iceberg, id,order.new_quantity,order.new_visible_quantity).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                        };
                    });
                }
                Structs::ModifyIcebergOrder(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_modficeberg, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert ModifyIcebergOrder: {:?}", e),
                        };
                    });
                }
                Structs::IcebergOrderStruct(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_p_iceberg, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert IcebergOrderStruct: {:?}", e),
                        };
                    });
                }
                Structs::PositionStruct(position) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_a_position, &position).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        let paid_commission = PaidCommission {
                            unix_time: position.unix_time,
                            commission_amount: coll_config.commission*position.position_quantity, // Assuming the commission value is fetched from the .env file as shown earlier
                            c_type: "Opening".to_string(),
                        };
                    
                        // Insert the PaidCommission into the coll_h_cmmss_paid collection
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_cmmss_paid, &paid_commission).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert PaidCommission: {:?}", e),
                        };
                        let balance: Result<TraderBalance, String> = fetch_document_traderid(&db1_broker.db, &coll_config.coll_trdr_bal, position.trader_identifier).await;
                        let balance_s = match balance {
                        Ok(user) => user, // Extract the user if the result is Ok
                        Err(err) => {
                            eprintln!("{}", err);
                            return;
                            }
                        };
                        let a_balance = balance_s.balance - coll_config.commission*position.position_quantity;
    
                        match  update_balance_by_traderid(&db1_broker.db, &coll_config.coll_trdr_bal, position.trader_identifier,a_balance).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                        };
                    });
                }
                Structs::ClosePositionStruct(position) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        let pos_id = position.position_identifier;
                        let pos_quant = position.position_quantity;
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_clsdpos, &position).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                       
                        let a_position: Result<PositionStruct, String> = fetch_document_position(&db1_broker.db, &coll_config.coll_a_position, pos_id).await;
                        let a_position_s = match a_position {
                        Ok(user) => user, // Extract the user if the result is Ok
                        Err(err) => {
                            eprintln!("{}", err);
                            return;
                            }
                        };
                        let remaining_quantity =  a_position_s.position_quantity - pos_quant;
                       
    
                        if remaining_quantity == 0 {
                            match delete_document_by_posid(&db1_broker.db, &coll_config.coll_a_position, pos_id).await {
                                Ok(_) => {},
                                Err(e) => eprintln!("Failed to delete limit Order: {:?}", e),
                            };
                        } else if remaining_quantity > 0 {
                            match modify_document_by_posid(&db1_broker.db, &coll_config.coll_a_position, pos_id,remaining_quantity).await {
                                Ok(_) => {},
                                Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                            };
                        } else {
                            match delete_document_by_posid(&db1_broker.db, &coll_config.coll_a_position, pos_id).await {
                                Ok(_) => {},
                                Err(e) => eprintln!("Failed to delete limit Order: {:?}", e),
                            };
                        }
                        let paid_commission = PaidCommission {
                            unix_time: position.unix_time,
                            commission_amount: coll_config.commission*position.position_quantity, // Assuming the commission value is fetched from the .env file as shown earlier
                            c_type: "Closing".to_string(),
                        };
                    
                        // Insert the PaidCommission into the coll_h_cmmss_paid collection
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_cmmss_paid, &paid_commission).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert PaidCommission: {:?}", e),
                        };
                        let balance: Result<TraderBalance, String> = fetch_document_traderid(&db1_broker.db, &coll_config.coll_trdr_bal, position.trader_identifier).await;
                        let balance_s = match balance {
                        Ok(user) => user, // Extract the user if the result is Ok
                        Err(err) => {
                            eprintln!("{}", err);
                            return;
                            }
                        };
                        let a_balance = balance_s.balance - coll_config.commission*position.position_quantity;
    
                        match  update_balance_by_traderid(&db1_broker.db, &coll_config.coll_trdr_bal, position.trader_identifier,a_balance).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                        };
                    });
                }
                Structs::PostTraderInf(order) => {
                    let db1_broker = Arc::clone(&db1_broker);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_pnl_no_cmmss, &order).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert LimitOrder: {:?}", e),
                        };
                        let adjusted_balance = order.trader_calcbalance - coll_config.commission * 2 * order.position_quantity;
    
                        // Create a new PostTraderInf struct with the updated balance
                        let order_with_commission = PostTraderInf {
                            trader_calcbalance: adjusted_balance,
                            ..order // Keep the rest of the fields unchanged
                        };
    
                        // Insert into coll_h_pnl_w_cmmss
                        match insert_document_collection(&db1_broker.db, &coll_config.coll_h_pnl_w_cmmss, &order_with_commission).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to insert PostTraderInf with commission: {:?}", e),
                        };
                        let balance: Result<TraderBalance, String> = fetch_document_traderid(&db1_broker.db, &coll_config.coll_trdr_bal, order.trader_identifier).await;
                        let balance_s = match balance {
                        Ok(user) => user, // Extract the user if the result is Ok
                        Err(err) => {
                            eprintln!("{}", err);
                            return;
                            }
                        };
                        let a_balance = balance_s.balance + order.trader_calcbalance;
    
                        match  update_balance_by_traderid(&db1_broker.db, &coll_config.coll_trdr_bal, order.trader_identifier,a_balance).await {
                            Ok(_) => {},
                            Err(e) => eprintln!("Failed to modified limitOrder: {:?}", e),
                        };
                    });
                }

                _ => {} 
            }
           
            thread::sleep(Duration::from_nanos(10));
        } 
    }
}});
thread::spawn({//Market
    let db1_market = Arc::clone(&db1_market);
    let connection_type_map = Arc::clone(&connection_type_map);
    let ws_connections = Arc::clone(&ws_connections);
    let coll_config = Arc::clone(&coll_config);
    move || {  

    loop {
     
        if let Ok(msg_a) = rx_market.recv() {
            match msg_a {
                Structs::Last(msg) => {
                    let db1_market = Arc::clone(&db1_market);
                    let connection_type_map = Arc::clone(&connection_type_map);
                    let ws_connections = Arc::clone(&ws_connections);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{ //Last
                        
                        
                            if let Err(err) = insert_document_collection(&db1_market.db, &coll_config.coll_h_last, &msg).await {
                                eprintln!("DATABASE_INSERTION_FAILURE: {}", err);
                                return;  // Continue to the next iteration even if insertion fails
                            }
                    
                            // Serialize the message to JSON
                            let json_data = match serde_json::to_string(&msg) {
                                Ok(data) => data,
                                Err(err) => {
                                    eprintln!("JSON_SERIALIZATION_FAILURE: {}", err);
                                    return;  // Continue to the next iteration if serialization fails
                                }
                            };
                    
                            if let Some(session_ids) = connection_type_map.get(&ConnectionType::Last) {
                                for session_id in session_ids.value().iter() {
                                    let session_id = session_id.clone();
                                    // Retrieve the session from ws_connections using the session_id
                                    if let Some(mut session) = ws_connections.get_mut(&session_id) {
                                        let session = session.value_mut();
                                        
                                        // Send the message to the session
                                        if let Err(e) = session.text(json_data.clone()).await {
                                            eprintln!("Failed to send message to session {}: {:?}", session_id, e);
                                        }
                                    } else {
                                        eprintln!("Session not found for ID: {}", session_id);
                                    }
                                }
                            }    
                           
                        
                    });

                }
                Structs::MBPEvents(msg) => {
                    let db1_market = Arc::clone(&db1_market);
                    let connection_type_map = Arc::clone(&connection_type_map);
                    let ws_connections = Arc::clone(&ws_connections);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{ // Mbpevent
                                               
                            if let Err(err) = insert_document_collection(&db1_market.db, &coll_config.coll_h_mbpevent, &msg).await {
                                eprintln!("DATABASE_INSERTION_FAILURE: {}", err);
                                return;  // Continue to the next iteration even if insertion fails
                            }
                    
                            // Serialize the message to JSON
                            let json_data = match serde_json::to_string(&msg) {
                                Ok(data) => data,
                                Err(err) => {
                                    eprintln!("JSON_SERIALIZATION_FAILURE: {}", err);
                                    return;  // Continue to the next iteration if serialization fails
                                }
                            };
                    
                            if let Some(session_ids) = connection_type_map.get(&ConnectionType::MbpEvent) {
                                for session_id in session_ids.value().iter() {
                                    let session_id = session_id.clone();
                                    // Retrieve the session from ws_connections using the session_id
                                    if let Some(mut session) = ws_connections.get_mut(&session_id) {
                                        let session = session.value_mut();
                                        
                                        // Send the message to the session
                                        if let Err(e) = session.text(json_data.clone()).await {
                                            eprintln!("Failed to send message to session {}: {:?}", session_id, e);
                                        }
                                    } else {
                                        eprintln!("Session not found for ID: {}", session_id);
                                    }
                                }
                            }  
                        
                    });
                }
                Structs::InterestEvents(msg) => {
                    let db1_market = Arc::clone(&db1_market);
                    let connection_type_map = Arc::clone(&connection_type_map);
                    let ws_connections = Arc::clone(&ws_connections);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{ // Interestevent
                       
                            if let Err(err) = insert_document_collection(&db1_market.db, &coll_config.coll_h_interestevent, &msg).await {
                                eprintln!("DATABASE_INSERTION_FAILURE: {}", err);
                                return;  // Continue to the next iteration even if insertion fails
                            }
                    
                            // Serialize the message to JSON
                            let json_data = match serde_json::to_string(&msg) {
                                Ok(data) => data,
                                Err(err) => {
                                    eprintln!("JSON_SERIALIZATION_FAILURE: {}", err);
                                    return;  // Continue to the next iteration if serialization fails
                                }
                            };
                    
                            if let Some(session_ids) = connection_type_map.get(&ConnectionType::InterestEvent) {
                                for session_id in session_ids.value().iter() {
                                    let session_id = session_id.clone();
                                    // Retrieve the session from ws_connections using the session_id
                                    if let Some(mut session) = ws_connections.get_mut(&session_id) {
                                        let session = session.value_mut();
                                        
                                        // Send the message to the session
                                        if let Err(e) = session.text(json_data.clone()).await {
                                            eprintln!("Failed to send message to session {}: {:?}", session_id, e);
                                        }
                                    } else {
                                        eprintln!("Session not found for ID: {}", session_id);
                                    }
                                }
                            }  
                        
                    });
                }
                Structs::BBO(msg) => {
                    let db1_market = Arc::clone(&db1_market);
                    let connection_type_map = Arc::clone(&connection_type_map);
                    let ws_connections = Arc::clone(&ws_connections);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{ //bbo
                       
                            if let Err(err) = insert_document_collection(&db1_market.db, &coll_config.coll_h_bbo, &msg).await {
                                eprintln!("DATABASE_INSERTION_FAILURE: {}", err);
                                return;  // Continue to the next iteration even if insertion fails
                            }
                    
                            // Serialize the message to JSON
                            let json_data = match serde_json::to_string(&msg) {
                                Ok(data) => data,
                                Err(err) => {
                                    eprintln!("JSON_SERIALIZATION_FAILURE: {}", err);
                                    return;  // Continue to the next iteration if serialization fails
                                }
                            };
                    
                            if let Some(session_ids) = connection_type_map.get(&ConnectionType::Nbbo) {
                                for session_id in session_ids.value().iter() {
                                    let session_id = session_id.clone();
                                    // Retrieve the session from ws_connections using the session_id
                                    if let Some(mut session) = ws_connections.get_mut(&session_id) {
                                        let session = session.value_mut();
                                        
                                        // Send the message to the session
                                        if let Err(e) = session.text(json_data.clone()).await {
                                            eprintln!("Failed to send message to session {}: {:?}", session_id, e);
                                        }
                                    } else {
                                        eprintln!("Session not found for ID: {}", session_id);
                                    }
                                }
                            }  
                        
                    });
                }
                Structs::TimeSale(msg) => {
                    let db1_market = Arc::clone(&db1_market);
                    let connection_type_map = Arc::clone(&connection_type_map);
                    let ws_connections = Arc::clone(&ws_connections);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{ //TNS
                       
                            if let Err(err) = insert_document_collection(&db1_market.db, &coll_config.coll_h_tns, &msg).await {
                                eprintln!("DATABASE_INSERTION_FAILURE: {}", err);
                                return;  // Continue to the next iteration even if insertion fails
                            }
                    
                            // Serialize the message to JSON
                            let json_data = match serde_json::to_string(&msg) {
                                Ok(data) => data,
                                Err(err) => {
                                    eprintln!("JSON_SERIALIZATION_FAILURE: {}", err);
                                    return;  // Continue to the next iteration if serialization fails
                                }
                            };
                    
                            if let Some(session_ids) = connection_type_map.get(&ConnectionType::Tns) {
                                for session_id in session_ids.value().iter() {
                                    let session_id = session_id.clone();
                                    // Retrieve the session from ws_connections using the session_id
                                    if let Some(mut session) = ws_connections.get_mut(&session_id) {
                                        let session = session.value_mut();
                                        
                                        // Send the message to the session
                                        if let Err(e) = session.text(json_data.clone()).await {
                                            eprintln!("Failed to send message to session {}: {:?}", session_id, e);
                                        }
                                    } else {
                                        eprintln!("Session not found for ID: {}", session_id);
                                    }
                                }
                            }  
                        
                    });
                }
                Structs::Volume(msg) => {
                    let db1_market = Arc::clone(&db1_market);
                    let connection_type_map = Arc::clone(&connection_type_map);
                    let ws_connections = Arc::clone(&ws_connections);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{ //volume
                       
                            if let Err(err) = insert_document_collection(&db1_market.db, &coll_config.coll_h_volume, &msg).await {
                                eprintln!("DATABASE_INSERTION_FAILURE: {}", err);
                                return;  // Continue to the next iteration even if insertion fails
                            }
                    
                            // Serialize the message to JSON
                            let json_data = match serde_json::to_string(&msg) {
                                Ok(data) => data,
                                Err(err) => {
                                    eprintln!("JSON_SERIALIZATION_FAILURE: {}", err);
                                    return;  // Continue to the next iteration if serialization fails
                                }
                            };
                    
                            if let Some(session_ids) = connection_type_map.get(&ConnectionType::Volume) {
                                for session_id in session_ids.value().iter() {
                                    let session_id = session_id.clone();
                                    // Retrieve the session from ws_connections using the session_id
                                    if let Some(mut session) = ws_connections.get_mut(&session_id) {
                                        let session = session.value_mut();
                                        
                                        // Send the message to the session
                                        if let Err(e) = session.text(json_data.clone()).await {
                                            eprintln!("Failed to send message to session {}: {:?}", session_id, e);
                                        }
                                    } else {
                                        eprintln!("Session not found for ID: {}", session_id);
                                    }
                                }
                            }  

                    });
                }
                Structs::FullOB(msg) => {
                    let db1_market = Arc::clone(&db1_market);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{ //fullob
                        
                            if let Err(err) = overwrite_document(&db1_market.db, &coll_config.coll_fullob, &msg).await {
                                eprintln!("DATABASE_INSERTION_FAILURE: {}", err);
                                return;  // Continue to the next iteration even if insertion fails
                            }    
                        
                    });
                   
                }
                Structs::FullInterest(msg) => {
                    let db1_market = Arc::clone(&db1_market);
                    let coll_config = Arc::clone(&coll_config);
                    tokio::spawn(async move{ //fullinterest
                        
                            if let Err(err) = overwrite_document(&db1_market.db, &coll_config.coll_fullinterest, &msg).await {
                                eprintln!("DATABASE_INSERTION_FAILURE: {}", err);
                                return;  // Continue to the next iteration even if insertion fails
                            }    
                        
                    });
                   
                }

                _ => {} 
            }
            thread::sleep(Duration::from_nanos(10));
        }
    }
}});

    let mut last_save_time = Instant::now();
     thread::spawn( move || { //order
        loop {
            //let message_arrive:Structs;
          
            match rx_order.try_recv() {
                Ok(msg) => {
                    // Since the message is already deserialized, directly assign it
                   let message_arrive = msg.clone();
                   let message_arrive_save = msg.clone();
                    if let Err(e) = tx_broker.send(msg) {
                        eprintln!("Error sending message through txb2: {:?}", e);
                    }
                    match message_arrive {
                        Structs::LimitOrder(arrival) => {
                            match arrival.order_side {
                                OrderSide::Long => {
                                    let order_idb = arrival.order_identifier.unwrap_or_else(id_i64);
                                    let unixtimeb = Utc::now().timestamp_micros(); 
                                    let lowest_ask_price = lowest_ask(&ask_mbp);
                                        match lowest_ask_price {
                                            None => {
                                                if arrival.expiration != OrderExpiration::FOK && arrival.expiration != OrderExpiration::IOC{
                                                    
                                                let bid_structi = TraderOrderStruct {
                                                    market: arrival.market.clone(),
                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                    unix_time: unixtimeb,
                                                    trader_identifier: arrival.trader_identifier,
                                                    order_identifier: order_idb,
                                                    order_quantity:arrival.order_quantity,
                                                    order_side: arrival.order_side.clone(),
                                                    expiration:arrival.expiration.clone(),
                                                    price: arrival.price,
                                                    pointing_at:arrival.pointing_at
                                                };
                                                bid_struct.insert(order_idb, bid_structi);//insert in last traded price
                    
                                                // Inserting into bid_map
                                                bid_map.map.entry(arrival.price)
                                                .and_modify(|vec| vec.push(order_idb))
                                                .or_insert_with(|| vec![order_idb]);
                    
                                                // Inserting into bid_mbo
                                                match bid_mbo.mbo.entry(arrival.price) {
                                                Entry::Occupied(mut entry) => {
                                                    entry.get_mut().push(arrival.order_quantity);
                                                }
                                                Entry::Vacant(entry) => {
                                                    entry.insert(vec![arrival.order_quantity]);
                                                }
                                                }
                                                
                                                // Inserting into bid_mbp
                                                *bid_mbp.mbp.entry(arrival.price).or_insert(0) += arrival.order_quantity;
                    
                                                let bid_structii = TraderOrderStruct {
                                                   
                                                    market: arrival.market.clone(),
                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                    unix_time: unixtimeb,
                                                    trader_identifier: arrival.trader_identifier,
                                                    order_identifier: order_idb,
                                                    order_quantity:arrival.order_quantity,
                                                    order_side: arrival.order_side.clone(),
                                                    expiration:arrival.expiration.clone(),
                                                    price: arrival.price,
                                                    pointing_at:arrival.pointing_at,
                                                };
                                                let order_message = Structs::TraderOrderStruct(bid_structii);
                                                if let Err(e) = tx_broker.send(order_message) {
                                                    eprintln!("Failed to send message: {:?}", e);
                                                }
                    
                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),arrival.price,order_idb );
                                               
                                                message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), arrival.order_quantity, arrival.price, 1, &tx_market,&config);
                                                
                                            } else {
                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                            }
                    
                                            },
                                            Some(pricea) => {
                                                if arrival.price < pricea {//lowest_ask_price{
                    
                                                    if arrival.expiration != OrderExpiration::FOK && arrival.expiration != OrderExpiration::IOC{
                                                        let bid_structi = TraderOrderStruct {
                                                           
                                                            market: arrival.market.clone(),
                                                            broker_identifier: arrival.broker_identifier.clone(),
                                                            unix_time: unixtimeb,
                                                            trader_identifier: arrival.trader_identifier,
                                                            order_identifier: order_idb,
                                                            order_quantity:arrival.order_quantity,
                                                            order_side: arrival.order_side.clone(),
                                                            expiration:arrival.expiration.clone(),
                                                            price: arrival.price,
                                                            pointing_at:arrival.pointing_at
                                                        };
                                                        bid_struct.insert(order_idb, bid_structi);//insert in last traded price
                    
                                                        // Inserting into bid_map
                                                        bid_map.map.entry(arrival.price)
                                                        .and_modify(|vec| vec.push(order_idb))
                                                        .or_insert_with(|| vec![order_idb]);
                    
                                                        // Inserting into bid_mbo
                                                        match bid_mbo.mbo.entry(arrival.price) {
                                                        Entry::Occupied(mut entry) => {
                                                            entry.get_mut().push(arrival.order_quantity);
                                                        }
                                                        Entry::Vacant(entry) => {
                                                            entry.insert(vec![arrival.order_quantity]);
                                                        }
                                                        }
                                                        
                                                        // Inserting into bid_mbp
                                                        *bid_mbp.mbp.entry(arrival.price).or_insert(0) += arrival.order_quantity;
                                
                                                        let bid_structii = TraderOrderStruct {
                                                           
                                                            market: arrival.market.clone(),
                                                            broker_identifier: arrival.broker_identifier.clone(),
                                                            unix_time: unixtimeb,
                                                            trader_identifier: arrival.trader_identifier,
                                                            order_identifier: order_idb,
                                                            order_quantity:arrival.order_quantity,
                                                            order_side: arrival.order_side.clone(),
                                                            expiration:arrival.expiration.clone(),
                                                            price: arrival.price,
                                                            pointing_at:arrival.pointing_at
                                                        };
                                                        let order_message = Structs::TraderOrderStruct(bid_structii);
                                                        if let Err(e) = tx_broker.send(order_message) {
                                                            eprintln!("Failed to send message: {:?}", e);
                                                        }
                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market.clone(),arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),arrival.price,order_idb );
                                                        message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                        mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), arrival.order_quantity, arrival.price, 1, &tx_market,&config);
                                                        
                                                    } else {
                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                        message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                        
                                                    }
                    
                                                }else if arrival.price >= pricea{ //eto ny tena operation
                    
                                                    match arrival.expiration {
                                                        OrderExpiration::FOK => {
                                                            // Handle FOK expiration
                                                            //Market Long FOK
                                                            if let Some(lowest_ask_price) = lowest_ask_price {
                                                                let mut total_quantity = 0;
                                                                let mut fill_ask_price = lowest_ask_price;
                                                               
                                                                for (&price, &quantity) in ask_mbp.mbp.range(lowest_ask_price..=arrival.price) {
                                                                    total_quantity += quantity;
                                                                    if total_quantity >= arrival.order_quantity {
                                                                        // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                        fill_ask_price = price;
                                                                        break; // No need to continue iterating once the condition is met
                                                                    }
                                                                }
                                                            
                                                                // Check if total quantity is superior to arrival.order_quantity
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */  if total_quantity >= arrival.order_quantity {
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                                    // Perform your operation here
                                                                    let mut remaining_quantity = arrival.order_quantity;
                                                                    let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                        .range(lowest_ask_price..=fill_ask_price)
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                           
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                           let volume_message = Structs::Volume(volume_struct);
                                                                                           if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }                                                                       let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                    
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                            
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                    ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                               
                                                                                               
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                               let tns_message = Structs::TimeSale(tns);
                                                                                               if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                let tplast_message = Structs::Last(tplast);
                                                                                                if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }                                                                               
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                                
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                               
                                                                                                remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                                                            
                                                            
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                           
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                           
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                           let tplast_message = Structs::Last(tplast);
                                                                                           if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                    
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                            
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                                quants_empty = true;
                                                                            }
                                                            
                                                                        }
                                                                        if quants_empty {
                                                                            ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                ask_mbp.mbp.remove(&price);
                                                                                ask_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    
                                                                    
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */   } else {
                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                                message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                                   
                                                                   
                                                                }
                                                            } 
                                                        }
                                                        OrderExpiration::IOC => {
                                                            // Handle IOC expiration
                                                            if let Some(lowest_ask_price) = lowest_ask_price {
                                                                let mut total_quantity = 0;
                                                                let mut fill_ask_price = lowest_ask_price;
                                                                let mut highest_disponible_price = lowest_ask_price;
                                                                let mut matchable_quant :i32 = 0;
                                                               
                                                                for (&price, &quantity) in ask_mbp.mbp.range(lowest_ask_price..=arrival.price) {
                                                                    total_quantity += quantity;
                                                                    if total_quantity >= arrival.order_quantity {
                                                                        // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                        fill_ask_price = price;
                                                                        break; // No need to continue iterating once the condition is met
                                                                    }else if total_quantity < arrival.order_quantity {
                                                                        // Update highest disponible price
                                                                        highest_disponible_price = price;
                                                                        matchable_quant = total_quantity;
                                                                    }
                                                                }
                                                    
                                                                // Check if total quantity is superior to arrival.order_quantity
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    if total_quantity >= arrival.order_quantity {
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                                    // Perform your operation here
                                                                    let mut remaining_quantity = arrival.order_quantity;
                                                                    let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                        .range(lowest_ask_price..=fill_ask_price)
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                       
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                             let tplast_message = Structs::Last(tplast);
                                                                                             if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price.clone(),&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                  
                                                                                    ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                               
                                                                                              
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                                 let tns_message = Structs::TimeSale(tns);
                                                                                                 if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                let tplast_message = Structs::Last(tplast);
                                                                                                if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                                
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                               let volume_message = Structs::Volume(volume_struct);
                                                                                               if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                               ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                               ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                               mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                                
                                                                                            remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                            
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                                quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                ask_mbp.mbp.remove(&price);
                                                                                ask_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    
                                                                   
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    } else if total_quantity < arrival.order_quantity{
                                                                    // Do something with highest_disponible_price and matchable quantity, avela ny reste
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched:{} matched, {} cancelled.", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price,matchable_quant,arrival.order_quantity - matchable_quant );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                    
                                                                    let mut remaining_quantity = matchable_quant;
                                                                    let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                        .range(lowest_ask_price..=highest_disponible_price)
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                      
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                           
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                   
                                                                                    ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                                
                                                                                              
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                               let tns_message = Structs::TimeSale(tns);
                                                                                               if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                let tplast_message = Structs::Last(tplast);
                                                                                                if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                                
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                               
                                                                                            remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                            
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                                quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                ask_mbp.mbp.remove(&price);
                                                                                ask_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            // Handle all other cases
                                                            if let Some(lowest_ask_price) = lowest_ask_price {
                                                                let mut total_quantity = 0;
                                                                let mut fill_ask_price = lowest_ask_price;
                                                                let mut highest_disponible_price = lowest_ask_price;
                                                                let mut matchable_quant :i32 = 0;
                                                                let mut limitable_quant :i32 =0;
                                                               
                                                                for (&price, &quantity) in ask_mbp.mbp.range(lowest_ask_price..=arrival.price) {
                                                                    total_quantity += quantity;
                                                                    if total_quantity >= arrival.order_quantity {
                                                                        // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                        fill_ask_price = price;
                                                                        break; // No need to continue iterating once the condition is met
                                                                    }else if total_quantity < arrival.order_quantity {
                                                                        // Update highest disponible price
                                                                        highest_disponible_price = price;
                                                                        matchable_quant = total_quantity;
                                                                        limitable_quant = arrival.order_quantity - total_quantity;
                    
                                                                    }
                                                                }
                                                    
                                                                // Check if total quantity is superior to arrival.order_quantity
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    if total_quantity >= arrival.order_quantity {
                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                                        message_limit_taker(&arrival,&tx_broker,string_m.clone());
                    
                                                                    // Perform your operation here
                                                                    let mut remaining_quantity = arrival.order_quantity;
                                                                    let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                        .range(lowest_ask_price..=fill_ask_price)
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                       
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                   
                                                                                    ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                                
                                                                                               
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                                let tns_message = Structs::TimeSale(tns);
                                                                                                if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                               let tplast_message = Structs::Last(tplast);
                                                                                               if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                                
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                            remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                           
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                             let tplast_message = Structs::Last(tplast);
                                                                                             if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                             let volume_message = Structs::Volume(volume_struct);
                                                                                             if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                             ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                             ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                             mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                                quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                ask_mbp.mbp.remove(&price);
                                                                                ask_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    } else if total_quantity < arrival.order_quantity{
                                                                    // mivadika buy limit ny unfilled order,Do something with highest_disponible_price,matchable quant and limitable quant
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched:{} matched, {} entered to book.", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price,matchable_quant,arrival.order_quantity - matchable_quant );
                                                                     message_limit_taker(&arrival,&tx_broker,string_m.clone());
                    
                                                                    let mut remaining_quantity = matchable_quant;
                                                                    let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                        .range(lowest_ask_price..=highest_disponible_price)
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                       
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                   
                                                                                    ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                               ;
                                                                                              
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                                let tns_message = Structs::TimeSale(tns);
                                                                                                if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                let tplast_message = Structs::Last(tplast);
                                                                                                if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                                
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                        
                                                                                            remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_idb,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                                if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                }
                                                                                
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                                quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                ask_mbp.mbp.remove(&price);
                                                                                ask_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    ///////////////////////////////////////////////////////////////////////////////////////////
                                                                    let bid_structi = TraderOrderStruct {
                                                                        
                                                                        market: arrival.market.clone(),
                                                                        broker_identifier: arrival.broker_identifier.clone(),
                                                                        unix_time: Utc::now().timestamp_micros(),
                                                                        trader_identifier: arrival.trader_identifier,
                                                                        order_identifier: order_idb,
                                                                        order_quantity:limitable_quant,
                                                                        order_side: arrival.order_side.clone(),
                                                                        expiration:arrival.expiration.clone(),
                                                                        price: arrival.price,
                                                                        pointing_at:arrival.pointing_at,
                                                                    };
                                                                    bid_struct.insert(order_idb, bid_structi);//insert in highest disponible price
                        
                                                                    // Inserting into bid_map
                                                                    bid_map.map.entry(arrival.price)
                                                                    .and_modify(|vec| vec.push(order_idb))
                                                                    .or_insert_with(|| vec![order_idb]);
                        
                                                                    // Inserting into bid_mbo
                                                                    match bid_mbo.mbo.entry(arrival.price) {
                                                                    Entry::Occupied(mut entry) => {
                                                                        entry.get_mut().push(limitable_quant);
                                                                    }
                                                                    Entry::Vacant(entry) => {
                                                                        entry.insert(vec![limitable_quant]);
                                                                    }
                                                                    }
                        
                                                                    // Inserting into bid_mbp
                                                                    *bid_mbp.mbp.entry(arrival.price).or_insert(0) += arrival.order_quantity-matchable_quant;
                                            
                                                                    let bid_structii = TraderOrderStruct {
                                                                       
                                                                        market: arrival.market.clone(),
                                                                        broker_identifier: arrival.broker_identifier.clone(),
                                                                        unix_time: Utc::now().timestamp_micros(),
                                                                        trader_identifier: arrival.trader_identifier,
                                                                        order_identifier: order_idb,
                                                                        order_quantity:limitable_quant,
                                                                        order_side: arrival.order_side.clone(),
                                                                        expiration:arrival.expiration.clone(),
                                                                        price: arrival.price,
                                                                        pointing_at:arrival.pointing_at,
                                                                    };
                                                                    let order_message = Structs::TraderOrderStruct(bid_structii);
                                                                    if let Err(e) = tx_broker.send(order_message) {
                                                                        eprintln!("Failed to send message: {:?}", e);
                                                                    }
                    
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market,limitable_quant,arrival.order_side,arrival.expiration,arrival.price,order_idb );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                                    mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), limitable_quant, arrival.price, 1, &tx_market,&config);
                                                                   //////////////////////////////////////
                                                                   
                                                                  
                                                                }
                                                            }
                                                        }
                                                    }
                    
                                                }
                                                
                                            },
                                            
                                    }
                                   
                                }
                                OrderSide::Short => {
                                    let order_ids = arrival.order_identifier.unwrap_or_else(id_i64);
                                    let unixtimes = Utc::now().timestamp_micros();
                                    let highest_bid_price = highest_bid(&bid_mbp);
                                        match highest_bid_price {
                                            None => {
                                                if arrival.expiration != OrderExpiration::FOK && arrival.expiration != OrderExpiration::IOC{
                                                    let ask_structi = TraderOrderStruct {
                                                        
                                                        market: arrival.market.clone(),
                                                        broker_identifier: arrival.broker_identifier.clone(),
                                                        unix_time: unixtimes,
                                                        trader_identifier: arrival.trader_identifier,
                                                        order_identifier: order_ids,
                                                        order_quantity:arrival.order_quantity,
                                                        order_side: arrival.order_side.clone(),
                                                        expiration:arrival.expiration.clone(),
                                                        price: arrival.price,
                                                        pointing_at:arrival.pointing_at,
                                                    };
                                                    ask_struct.insert(order_ids, ask_structi); //insert in last traded price
                    
                                                    // Inserting into bid_map
                                                    ask_map.map.entry(arrival.price)
                                                    .and_modify(|vec| vec.push(order_ids))
                                                    .or_insert_with(|| vec![order_ids]);
                    
                                                    // Inserting into bid_mbo
                                                    match ask_mbo.mbo.entry(arrival.price) {
                                                    Entry::Occupied(mut entry) => {
                                                        entry.get_mut().push(arrival.order_quantity);
                                                    }
                                                    Entry::Vacant(entry) => {
                                                        entry.insert(vec![arrival.order_quantity]);
                                                    }
                                                    }
                    
                                                    // Inserting into bid_mbp
                                                    *ask_mbp.mbp.entry(arrival.price).or_insert(0) += arrival.order_quantity;
                    
                                                    let ask_structii = TraderOrderStruct {
                                                       
                                                        market: arrival.market.clone(),
                                                        broker_identifier: arrival.broker_identifier.clone(),
                                                        unix_time: unixtimes,
                                                        trader_identifier: arrival.trader_identifier,
                                                        order_identifier: order_ids,
                                                        order_quantity:arrival.order_quantity,
                                                        order_side: arrival.order_side.clone(),
                                                        expiration:arrival.expiration.clone(),
                                                        price: arrival.price,
                                                        pointing_at:arrival.pointing_at,
                                                    };
                    
                                                    let order_message = Structs::TraderOrderStruct(ask_structii);
                                                    if let Err(e) = tx_broker.send(order_message) {
                                                        eprintln!("Failed to send message: {:?}", e);
                                                    }
                                                    
                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market.clone(),arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),arrival.price,order_ids );
                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                    
                                                        mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), arrival.order_quantity, arrival.price, 1, &tx_market,&config);
                    
                                                
                                                }  else
                                                {  
                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),arrival.price );
                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                }  
                                            },
                                            Some(pricea) => {
                                                if arrival.price > pricea { //highest_bid_price.unwrap(){
                    
                                                    if arrival.expiration != OrderExpiration::FOK && arrival.expiration != OrderExpiration::IOC{
                                                        let ask_structi = TraderOrderStruct {
                                                          
                                                            market: arrival.market.clone(),
                                                            broker_identifier: arrival.broker_identifier.clone(),
                                                            unix_time: unixtimes,
                                                            trader_identifier: arrival.trader_identifier,
                                                            order_identifier: order_ids,
                                                            order_quantity:arrival.order_quantity,
                                                            order_side: arrival.order_side.clone(),
                                                            expiration:arrival.expiration.clone(),
                                                            price: arrival.price,
                                                            pointing_at:arrival.pointing_at,
                                                        };
                                                        ask_struct.insert(order_ids, ask_structi); //insert in last traded price
                    
                                                        // Inserting into bid_map
                                                        ask_map.map.entry(arrival.price)
                                                        .and_modify(|vec| vec.push(order_ids))
                                                        .or_insert_with(|| vec![order_ids]);
                    
                                                        // Inserting into bid_mbo
                                                        match ask_mbo.mbo.entry(arrival.price) {
                                                        Entry::Occupied(mut entry) => {
                                                            entry.get_mut().push(arrival.order_quantity);
                                                        }
                                                        Entry::Vacant(entry) => {
                                                            entry.insert(vec![arrival.order_quantity]);
                                                        }
                                                        }
                    
                                                        // Inserting into bid_mbp
                                                        *ask_mbp.mbp.entry(arrival.price).or_insert(0) += arrival.order_quantity;
                    
                                                        let ask_structii = TraderOrderStruct {
                                                         
                                                            market: arrival.market.clone(),
                                                            broker_identifier: arrival.broker_identifier.clone(),
                                                            unix_time: unixtimes,
                                                            trader_identifier: arrival.trader_identifier,
                                                            order_identifier: order_ids,
                                                            order_quantity:arrival.order_quantity,
                                                            order_side: arrival.order_side.clone(),
                                                            expiration:arrival.expiration.clone(),
                                                            price: arrival.price,
                                                            pointing_at:arrival.pointing_at,
                                                        };
                    
                                                        let order_message = Structs::TraderOrderStruct(ask_structii);
                                                        if let Err(e) = tx_broker.send(order_message) {
                                                            eprintln!("Failed to send message: {:?}", e);
                                                        }
                                                    
                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market.clone(),arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),arrival.price,order_ids );
                                                        message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                        mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), arrival.order_quantity, arrival.price, 1, &tx_market,&config);
                                                    
                                                    } else {
                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),arrival.price );
                                                        message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                    }  
                    
                                                }else if arrival.price <= pricea { //eto ny tena operation
                    
                                                    match arrival.expiration {
                                                        OrderExpiration::FOK => {
                                                            // Handle FOK expiration
                                                            if let Some(highest_bid_price) = highest_bid_price {
                                                                let mut total_quantity = 0;
                                                                let mut fill_bid_price = highest_bid_price;
                                                                
                                                                for (&price, &quantity) in bid_mbp.mbp.range(arrival.price..=highest_bid_price).rev() {
                                                                    total_quantity += quantity;
                                                                    if total_quantity >= arrival.order_quantity {
                                                                        // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                        fill_bid_price = price;
                                                                        break; // No need to continue iterating once the condition is met
                                                                    }
                                                                }
                                                    
                                                                // Check if total quantity is superior to arrival.order_quantity
                                                                if total_quantity >= arrival.order_quantity {
                                                                    // Perform your operation here
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                    
                                                                    let mut remaining_quantity = arrival.order_quantity;
                                                                    let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                        .range(fill_bid_price..=highest_bid_price)
                                                                        .rev()
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                      
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                         
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                           let volume_message = Structs::Volume(volume_struct);
                                                                                           if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                           ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                   
                                                                                    ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                               
                                                                                               
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                                let tns_message = Structs::TimeSale(tns);
                                                                                                if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                 let tplast_message = Structs::Last(tplast);
                                                                                                 if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                                
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                                
                                                                                                remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                                quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                bid_mbp.mbp.remove(&price);
                                                                                bid_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    
                                                                } else {
                                                                    // Do nothing
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                                }
                                                            }  
                                                        }
                                                        OrderExpiration::IOC => {
                                                            // Handle IOC expiration
                                                            if let Some(highest_bid_price) = highest_bid_price {
                                                                let mut total_quantity = 0;
                                                                let mut fill_bid_price = highest_bid_price;
                                                                let mut lowest_disponible_price = highest_bid_price;
                                                                let mut matchable_quant :i32 = 0;
                                                                
                                                                for (&price, &quantity) in bid_mbp.mbp.range(arrival.price..=highest_bid_price).rev() {
                                                                    total_quantity += quantity;
                                                                    if total_quantity >= arrival.order_quantity {
                                                                        // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                        fill_bid_price = price;
                                                                        break; // No need to continue iterating once the condition is met
                                                                    }else if total_quantity < arrival.order_quantity {
                                                                        // Update highest disponible price
                                                                        lowest_disponible_price = price;
                                                                        matchable_quant = total_quantity;
                                                                    }
                                                                }
                                                    
                                                                // Check if total quantity is superior to arrival.order_quantity
                                                                if total_quantity >= arrival.order_quantity {
                                                                    // Perform your operation here
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());     
                                                                    let mut remaining_quantity = arrival.order_quantity;
                                                                    let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                        .range(fill_bid_price..=highest_bid_price)
                                                                        .rev()
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                       
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                          
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                           
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                             let tns_message = Structs::TimeSale(tns);
                                                                                             if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                   
                                                                                    ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                                
                                                                                              
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                                let tns_message = Structs::TimeSale(tns);
                                                                                                if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                let tplast_message = Structs::Last(tplast);
                                                                                                if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                                
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                             
                                                                                            remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                           
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                            
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side,price,&config);
                                                                                             let volume_message = Structs::Volume(volume_struct);
                                                                                             if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                             ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                             ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                             mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                                quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                bid_mbp.mbp.remove(&price);
                                                                                bid_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    
                                                                } else if total_quantity < arrival.order_quantity {
                                                                    // avela any ny unfilled order
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched:{} matched, {} cancelled.", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price,matchable_quant,arrival.order_quantity - matchable_quant );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());     
                                                                    let mut remaining_quantity = matchable_quant;
                                                                    let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                        .range(lowest_disponible_price..=highest_bid_price)
                                                                        .rev()
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                      
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                            
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                             let tns_message = Structs::TimeSale(tns);
                                                                                             if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                        
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side,price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                   
                                                                                    ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                                
                                                                                               
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                                let tns_message = Structs::TimeSale(tns);
                                                                                                if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                let tplast_message = Structs::Last(tplast);
                                                                                                if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                               
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                     
                                                                                                remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side,price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                                quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                bid_mbp.mbp.remove(&price);
                                                                                bid_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    
                                                                }
                                                            }
                                                        }
                                                        _ => {
                                                            // Handle all other cases
                                                            if let Some(highest_bid_price) = highest_bid_price {
                                                                let mut total_quantity = 0;
                                                                let mut fill_bid_price = highest_bid_price;
                                                                let mut lowest_disponible_price = highest_bid_price;
                                                                let mut matchable_quant :i32 = 0;
                                                                let mut limitable_quant :i32 =0;
                                                                
                                                                for (&price, &quantity) in bid_mbp.mbp.range(arrival.price..=highest_bid_price).rev() {
                                                                    total_quantity += quantity;
                                                                    if total_quantity >= arrival.order_quantity {
                                                                        // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                        fill_bid_price = price;
                                                                        break; // No need to continue iterating once the condition is met
                                                                    }else if total_quantity < arrival.order_quantity {
                                                                        // Update highest disponible price
                                                                        lowest_disponible_price = price;
                                                                        matchable_quant = total_quantity;
                                                                        limitable_quant = arrival.order_quantity - total_quantity;
                                                                    }
                                                                }
                                                    
                                                                // Check if total quantity is superior to arrival.order_quantity
                                                                if total_quantity >= arrival.order_quantity {
                                                                    // Perform your operation here
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                                    // Perform your operation here
                                                                    let mut remaining_quantity = arrival.order_quantity;
                                                                    let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                        .range(fill_bid_price..=highest_bid_price)
                                                                        .rev()
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                      
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                            
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                           
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                   
                                                                                    ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                                
                                                                                              
                    
                                                                                                tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                                let tns_message = Structs::TimeSale(tns);
                                                                                                if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                let tplast_message = Structs::Last(tplast);
                                                                                                if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                               
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                                
                                                                                            remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                          
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                          
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                           let tplast_message = Structs::Last(tplast);
                                                                                           if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                           let volume_message = Structs::Volume(volume_struct);
                                                                                           if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                           ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                           ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                           mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                               quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                bid_mbp.mbp.remove(&price);
                                                                                bid_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                    
                                                                
                                                                } else if total_quantity < arrival.order_quantity {
                                                                    // mivadika sell limit ny unfilled order
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched:{} matched, {} entered to book.", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.price,matchable_quant,arrival.order_quantity - matchable_quant );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                                    let mut remaining_quantity = matchable_quant;
                                                                    let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                        .range(lowest_disponible_price..=highest_bid_price)
                                                                        .rev()
                                                                        .map(|(price, _)| *price)
                                                                        .collect();
                                                                    for price in keys_to_remove {
                                                                        let mut quants_empty = false;
                                                                        if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                            while !quantities.is_empty() && remaining_quantity > 0 {
                                                                                let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                    // If the first quantity can be fully consumed
                                                                                    remaining_quantity -= quantity;
                                                                                    quantities.remove(0); // Remove the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                       
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                           
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                           let tplast_message = Structs::Last(tplast);
                                                                                           if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                        }
                                                                                    }
                                                                                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                    // If the first quantity cannot be fully consumed
                                                                                    quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                   
                                                                                    ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids[0];
                                                                                        if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                            trader_order_struct.order_quantity -= remaining_quantity;
                                                                                            ////////////////////////////////////////////////////////////////////////////////////
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                                
                                                                                              
                    
                                                                                                let tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                                let tns_message = Structs::TimeSale(tns);
                                                                                                if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                                let tplast_message = Structs::Last(tplast);
                                                                                                if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                                last_dyn.price = trader_order_struct.price;
                                                                                                
                                                                                                let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                                let volume_message = Structs::Volume(volume_struct);
                                                                                                if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                                ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                                
                                                                                            remaining_quantity = 0;
                                                                                        }
                                                                                        
                                                                                    }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                    quantities.remove(0);
                                                                                    bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                    remaining_quantity = 0;
                                                                                    if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                        let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                        // Now you can use order_id for further operations
                                                                                        ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                        ex_iceberg(&mut iceberg_struct,order_ids,&tx2,&tx_broker);
                                                                                        if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                           
                                                                                                //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                                let trade_id = id_i64();
                                                                                                let current_time = Utc::now().timestamp_micros(); 
                                                                                                trade = match_struct_limit(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                                let trade_message = Structs::MatchStruct(trade);
                                                                                               if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            //////////////////////////////////////////////////////////////////////////////////////////
                                                                                           
                                                                                          
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                           
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                        }
                                                                                    }
                                                                                }
                                                                                 
                                                                                
                                                                            }
                                                                            if quantities.is_empty() {
                                                                                // If the vector is fully consumed
                                                                               quants_empty = true;
                                                                            }
                    
                                                                        }
                                                                        if quants_empty {
                                                                            bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                                bid_mbp.mbp.remove(&price);
                                                                                bid_map.map.remove(&price);
                                                                                quants_empty = false;
                                                                        }
                                                                    }
                                                                     ///////////////////////////////////////////////////////////////////////////////////////////
                                                                     let ask_structi = TraderOrderStruct {
                                                                       
                                                                        market: arrival.market.clone(),
                                                                        broker_identifier: arrival.broker_identifier.clone(),
                                                                        unix_time: Utc::now().timestamp_micros(),
                                                                        trader_identifier: arrival.trader_identifier,
                                                                        order_identifier: order_ids,
                                                                        order_quantity:limitable_quant,
                                                                        order_side: arrival.order_side.clone(),
                                                                        expiration:arrival.expiration.clone(),
                                                                        price: arrival.price,
                                                                        pointing_at:arrival.pointing_at,
                                                                    };
                                                                    ask_struct.insert(order_ids, ask_structi);//insert in highest disponible price
                        
                                                                    // Inserting into bid_map
                                                                    ask_map.map.entry(arrival.price)
                                                                    .and_modify(|vec| vec.push(order_ids))
                                                                    .or_insert_with(|| vec![order_ids]);
                        
                                                                    // Inserting into bid_mbo
                                                                    match ask_mbo.mbo.entry(arrival.price) {
                                                                    Entry::Occupied(mut entry) => {
                                                                        entry.get_mut().push(limitable_quant);
                                                                    }
                                                                    Entry::Vacant(entry) => {
                                                                        entry.insert(vec![limitable_quant]);
                                                                    }
                                                                    }
                        
                                                                    // Inserting into bid_mbp
                                                                    *ask_mbp.mbp.entry(arrival.price).or_insert(0) += limitable_quant;
                                            
                                                                    
                        
                                                                    let ask_structii = TraderOrderStruct {
                                                                       
                                                                        market: arrival.market.clone(),
                                                                        broker_identifier: arrival.broker_identifier.clone(),
                                                                        unix_time: Utc::now().timestamp_micros(),
                                                                        trader_identifier: arrival.trader_identifier,
                                                                        order_identifier: order_ids,
                                                                        order_quantity:limitable_quant,
                                                                        order_side: arrival.order_side.clone(),
                                                                        expiration:arrival.expiration.clone(),
                                                                        price: arrival.price,
                                                                        pointing_at:arrival.pointing_at,
                                                                    };
                        
                                                                    let order_message = Structs::TraderOrderStruct(ask_structii);
                                                                    if let Err(e) = tx_broker.send(order_message) {
                                                                        eprintln!("Failed to send message: {:?}", e);
                                                                    }
                    
                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market.clone(),limitable_quant,arrival.order_side,arrival.expiration,arrival.price,order_ids );
                                                                    message_limit_taker(&arrival,&tx_broker,string_m.clone());
                                                                    mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), limitable_quant, arrival.price, 1, &tx_market,&config);
                    
                                                                    ///////////////////////////////////////////////////////////////////////////////////////////
                                                                    
                                                                    
                                                                }
                                                            }
                                                        }
                                                    }
                    
                                                }
                                                
                                            },
                                            
                                    } 
                                   
                                    
                                }
                                _ => {
                                    // Handle any other cases
                                }
                        }
                        }
                        Structs::MarketOrder(arrival) => {
                            match arrival.order_side {
                                OrderSide::Long => {
                                    let order_idb = arrival.order_identifier.unwrap_or_else(id_i64);
                                    let unixtimeb = Utc::now().timestamp_micros(); 
                                    let lowest_ask_price = lowest_ask(&ask_mbp);
                                        match lowest_ask_price {
                    /*hhhhhhhhhhhhhhhhhhhhhhh */        None => {
                                            if arrival.expiration != OrderExpiration::FOK && arrival.expiration != OrderExpiration::IOC{
                                                let bid_structi = TraderOrderStruct {
                                                    
                                                    market: arrival.market.clone(),
                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                    unix_time: unixtimeb,
                                                    trader_identifier: arrival.trader_identifier,
                                                    order_identifier: order_idb,
                                                    order_quantity:arrival.order_quantity,
                                                    order_side: arrival.order_side.clone(),
                                                    expiration:arrival.expiration.clone(),
                                                    price: last_dyn.price,
                                                    pointing_at:arrival.pointing_at,
                                                };
                                                bid_struct.insert(order_idb, bid_structi);//insert in last traded price
                    
                                                // Inserting into bid_map
                                                bid_map.map.entry(last_dyn.price)
                                                .and_modify(|vec| vec.push(order_idb))
                                                .or_insert_with(|| vec![order_idb]);
                    
                                                // Inserting into bid_mbo
                                                match bid_mbo.mbo.entry(last_dyn.price) {
                                                Entry::Occupied(mut entry) => {
                                                    entry.get_mut().push(arrival.order_quantity);
                                                }
                                                Entry::Vacant(entry) => {
                                                    entry.insert(vec![arrival.order_quantity]);
                                                }
                                                }
                                                
                                                // Inserting into bid_mbp
                                                *bid_mbp.mbp.entry(last_dyn.price).or_insert(0) += arrival.order_quantity;
                        
                                                // Inserting or updating the trader_order_identifier
                                    
                                                let bid_structii = TraderOrderStruct {
                                                
                                                    market: arrival.market.clone(),
                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                    unix_time: unixtimeb,
                                                    trader_identifier: arrival.trader_identifier,
                                                    order_identifier: order_idb,
                                                    order_quantity:arrival.order_quantity,
                                                    order_side: arrival.order_side.clone(),
                                                    expiration:arrival.expiration.clone(),
                                                    price: last_dyn.price,
                                                    pointing_at:arrival.pointing_at,
                                                };
                                                let order_message = Structs::TraderOrderStruct(bid_structii);
                                                if let Err(e) = tx_broker.send(order_message) {
                                                    eprintln!("Failed to send message: {:?}", e);
                                                }
                    
                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market.clone(),arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),last_dyn.price,order_idb );
                                                message_market_taker(&arrival,&tx_broker,string_m);
                    
                                                        mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), arrival.order_quantity, last_dyn.price, 1, &tx_market,&config);
                                               
                                            } else {
                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),last_dyn.price );
                                                message_market_taker(&arrival,&tx_broker,string_m);
                                            }
                                            },
                                            Some(pricea) => { // eto ny tena operation
                    
                                                match arrival.expiration {
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhh */   OrderExpiration::FOK => {
                                                        // Handle FOK expiration
                                                        if let Some(lowest_ask_price) = lowest_ask_price {
                                                            let mut total_quantity = 0;
                                                            let mut fill_ask_price = lowest_ask_price;
                                                           
                                                            for (&price, &quantity) in ask_mbp.mbp.range(lowest_ask_price..) {
                                                                total_quantity += quantity;
                                                                if total_quantity >= arrival.order_quantity {
                                                                    // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                    fill_ask_price = price;
                                                                    break; // No need to continue iterating once the condition is met
                                                                }
                                                            }
                                                
                                                            // Check if total quantity is superior to arrival.order_quantity
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */  if total_quantity >= arrival.order_quantity {
                                                                // Perform your operation here
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,lowest_ask_price );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                                let mut remaining_quantity = arrival.order_quantity;
                                                                let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                    .range(lowest_ask_price..=fill_ask_price)
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                    
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                   
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                        // trader_order_struct now contains the TraderOrderStruct that was removed from ask_struct
                                                                                        // You can now use trader_order_struct for further operations
                                                                                     
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                                                                                                                 
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                       
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                          
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                     
                                                                                            remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                remaining_quantity = 0;
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                        // trader_order_struct now contains the TraderOrderStruct that was removed from ask_struct
                                                                                        // You can now use trader_order_struct for further operations
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                        
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                                                       
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                        ask_mbp.mbp.remove(&price);
                                                                        ask_map.map.remove(&price);
                                                                        quants_empty = false;
                                                                    }
                                                                }
                                                               
                                                                
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */   } else {
                                                                // Do nothing
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,lowest_ask_price );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                                
                                                            }
                                                        } 
                                                    }
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */  OrderExpiration::IOC => {
                                                        // Handle IOC expiration
                                                        if let Some(lowest_ask_price) = lowest_ask_price {
                                                            let mut total_quantity = 0;
                                                            let mut fill_ask_price = lowest_ask_price;
                                                            let mut highest_disponible_price = lowest_ask_price;
                                                            let mut matchable_quant :i32 = 0;
                                                           
                                                            for (&price, &quantity) in ask_mbp.mbp.range(lowest_ask_price..) {
                                                                total_quantity += quantity;
                                                                if total_quantity >= arrival.order_quantity {
                                                                    // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                    fill_ask_price = price;
                                                                    break; // No need to continue iterating once the condition is met
                                                                }else if total_quantity < arrival.order_quantity {
                                                                    // Update highest disponible price
                                                                    highest_disponible_price = price;
                                                                    matchable_quant = total_quantity;
                                                                }
                                                            }
                                                
                                                            // Check if total quantity is superior to arrival.order_quantity
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    if total_quantity >= arrival.order_quantity {
                                                                // Perform your operation here
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,lowest_ask_price );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                    
                                                                let mut remaining_quantity = arrival.order_quantity;
                                                                let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                    .range(lowest_ask_price..=fill_ask_price)
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                        // trader_order_struct now contains the TraderOrderStruct that was removed from ask_struct
                                                                                        // You can now use trader_order_struct for further operations
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                        
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone()); 
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                          
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            // Insert the TimeSale into the tns DashMap
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                            
                                                                                        remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                remaining_quantity = 0;
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                        // trader_order_struct now contains the TraderOrderStruct that was removed from ask_struct
                                                                                        // You can now use trader_order_struct for further operations
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                       
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            ask_mbp.mbp.remove(&price);
                                                                            ask_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                    
                                                                }
                                                                
                                                               
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    } else if total_quantity < arrival.order_quantity{
                                                                // Do something with highest_disponible_price and matchable quantity, avela ny reste
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, partially matched:{} matched, {} cancelled.", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,lowest_ask_price,matchable_quant,arrival.order_quantity - matchable_quant );
                                                               message_market_taker(&arrival,&tx_broker,string_m);
                                                                let mut remaining_quantity = matchable_quant;
                                                                let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                    .range(lowest_ask_price..=highest_disponible_price)
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                        // trader_order_struct now contains the TraderOrderStruct that was removed from ask_struct
                                                                                        // You can now use trader_order_struct for further operations
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                       
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                                        eprintln!("Failed to send message: {:?}", e);
                                                                                                    }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                       
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                          
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                            
                                                                                        remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                remaining_quantity = 0;
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                        // trader_order_struct now contains the TraderOrderStruct that was removed from ask_struct
                                                                                        // You can now use trader_order_struct for further operations
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                        eprintln!("Failed to send message: {:?}", e);
                                                                                                    }
                                                                                                    if let Err(e) = tx_position.send(trade_message) {
                                                                                                        eprintln!("Failed to send message: {:?}", e);
                                                                                                    }
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                       
                    
                                                                                        let tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            ask_mbp.mbp.remove(&price);
                                                                            ask_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                                                                }
                                                                
                                                        }
                                                    }
                                                }
                                                
                                                    _ => {
                                                        // Handle all other cases
                                                        if let Some(lowest_ask_price) = lowest_ask_price {
                                                            let mut total_quantity = 0;
                                                            let mut fill_ask_price = lowest_ask_price;
                                                            let mut highest_disponible_price = lowest_ask_price;
                                                            let mut matchable_quant :i32 = 0;
                                                            let mut limitable_quant :i32 =0;
                                                           
                                                            for (&price, &quantity) in ask_mbp.mbp.range(lowest_ask_price..) {
                                                                total_quantity += quantity;
                                                                if total_quantity >= arrival.order_quantity{
                                                                    // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                    fill_ask_price = price;
                                                                    break; // No need to continue iterating once the condition is met
                                                                }else if total_quantity < arrival.order_quantity {
                                                                    // Update highest disponible price
                                                                    highest_disponible_price = price;
                                                                    matchable_quant = total_quantity;
                                                                    limitable_quant = arrival.order_quantity - total_quantity;
                                                                   
                                                            
                    
                                                                }
                                                            }
                                                
                                                            // Check if total quantity is superior to arrival.order_quantity
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    if total_quantity >= arrival.order_quantity {
                                                                // Perform your operation here
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,lowest_ask_price );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                                let mut remaining_quantity = arrival.order_quantity;
                                                                let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                    .range(lowest_ask_price..=fill_ask_price)
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                       
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                        
                    
                                                                                        let tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                    
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                               
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                  
                                                                                    if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                     
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                        eprintln!("Failed to send message: {:?}", e);
                                                                                                    }
                                                                                                    if let Err(e) = tx_position.send(trade_message) {
                                                                                                        eprintln!("Failed to send message: {:?}", e);
                                                                                                    }
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                            
                                                                                        remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                               
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                               
                                                                                remaining_quantity = 0;
                                                                              
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    // Now you can use order_id for further operations
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                       
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                           
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                           
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                       
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                           
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            ask_mbp.mbp.remove(&price);
                                                                            ask_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                                                                }
                                                               
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    } else if total_quantity < arrival.order_quantity{
                                                                // mivadika buy limit ny unfilled order,Do something with highest_disponible_price,matchable quant and limitable quant
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, partially matched:{} matched, {} entered to book.", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,lowest_ask_price,matchable_quant,arrival.order_quantity - matchable_quant );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                                let mut remaining_quantity = matchable_quant;
                                                                let keys_to_remove: Vec<i32> = ask_mbo.mbo
                                                                    .range(lowest_ask_price..=highest_disponible_price)
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = ask_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                                                                           
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                               
                                                                                quantities.remove(0); // Remove the first quantity
                                                                               
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                               
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                       
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                           
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                          
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                
                                                                                        
                                                                                      
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                      
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);                            
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                      
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                      
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                      
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = ask_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                            
                                                                                           
                    
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                           let volume_message = Structs::Volume(volume_struct);
                                                                                           if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                           ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                           ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                           mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                          
                                                                                        remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                               
                                                                                ask_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                               
                                                                                remaining_quantity = 0;
                                                                              
                                                                                if let Some(maker_ids) = ask_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = ask_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                          
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_idb, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                          
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                  
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                     
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                   
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                   
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                    
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                 
                                                                                    }
                                                                                }
                                                                            }
                                                                            
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        ask_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            ask_mbp.mbp.remove(&price);
                                                                            ask_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                                                                }
                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                let bid_structi = TraderOrderStruct {
                                                                    
                                                                    market: arrival.market.clone(),
                                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                                    unix_time: Utc::now().timestamp_micros(),
                                                                    trader_identifier: arrival.trader_identifier,
                                                                    order_identifier: order_idb,
                                                                    order_quantity:limitable_quant,
                                                                    order_side: arrival.order_side.clone(),
                                                                    expiration:arrival.expiration.clone(),
                                                                    price: highest_disponible_price,
                                                                    pointing_at:arrival.pointing_at,
                                                                };
                                                                bid_struct.insert(order_idb, bid_structi);//insert in highest disponible price
                                                              
                                                                // Inserting into bid_map
                                                                bid_map.map.entry(highest_disponible_price)
                                                                .and_modify(|vec| vec.push(order_idb))
                                                                .or_insert_with(|| vec![order_idb]);
                    
                                                                // Inserting into bid_mbo
                                                                match bid_mbo.mbo.entry(highest_disponible_price) {
                                                                Entry::Occupied(mut entry) => {
                                                                    entry.get_mut().push(limitable_quant);
                                                                }
                                                                Entry::Vacant(entry) => {
                                                                    entry.insert(vec![limitable_quant]);
                                                                }
                                                                }
                    
                                                                // Inserting into bid_mbp
                                                                *bid_mbp.mbp.entry(highest_disponible_price).or_insert(0) += arrival.order_quantity - matchable_quant;
                                        
                                                                let bid_structii = TraderOrderStruct {
                                                                  
                                                                    market: arrival.market.clone(),
                                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                                    unix_time: Utc::now().timestamp_micros(),
                                                                    trader_identifier: arrival.trader_identifier,
                                                                    order_identifier: order_idb,
                                                                    order_quantity:limitable_quant,
                                                                    order_side: arrival.order_side.clone(),
                                                                    expiration:arrival.expiration.clone(),
                                                                    price: highest_disponible_price,
                                                                    pointing_at:arrival.pointing_at,
                                                                };
                    
                                                                let order_message = Structs::TraderOrderStruct(bid_structii);
                                                                if let Err(e) = tx_broker.send(order_message) {
                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                }
                    
                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market.clone(),limitable_quant,arrival.order_side.clone(),arrival.expiration.clone(),highest_disponible_price,order_idb );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                        mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), limitable_quant, highest_disponible_price, 1, &tx_market,&config);
                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                
                                                               
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                            },
                                            
                                    }
                                   
                                }
                                OrderSide::Short => {
                    
                                    let order_ids = arrival.order_identifier.unwrap_or_else(id_i64);
                                    let unixtimes = Utc::now().timestamp_micros();
                                    let highest_bid_price = highest_bid(&bid_mbp);
                                        match highest_bid_price {
                    /*hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh */    None => {
                                            if arrival.expiration != OrderExpiration::FOK && arrival.expiration != OrderExpiration::IOC {
                                                let ask_structi = TraderOrderStruct {
                                                  
                                                    market: arrival.market.clone(),
                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                    unix_time: unixtimes,
                                                    trader_identifier: arrival.trader_identifier,
                                                    order_identifier: order_ids,
                                                    order_quantity:arrival.order_quantity,
                                                    order_side: arrival.order_side.clone(),
                                                    expiration:arrival.expiration.clone(),
                                                    price: last_dyn.price,
                                                    pointing_at:arrival.pointing_at,
                                                };
                                                ask_struct.insert(order_ids, ask_structi); //insert in last traded price
                    
                                                // Inserting into bid_map
                                                ask_map.map.entry(last_dyn.price)
                                                .and_modify(|vec| vec.push(order_ids))
                                                .or_insert_with(|| vec![order_ids]);
                    
                                                // Inserting into bid_mbo
                                                match ask_mbo.mbo.entry(last_dyn.price) {
                                                Entry::Occupied(mut entry) => {
                                                    entry.get_mut().push(arrival.order_quantity);
                                                }
                                                Entry::Vacant(entry) => {
                                                    entry.insert(vec![arrival.order_quantity]);
                                                }
                                                }
                    
                                                // Inserting into bid_mbp
                                                *ask_mbp.mbp.entry(last_dyn.price).or_insert(0) += arrival.order_quantity;
                    
                    
                                                let ask_structii = TraderOrderStruct {
                                                    
                                                    market: arrival.market.clone(),
                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                    unix_time: unixtimes,
                                                    trader_identifier: arrival.trader_identifier,
                                                    order_identifier: order_ids,
                                                    order_quantity:arrival.order_quantity,
                                                    order_side: arrival.order_side.clone(),
                                                    expiration:arrival.expiration.clone(),
                                                    price: last_dyn.price,
                                                    pointing_at:arrival.pointing_at,
                                                };
                    
                                                let order_message = Structs::TraderOrderStruct(ask_structii);
                                                if let Err(e) = tx_broker.send(order_message) {
                                                    eprintln!("Failed to send message: {:?}", e);
                                                }
                    
                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market.clone(),arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),last_dyn.price,order_ids );
                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                        mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), arrival.order_quantity, last_dyn.price, 1, &tx_market,&config);
                                            
                                            } else {
                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),last_dyn.price );
                                                message_market_taker(&arrival,&tx_broker,string_m);
                                            }    
                                            },
                                            Some(pricea) => { // eto ny tena operation
                    
                                                match arrival.expiration {
                                                    OrderExpiration::FOK => {
                                                        // Handle FOK expiration
                                                        if let Some(highest_bid_price) = highest_bid_price {
                                                            let mut total_quantity = 0;
                                                            let mut fill_bid_price = highest_bid_price;
                                                            
                                                            for (&price, &quantity) in bid_mbp.mbp.range(..=highest_bid_price).rev() {
                                                                total_quantity += quantity;
                                                                if total_quantity >= arrival.order_quantity {
                                                                    // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                    fill_bid_price = price;
                                                                    break; // No need to continue iterating once the condition is met
                                                                }
                                                            }
                                                
                                                            // Check if total quantity is superior to arrival.order_quantity
                                                            if total_quantity >= arrival.order_quantity {
                                                                // Perform your operation here
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,highest_bid_price );
                                                                message_market_taker(&arrival,&tx_broker,string_m);  
                                                           
                                                                let mut remaining_quantity = arrival.order_quantity;
                                                                let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                    .range(fill_bid_price..=highest_bid_price)
                                                                    .rev()
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                       
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                        
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                        ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                        mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                           
                                                                                           
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                           let tplast_message = Structs::Last(tplast);
                                                                                           if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                           let volume_message = Structs::Volume(volume_struct);
                                                                                           if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                           ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                           ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                           mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                           
                                                                                            remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                remaining_quantity = 0;
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                        ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                        mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            bid_mbp.mbp.remove(&price);
                                                                            bid_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                                                                }
                                                                
                                                            } else {
                                                                // Do nothing
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, Not matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),highest_bid_price );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                            }
                                                        } 
                                                    }
                                                    OrderExpiration::IOC => {
                                                        // Handle IOC expiration
                                                        if let Some(highest_bid_price) = highest_bid_price {
                                                            let mut total_quantity = 0;
                                                            let mut fill_bid_price = highest_bid_price;
                                                            let mut lowest_disponible_price = highest_bid_price;
                                                            let mut matchable_quant :i32 = 0;
                                                            
                                                            for (&price, &quantity) in bid_mbp.mbp.range(..=highest_bid_price).rev() {
                                                                total_quantity += quantity;
                                                                if total_quantity >= arrival.order_quantity {
                                                                    // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                    fill_bid_price = price;
                                                                    break; // No need to continue iterating once the condition is met
                                                                }else if total_quantity < arrival.order_quantity {
                                                                    // Update highest disponible price
                                                                    lowest_disponible_price = price;
                                                                    matchable_quant = total_quantity;
                                                                }
                                                            }
                                                
                                                            // Check if total quantity is superior to arrival.order_quantity
                                                            if total_quantity >= arrival.order_quantity {
                                                                // Perform your operation here
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,highest_bid_price );
                                                                message_market_taker(&arrival,&tx_broker,string_m); 
                                                           
                                                                let mut remaining_quantity = arrival.order_quantity;
                                                                let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                    .range(fill_bid_price..=highest_bid_price)
                                                                    .rev()
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                     
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                       let tplast_message = Structs::Last(tplast);
                                                                                       if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                       
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                     let volume_message = Structs::Volume(volume_struct);
                                                                                     if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                    let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                    message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone()); 
                                                                                     ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                     ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);  
                                                                                     mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                            
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                       
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                         
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());   
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                        ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                        mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                           
                                                                                        remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                remaining_quantity = 0;
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                      
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                         let tns_message = Structs::TimeSale(tns);
                                                                                         if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                         let tplast_message = Structs::Last(tplast);
                                                                                         if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                                message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            bid_mbp.mbp.remove(&price);
                                                                            bid_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                                                                }
                                                                
                                                            } else if total_quantity < arrival.order_quantity {
                                                                // avela any ny unfilled order
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, partially matched:{} matched, {} cancelled.", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,highest_bid_price,matchable_quant,arrival.order_quantity - matchable_quant );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                                let mut remaining_quantity = matchable_quant;
                                                                let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                    .range(lowest_disponible_price..=highest_bid_price)
                                                                    .rev()
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                      
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                         let tplast_message = Structs::Last(tplast);
                                                                                         if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                        ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                        mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                          
                                                                                            tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                        eprintln!("Failed to send message: {:?}", e);
                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                            
                                                                                            remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                remaining_quantity = 0;
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                     
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            bid_mbp.mbp.remove(&price);
                                                                            bid_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                                                                }
                                                               
                                                            }
                                                        }
                                                    }
                                                    _ => {
                                                        // Handle all other cases
                                                        if let Some(highest_bid_price) = highest_bid_price {
                                                            let mut total_quantity = 0;
                                                            let mut fill_bid_price = highest_bid_price;
                                                            let mut lowest_disponible_price = highest_bid_price;
                                                            let mut matchable_quant :i32 = 0;
                                                            let mut limitable_quant :i32 =0;
                                                            
                                                            for (&price, &quantity) in bid_mbp.mbp.range(..=highest_bid_price).rev() {
                                                                total_quantity += quantity;
                                                                if total_quantity >= arrival.order_quantity {
                                                                    // If total quantity exceeds or equals arrival order quantity, update highest ask price
                                                                    fill_bid_price = price;
                                                                    break; // No need to continue iterating once the condition is met
                                                                }else if total_quantity < arrival.order_quantity {
                                                                    // Update highest disponible price
                                                                    lowest_disponible_price = price;
                                                                    matchable_quant = total_quantity;
                                                                    limitable_quant = arrival.order_quantity - total_quantity;
                                                                }
                                                            }
                                                
                                                            // Check if total quantity is superior to arrival.order_quantity
                                                            if total_quantity >= arrival.order_quantity {
                                                                // Perform your operation here
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, totally matched", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,highest_bid_price );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                                // Perform your operation here
                                                                let mut remaining_quantity = arrival.order_quantity;
                                                                let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                    .range(fill_bid_price..=highest_bid_price)
                                                                    .rev()
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                       
                                                                                       
                    
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                       let tns_message = Structs::TimeSale(tns);
                                                                                       if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                         let volume_message = Structs::Volume(volume_struct);
                                                                                         if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                         ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                         ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                         mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                    
                                                                                            let tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                             let tns_message = Structs::TimeSale(tns);
                                                                                             if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                           
                                                                                        remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                remaining_quantity = 0;
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                     
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            bid_mbp.mbp.remove(&price);
                                                                            bid_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                                                                }
                                                               
                                                            
                                                            } else if total_quantity < arrival.order_quantity {
                                                                // mivadika sell limit ny unfilled order
                                                                let string_m = format! ("{} {}, {} {} {} market order at {} price level, partially matched:{} matched, {} entered to book.", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,highest_bid_price,matchable_quant,arrival.order_quantity - matchable_quant );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                                let mut remaining_quantity = matchable_quant;
                                                                let keys_to_remove: Vec<i32> = bid_mbo.mbo
                                                                    .range(lowest_disponible_price..=highest_bid_price)
                                                                    .rev()
                                                                    .map(|(price, _)| *price)
                                                                    .collect();
                                                                for price in keys_to_remove {
                                                                    let mut quants_empty = false;
                                                                    if let Some(quantities) = bid_mbo.mbo.get_mut(&price) {
                                                                        while !quantities.is_empty() && remaining_quantity > 0 {
                                                                            let quantity = quantities[0]; // Get the first quantity
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */             if quantity < remaining_quantity {
                                                                                // If the first quantity can be fully consumed
                                                                                remaining_quantity -= quantity;
                                                                                quantities.remove(0); // Remove the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                     
                                                                                        let tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                    
                                                                                    }
                                                                                }
                                                                                
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */  } else if quantity > remaining_quantity {
                                                                                // If the first quantity cannot be fully consumed
                                                                                quantities[0] -= remaining_quantity; // Reduce the first quantity
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= remaining_quantity);
                                                                                
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids[0];
                                                                                    if let Some(mut trader_order_struct) = bid_struct.get_mut(&maker_id) {
                                                                                        trader_order_struct.order_quantity -= remaining_quantity;
                                                                                        ////////////////////////////////////////////////////////////////////////////////////
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, remaining_quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                                if let Err(e) = tx_position.send(trade_message) {
                                                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                                                }
                                                                                            ///////////////////////////////////////////////////////////////////////////////////////////
                                                                                        
                                                                                            let tns = time_sale(&config, Utc::now().timestamp_micros(), remaining_quantity, arrival.order_side.clone(), price);
                                                                                            let tns_message = Structs::TimeSale(tns);
                                                                                            if let Err(e) = tx_market.send(tns_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                            let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                            let tplast_message = Structs::Last(tplast);
                                                                                            if let Err(e) = tx_market.send(tplast_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                            last_dyn.price = trader_order_struct.price;
                                                                                            
                                                                                            let volume_struct = volume_struct(Utc::now().timestamp_micros(), remaining_quantity,&arrival.order_side.clone(),price,&config);
                                                                                            let volume_message = Structs::Volume(volume_struct);
                                                                                            if let Err(e) = tx_market.send(volume_message) {
                                                                                                        eprintln!("Failed to send message: {:?}", e);
                                                                                                    }
                                                                                            let string_m = format! ("{} {}, {} {} {} limit order at {} price level, partially matched, {} matched, {} remaining.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price,remaining_quantity,quantity-remaining_quantity );
                                                                                            message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                            ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), remaining_quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                           
                                                                                        remaining_quantity = 0;
                                                                                    }
                                                                                    
                                                                                }
                    
                    
                    /*aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa */                 } else if quantity == remaining_quantity{
                                                                                quantities.remove(0);
                                                                                bid_mbp.mbp.entry(price).and_modify(|e| *e -= quantity);
                                                                                remaining_quantity = 0;
                                                                                if let Some(maker_ids) = bid_map.map.get_mut(&price) {
                                                                                    let maker_id = maker_ids.remove(0); // Remove the first order ID and get its value
                                                                                    // Now you can use order_id for further operations
                                                                                    ex_iceberg(&mut iceberg_struct,maker_id,&tx2,&tx_broker);
                                                                                    if let Some((_,trader_order_struct)) = bid_struct.remove(&maker_id) {
                                                                                        
                                                                                            //////////////////////////////////////////////////////////////////////////////////Trade
                                                                                            let trade_id = id_i64();
                                                                                            let current_time = Utc::now().timestamp_micros(); 
                                                                                            trade = match_struct_market(&arrival, &trader_order_struct, trade_id, order_ids, current_time, quantity);
                                                                                            let trade_message = Structs::MatchStruct(trade);
                                                                                           if let Err(e) = tx_broker.send(trade_message.clone()) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                            if let Err(e) = tx_position.send(trade_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        
                                                                                        //////////////////////////////////////////////////////////////////////////////////////////
                                                                                   
                                                                                        tns = time_sale(&config, Utc::now().timestamp_micros(), quantity, arrival.order_side.clone(), price);
                                                                                        let tns_message = Structs::TimeSale(tns);
                                                                                        if let Err(e) = tx_market.send(tns_message) {
                                                                                            eprintln!("Failed to send message: {:?}", e);
                                                                                        }
                    
                                                                                        let tplast = tp_last(Utc::now().timestamp_micros(),trader_order_struct.price,&config);
                                                                                        let tplast_message = Structs::Last(tplast);
                                                                                        if let Err(e) = tx_market.send(tplast_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                    
                                                                                        last_dyn.price = trader_order_struct.price;
                                                                                        
                                                                                        let volume_struct = volume_struct(Utc::now().timestamp_micros(), quantity,&arrival.order_side.clone(),price,&config);
                                                                                        let volume_message = Structs::Volume(volume_struct);
                                                                                        if let Err(e) = tx_market.send(volume_message) {
                                                                                                eprintln!("Failed to send message: {:?}", e);
                                                                                            }
                                                                                        let string_m = format! ("{} {}, {} {} {} limit order at {} price level, totally matched.", Utc::now().timestamp_micros(),trader_order_struct.market,quantity,trader_order_struct.order_side.clone(),trader_order_struct.expiration.clone(),price );
                                                                                        message_limit_maker(&trader_order_struct,&tx_broker,string_m.clone());
                                                                                        ex_stop (&stop_struct,&mut stop_map,&last_dyn,&tx2,&tx_broker);
                                                                                            ex_stop_limit(&stop_limit_struct, &mut stop_limit_map, &last_dyn, &tx2, &tx_broker);
                                                                                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), quantity, trader_order_struct.price, -1, &tx_market,&config);
                                                                                       
                    
                                                                                    }
                                                                                }
                                                                            }
                                                                             
                                                                            
                                                                        }
                                                                        if quantities.is_empty() {
                                                                            // If the vector is fully consumed
                                                                            quants_empty = true;
                                                                        }
                    
                                                                    }
                                                                    if quants_empty {
                                                                        bid_mbo.mbo.remove(&price); // Remove the entry from the ask_mbo map
                                                                            bid_mbp.mbp.remove(&price);
                                                                            bid_map.map.remove(&price);
                                                                            quants_empty = false;
                                                                    }
                                                                }
                                                                 ///////////////////////////////////////////////////////////////////////////////////////////
                                                                 let ask_structi = TraderOrderStruct {
                                                                
                                                                    market: arrival.market.clone(),
                                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                                    unix_time: Utc::now().timestamp_micros(),
                                                                    trader_identifier: arrival.trader_identifier,
                                                                    order_identifier: order_ids,
                                                                    order_quantity:limitable_quant,
                                                                    order_side: arrival.order_side.clone(),
                                                                    expiration:arrival.expiration.clone(),
                                                                    price: lowest_disponible_price,
                                                                    pointing_at:arrival.pointing_at,
                                                                };
                                                                ask_struct.insert(order_ids, ask_structi);//insert in highest disponible price
                    
                                                                // Inserting into bid_map
                                                                ask_map.map.entry(lowest_disponible_price)
                                                                .and_modify(|vec| vec.push(order_ids))
                                                                .or_insert_with(|| vec![order_ids]);
                    
                                                                // Inserting into bid_mbo
                                                                match ask_mbo.mbo.entry(lowest_disponible_price) {
                                                                Entry::Occupied(mut entry) => {
                                                                    entry.get_mut().push(limitable_quant);
                                                                }
                                                                Entry::Vacant(entry) => {
                                                                    entry.insert(vec![limitable_quant]);
                                                                }
                                                                }
                    
                                                                // Inserting into bid_mbp
                                                                *ask_mbp.mbp.entry(lowest_disponible_price).or_insert(0) += limitable_quant;
                                        
                    
                                                                let ask_structii = TraderOrderStruct {
                                                                  
                                                                    market: arrival.market.clone(),
                                                                    broker_identifier: arrival.broker_identifier.clone(),
                                                                    unix_time: Utc::now().timestamp_micros(),
                                                                    trader_identifier: arrival.trader_identifier,
                                                                    order_identifier: order_ids,
                                                                    order_quantity:limitable_quant,
                                                                    order_side: arrival.order_side.clone(),
                                                                    expiration:arrival.expiration.clone(),
                                                                    price: lowest_disponible_price,
                                                                    pointing_at:arrival.pointing_at,
                                                                };
                    
                                                                let order_message = Structs::TraderOrderStruct(ask_structii);
                                                                if let Err(e) = tx_broker.send(order_message) {
                                                                    eprintln!("Failed to send message: {:?}", e);
                                                                }
                    
                                                                let string_m = format! ("{} {}, {} {} {} limit order at {} price level, id {} added to order-book", Utc::now().timestamp_micros(),arrival.market.clone(),arrival.order_quantity,arrival.order_side.clone(),arrival.expiration.clone(),lowest_disponible_price,order_ids );
                                                                message_market_taker(&arrival,&tx_broker,string_m);
                                                        mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), limitable_quant, lowest_disponible_price, 1, &tx_market,&config);
                    
                                                                ///////////////////////////////////////////////////////////////////////////////////////////
                                                                
                                                                
                                                            }
                                                        }
                                                    }
                                                }
                                                
                                            },
                                            
                                    } 
                                    
                                }
                                _ => {
                                    // Handle any other cases
                                }
                        }
                           
                        }
                        Structs::StopOrder(arrival) => {
                            let order_idstop = id_i64();
                            let stop_structi = TraderStopOrderStruct {
                                market: arrival.market.clone(),
                                broker_identifier: arrival.broker_identifier.clone(),
                                unix_time: Utc::now().timestamp_micros(),
                                trader_identifier: arrival.trader_identifier,
                                order_identifier: order_idstop,
                                order_quantity:arrival.order_quantity,
                                order_side: arrival.order_side.clone(),
                                expiration:arrival.expiration.clone(),
                                trigger_price: arrival.trigger_price,
                                pointing_at:arrival.pointing_at,
                            };
                            stop_struct.insert(order_idstop, stop_structi.clone());//insert in last traded price
                    
                            // Inserting into bid_map
                            stop_map.map.entry(arrival.trigger_price)
                            .and_modify(|vec| vec.push(order_idstop))
                            .or_insert_with(|| vec![order_idstop]);
                    
                            let order_message = Structs::TraderStopOrderStruct(stop_structi);
                            if let Err(e) = tx_broker.send(order_message) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
                    
                            let string_m = format! ("{} {}, {} {} {} Stop order at {} price level, id {} waiting for execution", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.expiration,arrival.trigger_price,order_idstop );
                            message_stop(&arrival,&tx_broker,string_m);
                    
                        }
                        Structs::StopLimitOrder(arrival) => {
                            let order_idstoplimit = id_i64();
                            let stoplimit_structi = TraderStopLimitOrderStruct {
                                market: arrival.market.clone(),
                                broker_identifier: arrival.broker_identifier.clone(),
                                unix_time: Utc::now().timestamp_micros(),
                                trader_identifier: arrival.trader_identifier,
                                order_identifier: order_idstoplimit,
                                order_quantity:arrival.order_quantity,
                                order_side: arrival.order_side.clone(),
                                expiration:arrival.expiration.clone(),
                                trigger_price: arrival.trigger_price,
                                price:arrival.price,
                                pointing_at:arrival.pointing_at,
                            };
                            stop_limit_struct.insert(order_idstoplimit, stoplimit_structi.clone());//insert in last traded price
                    
                            stop_limit_map.map.entry(arrival.trigger_price)
                            .and_modify(|vec| vec.push(order_idstoplimit))
                            .or_insert_with(|| vec![order_idstoplimit]);
                    
                            let order_message = Structs::TraderStopLimitOrderStruct(stoplimit_structi);
                            if let Err(e) = tx_broker.send(order_message) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
                    
                            let string_m = format! ("{} {}, {} {} at {} price level {} Stop limit order id {} waiting for execution", Utc::now().timestamp_micros(),arrival.market,arrival.order_quantity,arrival.order_side,arrival.trigger_price,arrival.expiration,order_idstoplimit );
                            message_stoplimit(&arrival,&tx_broker,string_m);
                    
                        }
                        Structs::ModifyOrder(arrival) => {
                            let unixtime = Utc::now().timestamp_micros(); 
                                 // Retrieve the value associated with the order_identifier key from bid_struct
                        if let Some(mut order_struct) = bid_struct.get_mut(&arrival.order_identifier) { //bid
                           
                            let old_quantity = order_struct.order_quantity;
                            order_struct.order_quantity = arrival.new_quantity;
                            let new_quantity = arrival.new_quantity;
                            let price = order_struct.price;
                            if let Some(order_ids) = bid_map.map.get_mut(&price) {
                                if let Some(index) = order_ids.iter().position(|&id| id == arrival.order_identifier) {
                    
                                    if let Some(quantity_vec) = bid_mbo.mbo.get_mut(&price) {
                                        if let Some(quantity) = quantity_vec.get_mut(index) {
                                            *quantity = arrival.new_quantity;
                                        }
                                    }
                                    
                                }
                                
                            }
                            // Update bid_mbp
                            if let Some(old_sum) = bid_mbp.mbp.get_mut(&price) {
                                // Subtract the old quantity
                                *old_sum -= old_quantity;
                                // Add the new quantity
                                *old_sum += arrival.new_quantity;
                            }
                          let modf_struct = modify_struct (&config,&order_struct,unixtime,old_quantity,new_quantity);
                          let order_message = Structs::ModifiedOrderStruct(modf_struct);
                          if let Err(e) = tx_broker.send(order_message) {
                            eprintln!("Failed to send message: {:?}", e);
                        }
                           let string_m = format! ("{} {}, {} {} at {} price level modified to {} quantity, order id {} ", Utc::now().timestamp_micros(),order_struct.market,old_quantity,order_struct.order_side,order_struct.price,new_quantity,arrival.order_identifier );
                           message_modify(&arrival,&tx_broker,string_m);
                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), old_quantity, price, -1, &tx_market,&config);
                            mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), new_quantity, price, 1, &tx_market,&config);
                    
                        } else if let Some(mut order_struct) = ask_struct.get_mut(&arrival.order_identifier) { //ask 
                                let old_quantity = order_struct.order_quantity;
                                order_struct.order_quantity = arrival.new_quantity;
                                let new_quantity = arrival.new_quantity;
                                let price = order_struct.price;
                                if let Some(order_ids) = ask_map.map.get_mut(&price) {
                                    if let Some(index) = order_ids.iter().position(|&id| id == arrival.order_identifier) {
                        
                                        if let Some(quantity_vec) = ask_mbo.mbo.get_mut(&price) {
                                            if let Some(quantity) = quantity_vec.get_mut(index) {
                                                *quantity = arrival.new_quantity;
                                            }
                                        }
                                    } 
                                }
                                // Update bid_mbp
                                if let Some(old_sum) = ask_mbp.mbp.get_mut(&price) {
                                    // Subtract the old quantity
                                    *old_sum -= old_quantity;
                                    // Add the new quantity
                                    *old_sum += arrival.new_quantity;
                                }
                                // Update trader_in_history
                                let modf_struct = modify_struct (&config,&order_struct,unixtime,old_quantity,new_quantity);
                                let order_message = Structs::ModifiedOrderStruct(modf_struct);
                                if let Err(e) = tx_broker.send(order_message) {
                                    eprintln!("Failed to send message: {:?}", e);
                                }
                                let string_m = format! ("{} {}, {} {} at {} price level modified to {} quantity, order id {} ", Utc::now().timestamp_micros(),order_struct.market,old_quantity,order_struct.order_side,order_struct.price,new_quantity,arrival.order_identifier );
                                message_modify(&arrival,&tx_broker,string_m);
                                mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), old_quantity, price, -1, &tx_market,&config);
                                mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), new_quantity, price, 1, &tx_market,&config);
                        
                            } 
                            else if let Some(mut stop_structa) = stop_struct.get_mut(&arrival.order_identifier) {//stop
                                let old_quantity = stop_structa.order_quantity;
                                stop_structa.order_quantity = arrival.new_quantity;
                                let new_quantity = arrival.new_quantity;
                                
                                // Update trader_in_history
                                let modf_struct = modify_stop_struct (&config,&stop_structa,unixtime,old_quantity,new_quantity);
                                let order_message = Structs::ModifiedStopOrderStruct(modf_struct);
                                if let Err(e) = tx_broker.send(order_message) {
                                    eprintln!("Failed to send message: {:?}", e);
                                }
                                let string_m = format! ("{} {}, {} {} at {} price level modified to {} quantity, order id {} ", Utc::now().timestamp_micros(),stop_structa.market,old_quantity,stop_structa.order_side,stop_structa.trigger_price,new_quantity,arrival.order_identifier );
                                message_modify(&arrival,&tx_broker,string_m);
                            } 
                            else if let Some(mut stop_limit_structa) = stop_limit_struct.get_mut(&arrival.order_identifier) {//stop limit
                                let old_quantity = stop_limit_structa.order_quantity;
                                stop_limit_structa.order_quantity = arrival.new_quantity;
                                let new_quantity = arrival.new_quantity;
                               
                                // Update trader_in_history
                                let modf_struct = modify_stop_limit_struct (&config,&stop_limit_structa,unixtime,old_quantity,new_quantity);
                                let order_message = Structs::ModifiedStopLimitOrderStruct(modf_struct);
                                if let Err(e) = tx_broker.send(order_message) {
                                    eprintln!("Failed to send message: {:?}", e);
                                }
                                let string_m = format! ("{} {}, {} {} at {} price level modified to {} quantity, order id {} ", Utc::now().timestamp_micros(),stop_limit_structa.market,old_quantity,stop_limit_structa.order_side,stop_limit_structa.trigger_price,new_quantity,arrival.order_identifier );
                                message_modify(&arrival,&tx_broker,string_m);
                            } 
                            else {
                                let string_m = format!(
                                    "{} Order with id {} not found in any book for modification.",
                                    Utc::now().timestamp_micros(),
                                    arrival.order_identifier
                                );
                                message_modify(&arrival,&tx_broker,string_m);
                            }
                        
                        }
                        Structs::DeleteOrder(arrival) => {
                            let unixtime = Utc::now().timestamp_micros();
                                    let mut isinbid = false; 
                                    let mut isinask = false;
                                    let mut isinstop = false;
                                    let mut isinstoplimit = false;
                                    if let Some(mut order_struct) = bid_struct.get_mut(&arrival.order_identifier) {//bid
                                        let price = order_struct.price;
                                        let old_quantity = order_struct.order_quantity;
                                        isinbid = true;
                                        let mut orderids_empty = false;
                                        let mut quantvec_empty = false;
                                        if let Some(order_ids) = bid_map.map.get_mut(&price) {
                                            if let Some(index) = order_ids.iter().position(|&id| id == arrival.order_identifier) {
                                                order_ids.remove(index);
                                                 if order_ids.is_empty() {
                                                    // Remove the key from bid_map if the order_ids vector is empty
                                                   orderids_empty = true;
                                                  }
                    
                                                if let Some(quantity_vec) = bid_mbo.mbo.get_mut(&price) {
                                                   quantity_vec.remove(index) ;
                                                   if quantity_vec.is_empty() {
                                                        // Remove the key from bid_mbo if the quantity_vec is empty
                                                     quantvec_empty = true;
                                                    }     
                                                    
                                                }
                                            }
                                        }
                                        if orderids_empty {
                                            bid_map.map.remove(&price);
                                            orderids_empty = false;
                                        }
                                        if quantvec_empty {
                                            bid_mbo.mbo.remove(&price);
                                            quantvec_empty = false;
                                        }
                    
                                        let mut oldsum_zero = false;
                                        if let Some(old_sum) = bid_mbp.mbp.get_mut(&price) {
                                            *old_sum -= old_quantity;
                                            if *old_sum == 0 {
                                                // Remove the key from bid_mbp if the old_sum becomes 0
                                                oldsum_zero = true;
                                           }
                                        }
                                        if oldsum_zero {
                                            bid_mbp.mbp.remove(&price);
                                            oldsum_zero = false;
                                        }
                    
                                        let del_struct = delete_struct(&order_struct, unixtime);
                                        let order_message = Structs::DeletedOrderStruct(del_struct);
                                        if let Err(e) = tx_broker.send(order_message) {
                                            eprintln!("Failed to send message: {:?}", e);
                                        }
                                        let string_m = format! ("{} {}, {} {} at {} price level deleted, order id {} ", Utc::now().timestamp_micros(),order_struct.market,old_quantity,order_struct.order_side,order_struct.price,arrival.order_identifier );
                                        message_delete(&arrival,&tx_broker,string_m);
                                        mbp_event( Utc::now().timestamp_micros(), "bid".to_string(), old_quantity, price, -1, &tx_market,&config);
                    
                    /*hhhhhhhhhhhhhh */ } else if let Some(mut order_struct) = ask_struct.get_mut(&arrival.order_identifier) {// ask
                                        let price = order_struct.price;
                                        let old_quantity = order_struct.order_quantity;
                                        //bid_struct.remove(&arrival.order_identifier);
                                        isinask = true;
                    
                                        let mut orderids_empty = false;
                                        let mut quantvec_empty = false;
                                        if let Some(order_ids) = ask_map.map.get_mut(&price) {
                                            if let Some(index) = order_ids.iter().position(|&id| id == arrival.order_identifier) {
                                                order_ids.remove(index);
                                                 if order_ids.is_empty() {
                                                    // Remove the key from bid_map if the order_ids vector is empty
                                                    orderids_empty = true;
                                                  }
                    
                                                if let Some(quantity_vec) = ask_mbo.mbo.get_mut(&price) {
                                                   quantity_vec.remove(index) ;
                                                      if quantity_vec.is_empty() {
                                                    // Remove the key from bid_mbo if the quantity_vec is empty
                                                    quantvec_empty = true;
                                                   }  
                                                    
                                                }
                                            }
                                            if orderids_empty {
                                                ask_map.map.remove(&price);
                                                orderids_empty = false;
                                            }
                                            if quantvec_empty {
                                                ask_mbo.mbo.remove(&price);
                                                quantvec_empty = false;
                                            }
                    
                                        }
                                        let mut oldsum_zero = false;
                                        if let Some(old_sum) = ask_mbp.mbp.get_mut(&price) {
                                            *old_sum -= old_quantity;
                                            if *old_sum == 0 {
                                                // Remove the key from bid_mbp if the old_sum becomes 0
                                                oldsum_zero = true;
                                           }
                                        }
                                        if oldsum_zero {
                                            ask_mbp.mbp.remove(&price);
                                            oldsum_zero = false;
                                        }
                    
                                        let del_struct = delete_struct(&order_struct, unixtime);
                                        let order_message = Structs::DeletedOrderStruct(del_struct);
                                        if let Err(e) = tx_broker.send(order_message) {
                                            eprintln!("Failed to send message: {:?}", e);
                                        }
                                        let string_m = format! ("{} {}, {} {} at {} price level deleted, order id {} ", Utc::now().timestamp_micros(),order_struct.market,old_quantity,order_struct.order_side,order_struct.price,arrival.order_identifier );
                                        message_delete(&arrival,&tx_broker,string_m);
                                        mbp_event( Utc::now().timestamp_micros(), "ask".to_string(), old_quantity, price, -1, &tx_market,&config);
                                        
                                    }
                                    else if let Some(mut stop_structa) = stop_struct.get_mut(&arrival.order_identifier) {// stop
                                        let price = stop_structa.trigger_price;                
                                        isinstop = true;
                    
                                        let mut orderids_empty = false;
                        
                                        if let Some(order_ids) = stop_map.map.get_mut(&price) {
                                            if let Some(index) = order_ids.iter().position(|&id| id == arrival.order_identifier) {
                                                order_ids.remove(index);
                                                 if order_ids.is_empty() {
                                                    // Remove the key from bid_map if the order_ids vector is empty
                                                    orderids_empty = true;
                                                  }
                    
                                            }
                                            if orderids_empty {
                                                stop_map.map.remove(&price);
                                                orderids_empty = false;
                                            }
                    
                                        }
                                        
                                        let del_struct = delete_stop_struct(&stop_structa, unixtime);
                                        let order_message = Structs::DeletedStopOrderStruct(del_struct);
                                        if let Err(e) = tx_broker.send(order_message) {
                                            eprintln!("Failed to send message: {:?}", e);
                                        }
                                        let string_m = format! ("{} {}, {} {} at {} price level deleted, order id {} ", Utc::now().timestamp_micros(),stop_structa.market,stop_structa.order_quantity,stop_structa.order_side,stop_structa.trigger_price,arrival.order_identifier );
                                        message_delete(&arrival,&tx_broker,string_m);
                    
                                    }
                                    else if let Some(mut stop_limit_structa) = stop_limit_struct.get_mut(&arrival.order_identifier) {// stop limit
                                        let price = stop_limit_structa.trigger_price;                
                                        isinstoplimit = true;
                    
                                        let mut orderids_empty = false;
                        
                                        if let Some(order_ids) = stop_limit_map.map.get_mut(&price) {
                                            if let Some(index) = order_ids.iter().position(|&id| id == arrival.order_identifier) {
                                                order_ids.remove(index);
                                                 if order_ids.is_empty() {
                                                    // Remove the key from bid_map if the order_ids vector is empty
                                                    orderids_empty = true;
                                                  }
                    
                                            }
                                            if orderids_empty {
                                                stop_limit_map.map.remove(&price);
                                                orderids_empty = false;
                                            }
                    
                                        }
                                        
                                        let del_struct = delete_stop_limit_struct(&stop_limit_structa, unixtime);
                                        let order_message = Structs::DeletedStopLimitOrderStruct(del_struct);
                                        if let Err(e) = tx_broker.send(order_message) {
                                            eprintln!("Failed to send message: {:?}", e);
                                        }
                                        let string_m = format! ("{} {}, {} {} at {} price level deleted, order id {} ", Utc::now().timestamp_micros(),stop_limit_structa.market,stop_limit_structa.order_quantity,stop_limit_structa.order_side,stop_limit_structa.trigger_price,arrival.order_identifier );
                                        message_delete(&arrival,&tx_broker,string_m);
                                        
                                    }
                                        
                                    
                                    if isinbid {
                                        bid_struct.remove(&arrival.order_identifier);
                                        isinbid = false;
                                    }
                                    if isinask { 
                                        ask_struct.remove(&arrival.order_identifier);
                                        isinask = false;
                                    }
                                    if isinstop{
                                        stop_struct.remove(&arrival.order_identifier);
                                        isinstop = false;
                                    }
                                    if isinstoplimit{
                                        stop_limit_struct.remove(&arrival.order_identifier);
                                        isinstoplimit = false;
                                    }
                                    if !isinbid && !isinask && !isinstop && !isinstoplimit {
                                        let string_m = format!(
                                            "{} Order with id {} not found in any book for deletion.",
                                            Utc::now().timestamp_micros(),
                                            arrival.order_identifier
                                        );
                                        message_delete(&arrival,&tx_broker,string_m);
                                    }
                        }
                        Structs::IcebergOrder(arrival) => {
                            let iceberg_id = id_i64();
                            let ice_structi = IcebergOrderStruct {
                                market: arrival.market.clone(),
                                broker_identifier: arrival.broker_identifier.clone(),
                                unix_time: Utc::now().timestamp_micros(),
                                trader_identifier: arrival.trader_identifier,
                                iceberg_identifier:  iceberg_id,
                                total_quantity:arrival.total_quantity,
                                visible_quantity : arrival.visible_quantity,
                                resting_quantity: arrival.total_quantity - arrival.visible_quantity,
                                order_side: arrival.order_side.clone(),
                                expiration:arrival.expiration.clone(),
                                price: arrival.price,
                               
                            };
                            iceberg_struct.insert(iceberg_id, ice_structi.clone());
                    
                    
                            let order_message = Structs::IcebergOrderStruct(ice_structi);
                            if let Err(e) = tx_broker.send(order_message) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
                    
                            let string_m = format! ("{} {}, {} {} {} iceberg order at {} price level, id {} inserted.", Utc::now().timestamp_micros(),arrival.market,arrival.total_quantity,arrival.order_side,arrival.expiration,arrival.price,iceberg_id );
                            message_arrival_iceberg(&arrival,&tx_broker,string_m);
                    
                            let limit_order = LimitOrder {
                                market: arrival.market.clone(),
                                broker_identifier: arrival.broker_identifier.clone(),
                                trader_identifier: arrival.trader_identifier,
                                order_identifier: Some(iceberg_id),
                                order_quantity: arrival.visible_quantity,
                                order_side: arrival.order_side.clone(),
                                expiration: arrival.expiration.clone(),
                                price: arrival.price,
                                pointing_at:None,
                                
                            };
                            let order_message = Structs::LimitOrder(limit_order);
                             
                            if let Err(e) = tx2.send(order_message) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
                    
                            let string_m = format! ("{} {}, {} {} at {} price level {}, visible iceberg limit order id {} entered to market", Utc::now().timestamp_micros(),arrival.market,arrival.visible_quantity,arrival.order_side,arrival.price,arrival.expiration,iceberg_id );
                            let message = Messaging {
                                unix_time :  Utc::now().timestamp_micros(),
                                market : arrival.market.clone(),
                                broker_identifier : arrival.broker_identifier.clone(),
                                trader_identifier : arrival.trader_identifier,
                                message : string_m,
                            };
                            let m_message = Structs::Messaging(message);
                            if let Err(e) = tx_broker.send(m_message) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
                    
                            let ex_iceberg = ExecutedIceberg {
                                unix_time :  Utc::now().timestamp_micros(),
                                market : arrival.market.clone(),
                                broker_identifier : arrival.broker_identifier.clone(),
                                trader_identifier :arrival.trader_identifier,
                                iceberg_identifier : iceberg_id,
                                executed_quantity : arrival.visible_quantity,
                                order_side : arrival.order_side.clone(),
                                expiration : arrival.expiration.clone(),
                                price : arrival.price,
                            };
                            let ex_message = Structs::ExecutedIceberg(ex_iceberg);
                            if let Err(e) = tx_broker.send(ex_message) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
                        }
                        Structs::ModifyIcebergOrder(arrival) => {
                            if let Some(mut ice_struct) = iceberg_struct.get_mut(&arrival.iceberg_identifier) { //bid
                           
                              
                              let modf_struct = modify_iceberg_struct (&config,&ice_struct,Utc::now().timestamp_micros(),arrival.new_quantity,arrival.new_visible_quantity,arrival.new_quantity);
                              let order_message = Structs::ModifiedIcebergStruct(modf_struct);
                              if let Err(e) = tx_broker.send(order_message) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
                               let string_m = format! ("{} {}, {} {} iceberg at {} price level modified to {} quantity, order id {} ", Utc::now().timestamp_micros(),ice_struct.market,ice_struct.total_quantity,ice_struct.order_side,ice_struct.price,arrival.new_quantity,arrival.iceberg_identifier );
                               message_modify_iceberg(&arrival,&tx_broker,string_m);
                    
                               ice_struct.total_quantity = arrival.new_quantity;
                               ice_struct.visible_quantity = arrival.new_visible_quantity;
                               ice_struct.resting_quantity = arrival.new_quantity;
                              
                            } else {
                                let string_m = format! ("No Iceberg found for id {} ",arrival.iceberg_identifier );
                               message_modify_iceberg(&arrival,&tx_broker,string_m);
                            }
                    
                        }
                        Structs::DeleteIcebergOrder(arrival) => {
                            let mut exist = false; 
                            if let Some( ice_struct) = iceberg_struct.get_mut(&arrival.iceberg_identifier) {
                                exist = true;
                                let string_m = format! ("Iceberg order id {} deleted. ",arrival.iceberg_identifier );
                                message_delete_iceberg(&arrival,&tx_broker,string_m);
                                let del_struct = delete_iceberg_struct (&ice_struct,Utc::now().timestamp_micros());
                              let order_message = Structs::DeletedIcebergStruct(del_struct);
                              if let Err(e) = tx_broker.send(order_message) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
                            } else {
                                let string_m = format! ("No Iceberg found for id {} ",arrival.iceberg_identifier );
                                message_delete_iceberg(&arrival,&tx_broker,string_m);
                            }
                            if exist {
                                iceberg_struct.remove(&arrival.iceberg_identifier);
                               
                            }
                        }
                        Structs::Save(_) => {
                            
                            if let Err(e) = tx_position.send(message_arrive_save) {
                                eprintln!("Failed to send message: {:?}", e);
                            }
    
                            if let Err(e) = save_to_json_file(&bid_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "bid_struct.json") {
                                eprintln!("Failed to save bid_struct: {}", e);
                            }
                            if let Err(e) = save_to_json_file(&bid_map, "bid_map.json") {
                                eprintln!("Failed to save bid_map: {}", e);
                            }
                            if let Err(e) = save_to_json_file(&bid_mbo, "bid_mbo.json") {
                                eprintln!("Failed to save bid_mbo: {}", e);
                            }
                            if let Err(e) = save_to_json_file(&bid_mbp, "bid_mbp.json") {
                                eprintln!("Failed to save bid_mbp: {}", e);
                            }
                        
                            // Save ask data
                            if let Err(e) = save_to_json_file(&ask_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "ask_struct.json") {
                                eprintln!("Failed to save ask_struct: {}", e);
                            }
                            if let Err(e) = save_to_json_file(&ask_map, "ask_map.json") {
                                eprintln!("Failed to save ask_map: {}", e);
                            }
                            if let Err(e) = save_to_json_file(&ask_mbo, "ask_mbo.json") {
                                eprintln!("Failed to save ask_mbo: {}", e);
                            }
                            if let Err(e) = save_to_json_file(&ask_mbp, "ask_mbp.json") {
                                eprintln!("Failed to save ask_mbp: {}", e);
                            }
                        
                            // Save stop data
                            if let Err(e) = save_to_json_file(&stop_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "stop_struct.json") {
                                eprintln!("Failed to save stop_struct: {}", e);
                            }
                            if let Err(e) = save_to_json_file(&stop_map, "stop_map.json") {
                                eprintln!("Failed to save stop_map: {}", e);
                            }
                        
                            // Save stop limit data
                            if let Err(e) = save_to_json_file(&stop_limit_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "stop_limit_struct.json") {
                                eprintln!("Failed to save stop_limit_struct: {}", e);
                            }
                            if let Err(e) = save_to_json_file(&stop_limit_map, "stop_limit_map.json") {
                                eprintln!("Failed to save stop_limit_map: {}", e);
                            }
                          
                            // Save iceberg data
                            if let Err(e) = save_to_json_file(&iceberg_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "iceberg_struct.json") {
                                eprintln!("Failed to save iceberg_struct: {}", e);
                            }
    
                            println!("Position map and tree manually saved");
    
                        }
                    
                        _ => {}    
                        
                    }
                    let dyn_bbo = struct_bbo(Utc::now().timestamp_micros(), &ask_mbp, &bid_mbp,&config);
                    let bbo_http_clone = Arc::clone(&bbo_http);
                    match bbo_http_clone.lock() {
                        Ok(mut bbo_http_mut) => {
                            *bbo_http_mut = dyn_bbo.clone(); // Update the data
                        }
                        Err(e) => {
                            eprintln!("Failed to lock the mutex for bbo_http: {:?}", e);
                            return; // Or handle the error in a way that makes sense for your application
                        }
                    }
                    let bbo_message = Structs::BBO(dyn_bbo);
                    if let Err(e) = tx_market.send(bbo_message) {
                        eprintln!("Failed to send FullOB message: {:?}", e);
                    }
                    
            
                let fullob = full_ob(Utc::now().timestamp_micros(), &ask_mbp, &bid_mbp, &config);
                let fullob_m = Structs::FullOB(fullob);
            
                // Send the full order book message
                if let Err(e) = tx_market.send(fullob_m) {
                    eprintln!("Failed to send FullOB message: {:?}", e);
                }
                },
                Err(e) => {
                    //println!("Error receiving message: {}", e);
                    continue;
                }
            }
    
    
    if last_save_time.elapsed().as_secs() >= 800 {
        // Save bid data
        if let Err(e) = save_to_json_file(&bid_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "bid_struct.json") {
            eprintln!("Failed to save bid_struct: {}", e);
        }
        if let Err(e) = save_to_json_file(&bid_map, "bid_map.json") {
            eprintln!("Failed to save bid_map: {}", e);
        }
        if let Err(e) = save_to_json_file(&bid_mbo, "bid_mbo.json") {
            eprintln!("Failed to save bid_mbo: {}", e);
        }
        if let Err(e) = save_to_json_file(&bid_mbp, "bid_mbp.json") {
            eprintln!("Failed to save bid_mbp: {}", e);
        }
    
        // Save ask data
        if let Err(e) = save_to_json_file(&ask_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "ask_struct.json") {
            eprintln!("Failed to save ask_struct: {}", e);
        }
        if let Err(e) = save_to_json_file(&ask_map, "ask_map.json") {
            eprintln!("Failed to save ask_map: {}", e);
        }
        if let Err(e) = save_to_json_file(&ask_mbo, "ask_mbo.json") {
            eprintln!("Failed to save ask_mbo: {}", e);
        }
        if let Err(e) = save_to_json_file(&ask_mbp, "ask_mbp.json") {
            eprintln!("Failed to save ask_mbp: {}", e);
        }
    
        // Save stop data
        if let Err(e) = save_to_json_file(&stop_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "stop_struct.json") {
            eprintln!("Failed to save stop_struct: {}", e);
        }
        if let Err(e) = save_to_json_file(&stop_map, "stop_map.json") {
            eprintln!("Failed to save stop_map: {}", e);
        }
    
        // Save stop limit data
        if let Err(e) = save_to_json_file(&stop_limit_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "stop_limit_struct.json") {
            eprintln!("Failed to save stop_limit_struct: {}", e);
        }
        if let Err(e) = save_to_json_file(&stop_limit_map, "stop_limit_map.json") {
            eprintln!("Failed to save stop_limit_map: {}", e);
        }
      
        // Save iceberg data
        if let Err(e) = save_to_json_file(&iceberg_struct.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect::<Vec<_>>(), "iceberg_struct.json") {
            eprintln!("Failed to save iceberg_struct: {}", e);
        }
        println!("Regular map and tree periodically saved");
        // Reset the save time counter
        last_save_time = Instant::now();
    }
                
        }
        });

        
       
    HttpServer::new(move || {
        let db2_market = Arc::clone(&db1_market);
        let db2_broker = Arc::clone(&db1_broker);
        let ws_connections = Arc::clone(&ws_connections);
        let connection_type_map = Arc::clone(&connection_type_map);
        let coll_config = Arc::clone(&coll_config);
        let market_name = &config_clone_http.market_name;
        let bbo_http = Arc::clone(&bbo_http_clone);
        let cors = Cors::default()
            
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);
        //let db1_market = Arc::clone(&db1_market);
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(tx_order.clone()))
            .app_data(web::Data::new(broker_config.clone()))
            .app_data(web::Data::new(market_config.clone()))
            .app_data(web::Data::new(config_clone_http.clone()))
            .app_data(web::Data::new(market_spec_config.clone()))
            .app_data(web::Data::new(db2_market)) 
            .app_data(web::Data::new(db2_broker)) 
            .app_data(web::Data::new(ws_connections))
            .app_data(web::Data::new(connection_type_map))
            .app_data(web::Data::new(coll_config))
            .app_data(web::Data::new(bbo_http))
            .route(&format!("/order/limit_order/{market_name}"), web::post().to(limit_order))
            .route(&format!("/order/iceberg_order/{market_name}"), web::post().to(iceberg_order))
            .route(&format!("/order/market_order/{market_name}"), web::post().to(market_order))
            .route(&format!("/order/stop_order/{market_name}"), web::post().to(stop_order))
            .route(&format!("/order/stoplimit_order/{market_name}"), web::post().to(stoplimit_order))
            .route(&format!("/order/modify_order/{market_name}"), web::post().to(modify_order))
            .route(&format!("/order/modify_iceberg_order/{market_name}"), web::post().to(modify_iceberg_order))
            .route(&format!("/order/delete_order/{market_name}"), web::post().to(delete_order))
            .route(&format!("/order/delete_iceberg_order/{market_name}"), web::post().to(delete_iceberg_order))
            .route(&format!("/order/save/{market_name}"), web::post().to(save))
            .route(&format!("/history_last/{market_name}"), web::post().to(history_last))
            .route(&format!("/history_bbo/{market_name}"), web::post().to(history_bbo))
            .route(&format!("/history_tns/{market_name}"), web::post().to(history_tns))
            .route(&format!("/history_mbpevent/{market_name}"), web::post().to(history_mbpevent))
            .route(&format!("/history_volume/{market_name}"), web::post().to(history_volume))
            .route(&format!("/full_ob/{market_name}"), web::get().to(full_ob_extractor))
            .route(&format!("/full_interest/{market_name}"), web::get().to(full_interest_extractor))
            .route(&format!("/history_interestevent/{market_name}"), web::post().to(history_interestevent))
            // WebSocket routes
            .route(&format!("/ws/last_rt/{market_name}"), web::get().to(last_handler))
            .route(&format!("/ws/mbp_event_rt/{market_name}"), web::get().to(mbp_event_handler))
            .route(&format!("/ws/best_bid_offer_rt/{market_name}"), web::get().to(bbo_handler))
            .route(&format!("/ws/volume_rt/{market_name}"), web::get().to(volume_handler))
            .route(&format!("/ws/time_sale_rt/{market_name}"), web::get().to(tns_handler))
            .route(&format!("/ws/interest_event_rt/{market_name}"), web::get().to(interest_event_handler))
    })
    .bind(server_url)?
    .run()
    .await
}
