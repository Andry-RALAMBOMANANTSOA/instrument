use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_ws::{handle, AggregatedMessage, Session};
use futures_util::StreamExt as _;
use tokio::time::{interval, Duration, Instant};
use crate::shared::*; // Assuming shared types and functions are defined here

// Define the type for WebSocket connections


// Enum to differentiate connection types


// WebSocket Connection Handler
pub struct WSConnection {
    connection_type: ConnectionType,
    ws_connections:WsConnections,
    connection_type_map: ConnectionTypeMap,
}


impl WSConnection {
    async fn handle_ws(
        self,
        mut session: actix_ws::Session,
        mut stream: impl futures_util::Stream<Item = Result<AggregatedMessage, actix_ws::ProtocolError>> + Unpin,
        
    ) {
        // Generate a unique ID for each WebSocket connection
        let session_id = uuid::Uuid::new_v4().to_string();

        // Store connection without a user ID
        self.ws_connections.insert(session_id.clone(), session.clone());

        self.connection_type_map
            .entry(self.connection_type.clone())
            .or_insert_with(Vec::new)
            .push(session_id.clone());

        // Create a ticker that ticks every minute
        let mut ticker = interval(Duration::from_secs(60));
        let mut last_ping = Instant::now();  // Record the last time a ping was received

        // Handle incoming messages
        loop {
            tokio::select! {
                // Handle WebSocket messages
                Some(msg) = stream.next() => {
                    match msg {
                        Ok(AggregatedMessage::Text(text)) => {
                            println!("Received from client: {}", text);
                            
                            // Handle ping message from the client
                            if text == r#"{"type":"ping"}"# {
                                last_ping = Instant::now();  // Update last ping time
                                session.text(r#"{"type":"pong"}"#).await.unwrap();
                              } else if text == r#"{"type":"close"}"# {
                                // Handle close message from the client
                                println!("Client requested connection close.");
                                self.ws_connections.remove(&session_id);
                                    // Use a scoped lock to remove the session_id from ws_connections
                                    let mut is_session_empty = false;
                                    if let Some(mut sessions) = self.connection_type_map.get_mut(&self.connection_type) {
                                        sessions.retain(|id| id != &session_id);
                                        // Remove the entry if it's empty
                                        if sessions.is_empty() {
                                           is_session_empty = true;
                                        }
                                    }
                                    if is_session_empty {
                                        self.connection_type_map.remove(&self.connection_type);
                                    }
                                    
                                break;
                            } else {
                                session.text(text).await.unwrap();
                            }
                        }
                        Ok(AggregatedMessage::Binary(bin)) => {
                            println!("Received binary from client");
                            session.binary(bin).await.unwrap();
                        }
                        Ok(AggregatedMessage::Ping(msg)) => {
                            println!("Received ping from client");
                            last_ping = Instant::now();  // Update last ping time
                            session.pong(&msg).await.unwrap();
                        }
                        Ok(_) => {}
                        Err(err) => {
                            println!("WebSocket error: {:?}", err);
                            break;
                        }
                    }
                },

                // Check if 5 minutes have passed without receiving a ping
                _ = ticker.tick() => {
                    if last_ping.elapsed() > Duration::from_secs(60) {  // 5 minutes
                        println!("No ping received from client for 5 minutes, closing connection.");
                        self.ws_connections.remove(&session_id);
                        let mut is_session_empty = false;
                        if let Some(mut sessions) = self.connection_type_map.get_mut(&self.connection_type) {
                            sessions.retain(|id| id != &session_id);
                            // Remove the entry if it's empty
                            if sessions.is_empty() {
                               is_session_empty = true;
                            }
                        }
                        if is_session_empty {
                            self.connection_type_map.remove(&self.connection_type);
                        }
                        break;
                    }
                },
            }
        }

        self.ws_connections.remove(&session_id);
        let mut is_session_empty = false;
        if let Some(mut sessions) = self.connection_type_map.get_mut(&self.connection_type) {
            sessions.retain(|id| id != &session_id);
            // Remove the entry if it's empty
            if sessions.is_empty() {
               is_session_empty = true;
            }
        }
        if is_session_empty {
            self.connection_type_map.remove(&self.connection_type);
        }
    } 
    
    }

  

// Handler for last price updates WebSocket
pub async fn last_handler(
    req: HttpRequest,
    stream: web::Payload,
    ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>,
) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::Last,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
       
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

// Handler for MBP event WebSocket
pub async fn mbp_event_handler(req: HttpRequest, stream: web::Payload, ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>,) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::MbpEvent,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}
pub async fn interest_event_handler(req: HttpRequest, stream: web::Payload, ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>,) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::InterestEvent,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

// Handler for BBO WebSocket
pub async fn bbo_handler(req: HttpRequest, stream: web::Payload,  ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::Nbbo,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

// Handler for TNS WebSocket
pub async fn tns_handler(req: HttpRequest, stream: web::Payload,  ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::Tns,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

// Handler for volume WebSocket
pub async fn volume_handler(req: HttpRequest, stream: web::Payload,  ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::Volume,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}


////MessagePack handler

pub async fn last_handler_msgp(
    req: HttpRequest,
    stream: web::Payload,
    ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>,
) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::LastMsgp,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
       
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

// Handler for MBP event WebSocket
pub async fn mbp_event_handler_msgp(req: HttpRequest, stream: web::Payload, ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>,) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::MbpEventMsgp,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}
pub async fn interest_event_handler_msgp(req: HttpRequest, stream: web::Payload, ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>,) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::InterestEventMsgp,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

// Handler for BBO WebSocket
pub async fn bbo_handler_msgp(req: HttpRequest, stream: web::Payload,  ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::NbboMsgp,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

// Handler for TNS WebSocket
pub async fn tns_handler_msgp(req: HttpRequest, stream: web::Payload,  ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::TnsMsgp,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

// Handler for volume WebSocket
pub async fn volume_handler_msgp(req: HttpRequest, stream: web::Payload,  ws_connections: web::Data<WsConnections>,
    connection_type_map: web::Data<ConnectionTypeMap>) -> Result<HttpResponse, Error> {
    let (res, session, stream) = handle(&req, stream)?;
    let aggregated_stream = stream.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    let ws = WSConnection {
        connection_type: ConnectionType::VolumeMsgp,
        ws_connections: ws_connections.get_ref().clone(),
        connection_type_map: connection_type_map.get_ref().clone(),
    };

    actix_web::rt::spawn(ws.handle_ws(session, aggregated_stream));
    Ok(res)
}

