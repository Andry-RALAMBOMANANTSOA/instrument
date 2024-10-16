use actix_web::{rt::time, web,HttpRequest, HttpResponse, Responder};
use serde_json::json;
use mongodb::{Database,bson::doc,bson,Collection};
use tokio::sync::mpsc;
use shared_structs::*;
use crate::function::*;
use std::sync::mpsc as sync_mpsc;
use chrono::{DateTime, Utc, SecondsFormat,Duration,NaiveDateTime,Local,TimeZone};
use std::sync::Arc;
use std::{default, env};


pub async fn limit_order(req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,) -> impl Responder {
        let received_hmac = req.headers().get("X-HMAC-SIGNATURE");
    
        if let Some(hmac_value) = received_hmac {
            if let Ok(hmac_str) = hmac_value.to_str() {
                let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>
    
                // Deserialize the body to extract the broker identifier
                match rmp_serde::from_slice::<LimitOrder>(&body_vec) {
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

pub async fn market_order(req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,) -> impl Responder {
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

pub async fn stop_order(req: HttpRequest, body: web::Bytes,tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,) -> impl Responder {
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

pub async fn stoplimit_order(req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,) -> impl Responder {
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

pub async fn modify_order(req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,) -> impl Responder {
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

pub async fn delete_order(req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,) -> impl Responder {
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

pub async fn iceberg_order(req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,) -> impl Responder {
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

pub async fn modify_iceberg_order(req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,
    market_spec_config: web::Data<MarketSpecConfig>,) -> impl Responder {
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

pub async fn delete_iceberg_order(req: HttpRequest, body: web::Bytes, tx: web::Data<sync_mpsc::Sender<Structs>>,
    broker_config: web::Data<BrokerConfigDedicaced>,
    market_conf: web::Data<MarketConf>,) -> impl Responder {
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

pub async fn save(
    _req: HttpRequest, // Removed HMAC header handling
    body: web::Bytes,
    tx: web::Data<Arc<mpsc::UnboundedSender<Structs>>>,
    market_conf: web::Data<MarketConf>, // Removed broker_config since it's no longer needed
) -> impl Responder {
    let body_vec = body.to_vec(); // Convert Bytes to Vec<u8>

    // Deserialize the body to extract the broker identifier
    match rmp_serde::from_slice::<Save>(&body_vec) {
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
}


pub async fn history_last(data: web::Json<Number>,db: web::Data<Database>)-> impl Responder {

    let coll_h_last = match env::var("COLL_H_LAST") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };
    match fetch_number_documents::<Last>(&db, &coll_h_last, data.number).await {
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
pub async fn history_bbo(data: web::Json<Number>,db: web::Data<Database>) -> impl Responder{

    let coll_h_bbo = match env::var("COLL_H_BBO") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };
    match fetch_number_documents::<BBO>(&db, &coll_h_bbo, data.number).await {
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
pub async fn history_tns(data: web::Json<Number>,db: web::Data<Database>)-> impl Responder {

    let coll_h_tns = match env::var("COLL_H_TNS") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };
    match fetch_number_documents::<TimeSale>(&db, &coll_h_tns, data.number).await {
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
pub async fn history_mbpevent(data: web::Json<Number>,db: web::Data<Database>) -> impl Responder{

    let coll_h_mbp_event = match env::var("COLL_H_MBP_EVENT") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };
    match fetch_number_documents::<MBPEvents>(&db, &coll_h_mbp_event, data.number).await {
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
pub async fn history_interestevent(data: web::Json<Number>,db: web::Data<Database>) -> impl Responder{

    let coll_h_interest_event = match env::var("COLL_H_INTEREST_EVENT") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };
    match fetch_number_documents::<InterestEvents>(&db, &coll_h_interest_event, data.number).await {
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
pub async fn history_volume(data: web::Json<Number>,db: web::Data<Database>)-> impl Responder {

    let coll_h_volume = match env::var("COLL_H_VOLUME") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };
    match fetch_number_documents::<Volume>(&db, &coll_h_volume, data.number).await {
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
pub async fn full_ob_extractor(db: web::Data<Database>)-> impl Responder {

    let coll_full_ob = match env::var("COLL_FULL_OB") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };
    match fetch_one_document::<FullOB>(&db, &coll_full_ob).await {
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
pub async fn full_interest_extractor(db: web::Data<Database>)-> impl Responder {

    let coll_full_interest = match env::var("COLL_FULL_INTEREST") {
        Ok(name) => name,
        Err(err) => {
            eprintln!("COLLECTION_NAME_SENSITIVE_ERROR: {}", err);
            return HttpResponse::InternalServerError()
            .json(json!({
                "Error": "Internal Server Error",
               
            }));
        }
    };
    match fetch_one_document::<FullOB>(&db, &coll_full_interest).await {
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