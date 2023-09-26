use std::{
    ffi::{c_char, c_void},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tokio::{
    sync::oneshot::{channel, Sender},
    time::timeout,
};

use super::RawBrowser;
use crate::ptr::{from_c_str, AsCStr};

type BridgeCallCallback = extern "C" fn(res: *const c_char, ctx: *mut c_void);

extern "C" {
    fn browser_bridge_call(
        browser: *const RawBrowser,
        req: *const c_char,
        callback: BridgeCallCallback,
        ctx: *mut c_void,
    );
}

#[async_trait]
pub trait BridgeObserver: Send + Sync {
    type Req: DeserializeOwned + Send;
    type Res: Serialize + 'static;
    type Err: ToString;

    async fn on(&self, req: Self::Req) -> Result<Self::Res, Self::Err>;
}

pub(crate) struct BridgeOnHandler<Q, S, E> {
    processor: Arc<dyn BridgeObserver<Req = Q, Res = S, Err = E>>,
}

impl<Q, S, E> BridgeOnHandler<Q, S, E>
where
    Q: DeserializeOwned + Send,
    S: Serialize + 'static,
    E: ToString,
{
    pub(crate) fn new<T: BridgeObserver<Req = Q, Res = S, Err = E> + 'static>(
        processer: T,
    ) -> Self {
        Self {
            processor: Arc::new(processer),
        }
    }

    pub(crate) async fn handle(&self, req: &str) -> Result<String, String> {
        serde_json::to_string(
            &self
                .processor
                .on(serde_json::from_str(unsafe { std::mem::transmute(req) })
                    .map_err(|s| s.to_string())?)
                .await
                .map_err(|s| s.to_string())?,
        )
        .map_err(|s| s.to_string())
    }
}

#[derive(Clone)]
pub(crate) struct BridgeOnContext(
    pub Arc<dyn Fn(String, Box<dyn FnOnce(Result<String, String>) + Send + Sync>)>,
);

#[derive(Debug)]
pub enum BridgeError {
    SerdeError,
    Timeout,
    CallError,
}

impl std::error::Error for BridgeError {}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub(crate) struct Bridge;

impl Bridge {
    pub(crate) async fn call<Q, S>(
        ptr: *const RawBrowser,
        req: &Q,
    ) -> Result<Option<S>, BridgeError>
    where
        Q: Serialize,
        S: DeserializeOwned,
    {
        let (tx, rx) = channel::<Option<String>>();
        let req = serde_json::to_string(req)
            .map_err(|_| BridgeError::SerdeError)?
            .as_c_str();

        unsafe {
            browser_bridge_call(
                ptr,
                req.ptr,
                bridge_call_callback,
                Box::into_raw(Box::new(tx)) as *mut c_void,
            );
        }

        Ok(
            if let Some(ret) = timeout(Duration::from_secs(10), rx)
                .await
                .map_err(|_| BridgeError::Timeout)?
                .map_err(|_| BridgeError::CallError)?
            {
                Some(serde_json::from_str(&ret).map_err(|_| BridgeError::SerdeError)?)
            } else {
                None
            },
        )
    }
}

extern "C" fn bridge_call_callback(res: *const c_char, ctx: *mut c_void) {
    let tx = unsafe { Box::from_raw(ctx as *mut Sender<Option<String>>) };
    tx.send(from_c_str(res))
        .expect("channel is closed, message send failed!");
}
