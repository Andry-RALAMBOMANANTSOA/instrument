use actix_web::{rt::time, web,HttpRequest, HttpResponse, Responder};
use serde_json::json;
use mongodb::{Database,bson::doc,bson,Collection};
use tokio::sync::mpsc;
use shared_structs::*;
use crate::env_coll_decl::CollConfig;
use crate::function::*;
use std::sync::mpsc as sync_mpsc;
use chrono::{DateTime, Utc, SecondsFormat,Duration,NaiveDateTime,Local,TimeZone};
use std::sync::{Arc,Mutex};
use std::{default, env};
use crate::dedic_structs::*;

pub async fn limit_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match serde_json::from_slice::<LimitOrder>(&body_vec) {
                    Ok(limit_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&limit_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if limit_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            // Retrieve market specifications for the limit order's market
                    
                           
                            if let Err(err) = validate_limit_order(&limit_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            if let Some(market_spec) = market_spec_config.market_specification.get(&limit_order.market) {
                                // Validate the order against market specs
                                if let Err(err) = validate_limit_specs(&limit_order, market_spec) {
                                    return HttpResponse::BadRequest().body(err);
                                }
                            } else {
                                return HttpResponse::BadRequest().body("Market specification not found");
                            }
                            if let Some(pointing_at) = limit_order.pointing_at {
                                let a_position: Result<PositionStruct, String> = fetch_document_position(&db, &coll_config.coll_a_position, pointing_at).await;
                                let a_position_s = match a_position {
                                Ok(user) => user, // Extract the user if the result is Ok
                                Err(err) => {
                                    return HttpResponse::InternalServerError().body(format!("Position not found: {}",err));
                                    }
                                };

                                if limit_order.trader_identifier != a_position_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The position is not owned by the user.");
                                }

                                if limit_order.order_side == a_position_s.position_side {
                                    return HttpResponse::BadRequest().body(format!("The order is not a closing position for the position because they have the same position side:{}.",a_position_s.position_side));
                                }
                                if limit_order.order_quantity > a_position_s.position_quantity {
                                    return HttpResponse::BadRequest().body(format!("The closing order quantity {} exceeds the position quantity {}.",limit_order.order_quantity,a_position_s.position_quantity));
                                }
                            } else {//No pointing at
                                let trader_documents: Result<Vec<PositionStruct>, String> = fetch_all_documents_by_trader_id(&db, &coll_config.coll_a_position, limit_order.trader_identifier).await;

                                match trader_documents {
                                    Ok(docs) => {
                                        // Check if the vector is not empty and take the first position side
                                        if let Some(first_position) = docs.get(0) {
                                            if limit_order.order_side == first_position.position_side {
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, limit_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (limit_order.order_quantity as f32
                                                    * market_conf.contract as f32
                                                    * limit_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                            } else if limit_order.order_side != first_position.position_side{ //limit_order.order_side != first_position.position_side

                                                // Calculate the total quantity of all positions
                                            let total_position_quantity: i32 = docs.iter().map(|pos| pos.position_quantity).sum();
                                
                                            // Compare the total quantity with the order quantity
                                            if limit_order.order_quantity > total_position_quantity {
                                                let limitable = limit_order.order_quantity - total_position_quantity;
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, limit_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (limitable as f32
                                                    * market_conf.contract as f32
                                                    * limit_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement for positionable quantity {} is {}",limitable,margin_requirement));
                                                }
                                            }

                                            }
                                        } else { //Mbola tsy misy position
                                           let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, limit_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (limit_order.order_quantity as f32
                                                    * market_conf.contract as f32
                                                    * limit_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                        }
                            
                                        
                                    }
                                    Err(err) => {
                                        return HttpResponse::InternalServerError().body(format!("Error fetching positions: {}", err));
                                    }
                                }       
                            }
             
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {

                                    if let Err(_) = tx.send(Structs::LimitOrder(limit_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

pub async fn market_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,coll_config: web::Data<Arc<CollConfig>>,bbo:web::Data<Arc<Mutex<BBO>>>) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<MarketOrder>(&body_vec) {
                    Ok(market_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&market_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if market_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            if let Err(err) = validate_market_order(&market_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            if let Some(market_spec) = market_spec_config.market_specification.get(&market_order.market) {
                                // Validate the order against market specs
                                if let Err(err) = validate_market_specs(&market_order, market_spec) {
                                    return HttpResponse::BadRequest().body(err);
                                }
                            } else {
                                return HttpResponse::BadRequest().body("Market specification not found");
                            }
                            if let Some(pointing_at) = market_order.pointing_at {
                                let a_position: Result<PositionStruct, String> = fetch_document_position(&db, &coll_config.coll_a_position, pointing_at).await;
                                let a_position_s = match a_position {
                                Ok(user) => user, // Extract the user if the result is Ok
                                Err(err) => {
                                    return HttpResponse::InternalServerError().body(format!("Position not found: {}",err));
                                    }
                                };

                                if market_order.trader_identifier != a_position_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The position is not owned by the user.");
                                }

                                if market_order.order_side == a_position_s.position_side {
                                    return HttpResponse::BadRequest().body(format!("The order is not a closing position for the position because they have the same position side:{}.",a_position_s.position_side));
                                }
                                if market_order.order_quantity > a_position_s.position_quantity {
                                    return HttpResponse::BadRequest().body(format!("The closing order quantity {} exceeds the position quantity {}.",market_order.order_quantity,a_position_s.position_quantity));
                                }
                            } else {//No pointing at
                                let trader_documents: Result<Vec<PositionStruct>, String> = fetch_all_documents_by_trader_id(&db, &coll_config.coll_a_position, market_order.trader_identifier).await;

                                match trader_documents {
                                    Ok(docs) => {
                                        // Check if the vector is not empty and take the first position side
                                        if let Some(first_position) = docs.get(0) {
                                            if market_order.order_side == first_position.position_side {
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, market_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let calc_price: Option<i32>;
                                                match bbo.lock() {
                                                    Ok(bbo_data) => {
                                                        // Successfully locked the Mutex, now access the fields of BBO
                                                        calc_price = match market_order.order_side {
                                                            OrderSide::Long => bbo_data.ask_price,
                                                            OrderSide::Short => bbo_data.bid_price,
                                                            _ => None, // Handle unexpected order side by setting calc_price to None
                                                        };
                                                    }
                                                    Err(_) => {
                                                        // Handle the error if the lock fails
                                                        return HttpResponse::InternalServerError().body("Failed to lock BBO data");
                                                    }
                                                }
                                                if let Some(price) = calc_price {
                                                    let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                    let margin_requirement = (market_order.order_quantity as f32
                                                        * market_conf.contract as f32
                                                        * price as f32
                                                        * margin_multiplier)
                                                        .round(); // Round the result to the nearest integer

                                                        if balance_s.balance <= margin_requirement as i32 {
                                                            return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                        }
                                                } else {
                                                    // Handle the case where the price is None (i.e., no valid ask_price or bid_price found)
                                                    return HttpResponse::BadRequest().body("Invalid price: ask or bid price is not available.");
                                                }
            
                                                
                                            } else if market_order.order_side != first_position.position_side{ //market_order.order_side != first_position.position_side

                                                // Calculate the total quantity of all positions
                                            let total_position_quantity: i32 = docs.iter().map(|pos| pos.position_quantity).sum();
                                
                                            // Compare the total quantity with the order quantity
                                            if market_order.order_quantity > total_position_quantity {
                                                let limitable = market_order.order_quantity - total_position_quantity;
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, market_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let calc_price: Option<i32>;
                                                match bbo.lock() {
                                                    Ok(bbo_data) => {
                                                        // Successfully locked the Mutex, now access the fields of BBO
                                                        calc_price = match market_order.order_side {
                                                            OrderSide::Long => bbo_data.ask_price,
                                                            OrderSide::Short => bbo_data.bid_price,
                                                            _ => None, // Handle unexpected order side by setting calc_price to None
                                                        };
                                                    }
                                                    Err(_) => {
                                                        // Handle the error if the lock fails
                                                        return HttpResponse::InternalServerError().body("Failed to lock BBO data");
                                                    }
                                                }
                                                if let Some(price) = calc_price {
                                                    let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                    let margin_requirement = (limitable as f32
                                                        * market_conf.contract as f32
                                                        * price as f32
                                                        * margin_multiplier)
                                                        .round(); // Round the result to the nearest integer

                                                        if balance_s.balance <= margin_requirement as i32 {
                                                            return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement for positionable quantity {} is {}",limitable,margin_requirement));
                                                        }
                                                } else {
                                                    // Handle the case where the price is None (i.e., no valid ask_price or bid_price found)
                                                    return HttpResponse::BadRequest().body("Invalid price: ask or bid price is not available.");
                                                }
                                            }

                                            }
                                        } else { //Mbola tsy misy position
                                           let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, market_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let calc_price: Option<i32>;
                                                match bbo.lock() {
                                                    Ok(bbo_data) => {
                                                        // Successfully locked the Mutex, now access the fields of BBO
                                                        calc_price = match market_order.order_side {
                                                            OrderSide::Long => bbo_data.ask_price,
                                                            OrderSide::Short => bbo_data.bid_price,
                                                            _ => None, // Handle unexpected order side by setting calc_price to None
                                                        };
                                                    }
                                                    Err(_) => {
                                                        // Handle the error if the lock fails
                                                        return HttpResponse::InternalServerError().body("Failed to lock BBO data");
                                                    }
                                                }
                                                if let Some(price) = calc_price {
                                                    let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                    let margin_requirement = (market_order.order_quantity as f32
                                                        * market_conf.contract as f32
                                                        * price as f32
                                                        * margin_multiplier)
                                                        .round(); // Round the result to the nearest integer

                                                        if balance_s.balance <= margin_requirement as i32 {
                                                            return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                        }
                                                } else {
                                                    // Handle the case where the price is None (i.e., no valid ask_price or bid_price found)
                                                    return HttpResponse::BadRequest().body("Invalid price: ask or bid price is not available.");
                                                }
                                        }
  
                                    }
                                    Err(err) => {
                                        return HttpResponse::InternalServerError().body(format!("Error fetching positions: {}", err));
                                    }
                                }       
                            }
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {
                                    if let Err(_) = tx.send(Structs::MarketOrder(market_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

pub async fn stop_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes,tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<StopOrder>(&body_vec) {
                    Ok(stop_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&stop_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if stop_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            if let Err(err) = validate_stop_order(&stop_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            if let Some(market_spec) = market_spec_config.market_specification.get(&stop_order.market) {
                                // Validate the order against market specs
                                if let Err(err) = validate_stop_specs(&stop_order, market_spec) {
                                    return HttpResponse::BadRequest().body(err);
                                }
                            } else {
                                return HttpResponse::BadRequest().body("Market specification not found");
                            }
                            if let Some(pointing_at) = stop_order.pointing_at {
                                let a_position: Result<PositionStruct, String> = fetch_document_position(&db, &coll_config.coll_a_position, pointing_at).await;
                                let a_position_s = match a_position {
                                Ok(user) => user, // Extract the user if the result is Ok
                                Err(err) => {
                                    return HttpResponse::InternalServerError().body(format!("Position not found: {}",err));
                                    }
                                };

                                if stop_order.trader_identifier != a_position_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The position is not owned by the user.");
                                }

                                if stop_order.order_side == a_position_s.position_side {
                                    return HttpResponse::BadRequest().body(format!("The order is not a closing position for the position because they have the same position side:{}.",a_position_s.position_side));
                                }
                                if stop_order.order_quantity > a_position_s.position_quantity {
                                    return HttpResponse::BadRequest().body(format!("The closing order quantity {} exceeds the position quantity {}.",stop_order.order_quantity,a_position_s.position_quantity));
                                }
                            } else {//No pointing at
                                let trader_documents: Result<Vec<PositionStruct>, String> = fetch_all_documents_by_trader_id(&db, &coll_config.coll_a_position, stop_order.trader_identifier).await;

                                match trader_documents {
                                    Ok(docs) => {
                                        // Check if the vector is not empty and take the first position side
                                        if let Some(first_position) = docs.get(0) {
                                            if stop_order.order_side == first_position.position_side {
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, stop_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (stop_order.order_quantity as f32
                                                    * market_conf.contract as f32
                                                    * stop_order.trigger_price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                            } else if stop_order.order_side != first_position.position_side{ //stop_order.order_side != first_position.position_side

                                                // Calculate the total quantity of all positions
                                            let total_position_quantity: i32 = docs.iter().map(|pos| pos.position_quantity).sum();
                                
                                            // Compare the total quantity with the order quantity
                                            if stop_order.order_quantity > total_position_quantity {
                                                let limitable = stop_order.order_quantity - total_position_quantity;
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, stop_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (limitable as f32
                                                    * market_conf.contract as f32
                                                    * stop_order.trigger_price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement for positionable quantity {} is {}",limitable,margin_requirement));
                                                }
                                            }

                                            }
                                        } else { //Mbola tsy misy position
                                           let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, stop_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (stop_order.order_quantity as f32
                                                    * market_conf.contract as f32
                                                    * stop_order.trigger_price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                        }
                            
                                        
                                    }
                                    Err(err) => {
                                        return HttpResponse::InternalServerError().body(format!("Error fetching positions: {}", err));
                                    }
                                }       
                            }
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {
                                    if let Err(_) = tx.send(Structs::StopOrder(stop_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

pub async fn stoplimit_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<StopLimitOrder>(&body_vec) {
                    Ok(stoplimit_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&stoplimit_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if stoplimit_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            if let Err(err) = validate_stoplimit_order(&stoplimit_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            if let Some(market_spec) = market_spec_config.market_specification.get(&stoplimit_order.market) {
                                // Validate the order against market specs
                                if let Err(err) = validate_stoplimit_specs(&stoplimit_order, market_spec) {
                                    return HttpResponse::BadRequest().body(err);
                                }
                            } else {
                                return HttpResponse::BadRequest().body("Market specification not found");
                            }
                            if let Some(pointing_at) = stoplimit_order.pointing_at {
                                let a_position: Result<PositionStruct, String> = fetch_document_position(&db, &coll_config.coll_a_position, pointing_at).await;
                                let a_position_s = match a_position {
                                Ok(user) => user, // Extract the user if the result is Ok
                                Err(err) => {
                                    return HttpResponse::InternalServerError().body(format!("Position not found: {}",err));
                                    }
                                };

                                if stoplimit_order.trader_identifier != a_position_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The position is not owned by the user.");
                                }

                                if stoplimit_order.order_side == a_position_s.position_side {
                                    return HttpResponse::BadRequest().body(format!("The order is not a closing position for the position because they have the same position side:{}.",a_position_s.position_side));
                                }
                                if stoplimit_order.order_quantity > a_position_s.position_quantity {
                                    return HttpResponse::BadRequest().body(format!("The closing order quantity {} exceeds the position quantity {}.",stoplimit_order.order_quantity,a_position_s.position_quantity));
                                }
                            } else {//No pointing at
                                let trader_documents: Result<Vec<PositionStruct>, String> = fetch_all_documents_by_trader_id(&db, &coll_config.coll_a_position, stoplimit_order.trader_identifier).await;

                                match trader_documents {
                                    Ok(docs) => {
                                        // Check if the vector is not empty and take the first position side
                                        if let Some(first_position) = docs.get(0) {
                                            if stoplimit_order.order_side == first_position.position_side {
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, stoplimit_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (stoplimit_order.order_quantity as f32
                                                    * market_conf.contract as f32
                                                    * stoplimit_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                            } else if stoplimit_order.order_side != first_position.position_side { //stoplimit_order.order_side != first_position.position_side

                                                // Calculate the total quantity of all positions
                                            let total_position_quantity: i32 = docs.iter().map(|pos| pos.position_quantity).sum();
                                
                                            // Compare the total quantity with the order quantity
                                            if stoplimit_order.order_quantity > total_position_quantity {
                                                let limitable = stoplimit_order.order_quantity - total_position_quantity;
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, stoplimit_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (limitable as f32
                                                    * market_conf.contract as f32
                                                    * stoplimit_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement for positionable quantity {} is {}",limitable,margin_requirement));
                                                }
                                            }

                                            }
                                        } else { //Mbola tsy misy position
                                           let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, stoplimit_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (stoplimit_order.order_quantity as f32
                                                    * market_conf.contract as f32
                                                    * stoplimit_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                        }
                            
                                        
                                    }
                                    Err(err) => {
                                        return HttpResponse::InternalServerError().body(format!("Error fetching positions: {}", err));
                                    }
                                }       
                            }
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {
                                    if let Err(_) = tx.send(Structs::StopLimitOrder(stoplimit_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

pub async fn modify_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<ModifyOrder>(&body_vec) {
                    Ok(modify_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&modify_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if modify_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            if let Err(err) = validate_modify_order(&modify_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            if let Some(market_spec) = market_spec_config.market_specification.get(&modify_order.market) {
                                // Validate the order against market specs
                                if let Err(err) = validate_modify_specs(&modify_order, market_spec) {
                                    return HttpResponse::BadRequest().body(err);
                                }
                            } else {
                                return HttpResponse::BadRequest().body("Market specification not found");
                            }
                            let a_order: Result<TraderOrderStruct, String> = fetch_document_byorderid(&db, &coll_config.coll_p_order, modify_order.order_identifier).await;

                            if let Ok(a_order_s) = a_order {
                                if modify_order.trader_identifier != a_order_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The order is not owned by the user.");
                                }
                                if modify_order.new_quantity > a_order_s.order_quantity {
                                    let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, modify_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (modify_order.new_quantity as f32
                                                    * market_conf.contract as f32
                                                    * a_order_s.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                }
                               
                            } else { //Tsisy ao @ limit order

                                let a_sorder: Result<TraderStopOrderStruct, String> = fetch_document_byorderid(&db, &coll_config.coll_p_sorder, modify_order.order_identifier).await;

                            if let Ok(a_sorder_s) = a_sorder {
                                if modify_order.trader_identifier != a_sorder_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The order is not owned by the user.");
                                }
                                if modify_order.new_quantity > a_sorder_s.order_quantity {
                                    let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, modify_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (modify_order.new_quantity as f32
                                                    * market_conf.contract as f32
                                                    * a_sorder_s.trigger_price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                }
                               
                            } else {//Tsisy ao @ stop order

                                let a_slorder: Result<TraderStopLimitOrderStruct, String> = fetch_document_byorderid(&db, &coll_config.coll_p_slorder, modify_order.order_identifier).await;

                            if let Ok(a_slorder_s) = a_slorder {
                                if modify_order.trader_identifier != a_slorder_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The order is not owned by the user.");
                                }
                                if modify_order.new_quantity > a_slorder_s.order_quantity {
                                    let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, modify_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (modify_order.new_quantity as f32
                                                    * market_conf.contract as f32
                                                    * a_slorder_s.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                }
                               
                            } else { //Tsisy ao @ stop limit order
                                return HttpResponse::InternalServerError().body("THe order is not found anywhere.");
                            }
                               
                            }
                               
                            }
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {
                                    if let Err(_) = tx.send(Structs::ModifyOrder(modify_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

pub async fn delete_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<DeleteOrder>(&body_vec) {
                    Ok(delete_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&delete_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if delete_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            if let Err(err) = validate_delete_order(&delete_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            let a_order: Result<TraderOrderStruct, String> = fetch_document_byorderid(&db, &coll_config.coll_p_order, delete_order.order_identifier).await;

                            if let Ok(a_order_s) = a_order {
                                if delete_order.trader_identifier != a_order_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The order is not owned by the user.");
                                }
                               
                            } else { //Tsisy ao @ limit order

                                let a_sorder: Result<TraderStopOrderStruct, String> = fetch_document_byorderid(&db, &coll_config.coll_p_sorder, delete_order.order_identifier).await;

                            if let Ok(a_sorder_s) = a_sorder {
                                if delete_order.trader_identifier != a_sorder_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The order is not owned by the user.");
                                }
                               
                            } else {//Tsisy ao @ stop order

                                let a_slorder: Result<TraderStopLimitOrderStruct, String> = fetch_document_byorderid(&db, &coll_config.coll_p_slorder, delete_order.order_identifier).await;

                            if let Ok(a_slorder_s) = a_slorder {
                                if delete_order.trader_identifier != a_slorder_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The order is not owned by the user.");
                                }
                               
                            } else { //Tsisy ao @ stop limit order
                                return HttpResponse::InternalServerError().body("THe order is not found anywhere.");
                            }
                               
                            }
                               
                            }
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {
                                    if let Err(_) = tx.send(Structs::DeleteOrder(delete_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

pub async fn iceberg_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<IcebergOrder>(&body_vec) {
                    Ok(iceberg_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&iceberg_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if iceberg_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            // Retrieve market specifications for the limit order's market
                           
                            if let Err(err) = validate_iceberg_order(&iceberg_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            if let Some(market_spec) = market_spec_config.market_specification.get(&iceberg_order.market) {
                                // Validate the order against market specs
                                if let Err(err) = validate_iceberg_specs(&iceberg_order, market_spec) {
                                    return HttpResponse::BadRequest().body(err);
                                }
                            } else {
                                return HttpResponse::BadRequest().body("Market specification not found");
                            }
                           
                                let trader_documents: Result<Vec<PositionStruct>, String> = fetch_all_documents_by_trader_id(&db, &coll_config.coll_a_position, iceberg_order.trader_identifier).await;

                                match trader_documents {
                                    Ok(docs) => {
                                        // Check if the vector is not empty and take the first position side
                                        if let Some(first_position) = docs.get(0) {
                                            if iceberg_order.order_side == first_position.position_side {
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, iceberg_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (iceberg_order.total_quantity as f32
                                                    * market_conf.contract as f32
                                                    * iceberg_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                            } else if iceberg_order.order_side != first_position.position_side { //iceberg_order.order_side != first_position.position_side

                                                // Calculate the total quantity of all positions
                                            let total_position_quantity: i32 = docs.iter().map(|pos| pos.position_quantity).sum();
                                
                                            // Compare the total quantity with the order quantity
                                            if iceberg_order.total_quantity > total_position_quantity {
                                                let limitable = iceberg_order.total_quantity - total_position_quantity;
                                                let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, iceberg_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (limitable as f32
                                                    * market_conf.contract as f32
                                                    * iceberg_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement for positionable quantity {} is {}",limitable,margin_requirement));
                                                }
                                            }

                                            }
                                        } else { //Mbola tsy misy position
                                           let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, iceberg_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (iceberg_order.total_quantity as f32
                                                    * market_conf.contract as f32
                                                    * iceberg_order.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                        }
                            
                                        
                                    }
                                    Err(err) => {
                                        return HttpResponse::InternalServerError().body(format!("Error fetching positions: {}", err));
                                    }
                                }       
                            
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {
                                    if let Err(_) = tx.send(Structs::IcebergOrder(iceberg_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

pub async fn modify_iceberg_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<ModifyIcebergOrder>(&body_vec) {
                    Ok(modify_iceberg_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&modify_iceberg_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if modify_iceberg_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            if let Err(err) = validate_modify_iceberg_order(&modify_iceberg_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            if let Some(market_spec) = market_spec_config.market_specification.get(&modify_iceberg_order.market) {
                                // Validate the order against market specs
                                if let Err(err) = validate_modify_iceberg_specs(&modify_iceberg_order, market_spec) {
                                    return HttpResponse::BadRequest().body(err);
                                }
                            } else {
                                return HttpResponse::BadRequest().body("Market specification not found");
                            }
                            let a_order: Result<IcebergOrderStruct, String> = fetch_document_byorderid(&db, &coll_config.coll_p_iceberg, modify_iceberg_order.iceberg_identifier).await;

                            if let Ok(a_order_s) = a_order {
                                if modify_iceberg_order.trader_identifier != a_order_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The order is not owned by the user.");
                                }
                                if modify_iceberg_order.new_quantity > a_order_s.resting_quantity {
                                    let balance: Result<TraderBalance, String> = fetch_document_traderid(&db, &coll_config.coll_trdr_bal, modify_iceberg_order.trader_identifier).await;
                                                let balance_s = match balance {
                                                Ok(user) => user, // Extract the user if the result is Ok
                                                Err(_) => {
                                                    return HttpResponse::InternalServerError().body("Error during balance fetching");
                                                    }
                                                };
                                                let margin_multiplier = market_conf.margin_perc as f32 / 100.0;
                                                let margin_requirement = (modify_iceberg_order.new_quantity as f32
                                                    * market_conf.contract as f32
                                                    * a_order_s.price as f32
                                                    * margin_multiplier)
                                                    .round(); // Round the result to the nearest integer
            
                                                if balance_s.balance <= margin_requirement as i32 {
                                                    return HttpResponse::BadRequest().body(format!("Not sufficient margin as margin requirement is {}",margin_requirement));
                                                }
                                }
                               
                            } else {
                                return HttpResponse::InternalServerError().body("The iceberg order is not found.");
                            }
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {
                                    if let Err(_) = tx.send(Structs::ModifyIcebergOrder(modify_iceberg_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

pub async fn delete_iceberg_order(broker_db: web::Data<BrokerDb>,req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder {
        let db: &Database = &broker_db.db;
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<DeleteIcebergOrder>(&body_vec) {
                    Ok(delete_iceberg_order) => {
                        // Lookup the broker's HMAC key in the broker config
                        if let Some(hmac_key) = broker_config.broker_list.get(&delete_iceberg_order.broker_identifier) {
                            // Verify the HMAC signature using the broker-specific key
                            if delete_iceberg_order.market != market_conf.market_name {
                                return HttpResponse::BadRequest().body("Wrong market!");
                            }
                            if let Err(err) = validate_delete_iceberg_order(&delete_iceberg_order) {
                                return HttpResponse::BadRequest().body(err);
                            }
                            let a_order: Result<IcebergOrderStruct, String> = fetch_document_byorderid(&db, &coll_config.coll_p_iceberg, delete_iceberg_order.iceberg_identifier).await;

                            if let Ok(a_order_s) = a_order {
                                if delete_iceberg_order.trader_identifier != a_order_s.trader_identifier {
                                    return HttpResponse::BadRequest().body("The order is not owned by the user.");
                                }
                     
                            } else {
                                return HttpResponse::InternalServerError().body("The iceberg order is not found.");
                            }
                            match verify_hmac(&body_vec, hmac_str, hmac_key) {
                                Ok(true) => {
                                    if let Err(_) = tx.send(Structs::DeleteIcebergOrder(delete_iceberg_order)) {
                                        return HttpResponse::InternalServerError().body("Failed to send order");
                                    }
                                    return HttpResponse::Ok().body("HMAC verified and body deserialized successfully");
                                }
                                Ok(false) => return HttpResponse::Unauthorized().body("Invalid HMAC signature"),
                                Err(_) => return HttpResponse::BadRequest().body("HMAC verification failed"),
                            }
                        } else {
                            return HttpResponse::BadRequest().body("Broker identifier not found");
                        }
                    }
                    Err(_) => return HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
                }
            } else {
                return HttpResponse::BadRequest().body("Invalid HMAC header");
            }
        } else {
            return HttpResponse::BadRequest().body("No HMAC signature provided");
        }
}

/*pub async fn save(
    _req: HttpRequest, // Removed HMAC header handling
    body: web::Bytes,
    tx: web::Data<Arc<mpsc::UnboundedSender<Structs>>>,
    market_conf: web::Data<MarketConf>, // Removed broker_config since it's no longer needed
) -> impl Responder {
    let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>

    // Deserialize the body to extract the broker identifier
    match serde_json::from_slice::<Save>(&body_vec) {
        Ok(save) => {
            // Check if the market exists in the market list
            if save.market != market_conf.market_name {
                return HttpResponse::BadRequest().body("Wrong market!");
            }
            // Send the struct to the channel
            if let Err(_) = tx.send(Structs::Save(save)) {
                return HttpResponse::InternalServerError().body("Failed to send order");
            }
            HttpResponse::Ok().body("Body deserialized successfully and sent to the channel")
        }
        Err(_) => HttpResponse::BadRequest().body("Failed to deserialize MessagePack"),
    }
}*/
pub async fn save(
    _req: HttpRequest,
    save: web::Json<Save>, // Use web::Json to automatically deserialize JSON
    tx: web::Data<sync_mpsc::Sender<Structs>>,
    market_conf: web::Data<MarketConf>,
) -> impl Responder {
    // Check if the market exists in the market list
    if save.market != market_conf.market_name {
        return HttpResponse::BadRequest().body("Wrong market!");
    }

    // Send the struct to the channel
    if let Err(_) = tx.send(Structs::Save(save.into_inner())) {
        return HttpResponse::InternalServerError().body("Failed to send order");
    }

    HttpResponse::Ok().body("JSON body deserialized successfully and sent to the channel")
}

pub async fn history_last(data: web::Json<Number>,market_db: web::Data<MarketDb>, coll_config: web::Data<Arc<CollConfig>>,)-> impl Responder {

    /*let coll_h_last = match env::var("COLL_H_LAST") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };*/
    let db: &Database = &market_db.db;
    match fetch_number_documents::<Last>(&db, &coll_config.coll_h_last, data.number).await {
        Ok(documents) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": documents,  // Return the documents as JSON
        })),
        Err(err) => {
            eprintln!("Error fetching documents: {}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": "Failed to fetch documents",
            }))
        }
    }
   
}
pub async fn history_bbo(data: web::Json<Number>,market_db: web::Data<MarketDb>, coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder{

    /*let coll_h_bbo = match env::var("COLL_H_BBO") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };*/
    let db: &Database = &market_db.db;
    match fetch_number_documents::<BBO>(&db, &coll_config.coll_h_bbo, data.number).await {
        Ok(documents) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": documents,  // Return the documents as JSON
        })),
        Err(err) => {
            eprintln!("Error fetching documents: {}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": "Failed to fetch documents",
            }))
        }
    }
   
}
pub async fn history_tns(data: web::Json<Number>,market_db: web::Data<MarketDb>, coll_config: web::Data<Arc<CollConfig>>,)-> impl Responder {

   /* let coll_h_tns = match env::var("COLL_H_TNS") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };*/
    let db: &Database = &market_db.db;
    match fetch_number_documents::<TimeSale>(&db, &coll_config.coll_h_tns, data.number).await {
        Ok(documents) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": documents,  // Return the documents as JSON
        })),
        Err(err) => {
            eprintln!("Error fetching documents: {}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": "Failed to fetch documents",
            }))
        }
    }
    
   
}
pub async fn history_mbpevent(data: web::Json<Number>,market_db: web::Data<MarketDb>, coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder{

    /*let coll_h_mbp_event = match env::var("COLL_H_MBP_EVENT") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };*/
    let db: &Database = &market_db.db;
    match fetch_number_documents::<MBPEvents>(&db, &coll_config.coll_h_mbpevent, data.number).await {
        Ok(documents) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": documents,  // Return the documents as JSON
        })),
        Err(err) => {
            eprintln!("Error fetching documents: {}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": "Failed to fetch documents",
            }))
        }
    }
   
}
pub async fn history_interestevent(data: web::Json<Number>,market_db: web::Data<MarketDb>, coll_config: web::Data<Arc<CollConfig>>,) -> impl Responder{

    /*let coll_h_interest_event = match env::var("COLL_H_INTEREST_EVENT") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };*/
    let db: &Database = &market_db.db;
    match fetch_number_documents::<InterestEvents>(&db, &coll_config.coll_h_interestevent, data.number).await {
        Ok(documents) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": documents,  // Return the documents as JSON
        })),
        Err(err) => {
            eprintln!("Error fetching documents: {}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": "Failed to fetch documents",
            }))
        }
    }
   
}
pub async fn history_volume(data: web::Json<Number>,market_db: web::Data<MarketDb>, coll_config: web::Data<Arc<CollConfig>>,)-> impl Responder {

    /*let coll_h_volume = match env::var("COLL_H_VOLUME") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };*/
    let db: &Database = &market_db.db;
    match fetch_number_documents::<Volume>(&db, &coll_config.coll_h_volume, data.number).await {
        Ok(documents) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": documents,  // Return the documents as JSON
        })),
        Err(err) => {
            eprintln!("Error fetching documents: {}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": "Failed to fetch documents",
            }))
        }
    }
   
}
pub async fn full_ob_extractor(market_db: web::Data<MarketDb>, coll_config: web::Data<Arc<CollConfig>>,)-> impl Responder {

    /*let coll_full_ob = match env::var("COLL_FULL_OB") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };*/
    let db: &Database = &market_db.db;
    match fetch_one_document::<FullOB>(&db, &coll_config.coll_fullob).await {
        Ok(documents) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": documents,  // Return the documents as JSON
        })),
        Err(err) => {
            eprintln!("Error fetching documents: {}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": "Failed to fetch documents",
            }))
        }
    }
   
}
pub async fn full_interest_extractor(market_db: web::Data<MarketDb>, coll_config: web::Data<Arc<CollConfig>>,)-> impl Responder {

    /*let coll_full_interest = match env::var("COLL_FULL_INTEREST") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };*/
    let db: &Database = &market_db.db;
    match fetch_one_document::<FullOB>(&db, &coll_config.coll_fullinterest).await {
        Ok(documents) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": documents,  // Return the documents as JSON
        })),
        Err(err) => {
            eprintln!("Error fetching documents: {}", err);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "error": "Failed to fetch documents",
            }))
        }
    }
   
}  