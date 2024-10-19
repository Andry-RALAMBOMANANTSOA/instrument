use hmac::Hmac;
use sha2::Sha256;
use dashmap::DashMap;
use std::sync::Arc;

pub type HmacSha256 = Hmac<Sha256>;

#[derive(Eq, Hash, PartialEq,Clone)]
pub enum ConnectionType {
    Last,
    MbpEvent,
    Nbbo,
    Tns,
    Volume,
    InterestEvent,
    LastMsgp,
    MbpEventMsgp,
    NbboMsgp,
    TnsMsgp,
    VolumeMsgp,
    InterestEventMsgp,
   
}

pub type WsConnections = Arc<DashMap<String, actix_ws::Session>>;
pub type ConnectionTypeMap = Arc<DashMap<ConnectionType, Vec<String>>>;