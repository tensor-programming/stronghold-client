use actors_rs::*;
use async_trait::async_trait;

use vault::{Id, Key};

use serde::{Deserialize, Serialize};

use crate::data::{Blob, Bucket};
use crate::provider::Provider;

actors_rs::builder!(
    #[derive(Clone)]
    BlobBuilder { id: Id }
);

#[derive(Serialize, Deserialize)]
pub enum BlobEvent {
    Create(Key<Provider>, Vec<u8>),
    Revoke(Id, Key<Provider>),
    Read(Key<Provider>, Id),
    GC(Key<Provider>),
    ReturnTxid(Option<Id>),
    Init(Id),
    DropOut,
}

pub struct BlobInit {
    tx: tokio::sync::mpsc::UnboundedSender<BlobEvent>,
    rx: tokio::sync::mpsc::UnboundedReceiver<BlobEvent>,
    service: Service,
    id: Id,
    blob: Blob<Provider>,
}

pub struct BlobSender {
    tx: tokio::sync::mpsc::UnboundedSender<BlobEvent>,
}

impl ThroughType for BlobBuilder {
    type Through = BlobEvent;
}

impl Builder for BlobBuilder {
    type State = BlobInit;

    fn build(self) -> Self::State {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<BlobEvent>();

        BlobInit {
            tx,
            rx,
            id: self.id.unwrap(),
            service: Service::new(),
            blob: Blob::new(),
        }
        .set_name()
    }
}

impl Name for BlobInit {
    fn get_name(&self) -> String {
        self.service.name.clone()
    }
    fn set_name(mut self) -> Self {
        self.service.update_name("Blob".to_string());
        self
    }
}

impl Passthrough<BlobEvent> for BlobSender {
    fn send_event(&mut self, _event: BlobEvent, _from_app_name: String) {}
    fn service(&mut self, _service: &Service) {}
    fn launcher_status_change(&mut self, _service: &Service) {}
    fn app_status_change(&mut self, _service: &Service) {}
}

impl Shutdown for BlobSender {
    fn shutdown(self) -> Option<Self> {
        let _ = self.tx.send(BlobEvent::DropOut);
        None
    }
}

impl<H: LauncherSender<BlobEvent> + AknShutdown<BlobInit>> AppBuilder<H> for BlobBuilder {}

impl<H: LauncherSender<BlobEvent> + AknShutdown<Self>> Actor<H> for BlobInit {}

#[async_trait]
impl<H: LauncherSender<BlobEvent> + AknShutdown<BlobInit>> Starter<H> for BlobBuilder {
    type Ok = BlobSender;
    type Error = ();
    type Input = BlobInit;

    async fn starter(mut self, handle: H, mut _input: Option<Self::Input>) -> Result<Self::Ok, Self::Error> {
        let blob = self.build();

        let app_handle = BlobSender { tx: blob.tx.clone() };

        tokio::spawn(blob.start(Some(handle)));

        Ok(app_handle)
    }
}

#[async_trait]
impl<H: LauncherSender<BlobEvent> + AknShutdown<Self>> Init<H> for BlobInit {
    async fn init(&mut self, status: Result<(), Need>, _supervisor: &mut Option<H>) -> Result<(), Need> {
        self.service.update_status(ServiceStatus::Initializing);

        self.blob = Blob::new();

        if let Some(BlobEvent::Init(uid)) = self.rx.recv().await {
            self.id = uid;
            println!("Set blob id");
        }

        _supervisor.as_mut().unwrap().status_change(self.service.clone());
        println!("starting blob");
        status
    }
}

#[async_trait]
impl<H: LauncherSender<BlobEvent> + AknShutdown<Self>> EventLoop<H> for BlobInit {
    async fn event_loop(&mut self, _status: Result<(), Need>, _supervisor: &mut Option<H>) -> Result<(), Need> {
        self.service.update_status(ServiceStatus::Running);
        _supervisor.as_mut().unwrap().status_change(self.service.clone());

        match self.rx.recv().await {
            Some(BlobEvent::Create(k, pay)) => {
                let xid = self.blob.create_record(self.id, k, pay);

                match self.tx.send(BlobEvent::ReturnTxid(xid)) {
                    Ok(_) => {}
                    Err(_) => panic!("Send error"),
                };
            }
            Some(BlobEvent::GC(k)) => {
                self.blob.garbage_collect(self.id, k);
            }
            Some(BlobEvent::Read(k, id)) => {
                self.blob.read_record(id, k);
            }
            Some(BlobEvent::Revoke(id, k)) => self.blob.revoke_record(self.id, id, k),
            Some(BlobEvent::DropOut) => {
                self.rx.close();
            }
            Some(_) => {}
            None => {}
        }

        _status
    }
}

#[async_trait]
impl<H: LauncherSender<BlobEvent> + AknShutdown<Self>> Terminating<H> for BlobInit {
    async fn terminating(&mut self, _status: Result<(), Need>, _supervisor: &mut Option<H>) -> Result<(), Need> {
        self.service.update_status(ServiceStatus::Stopping);
        _supervisor.as_mut().unwrap().status_change(self.service.clone());
        _status
    }
}
