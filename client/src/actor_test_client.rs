use actors_rs::*;
use async_trait::async_trait;

use vault::{BoxProvider, Id, Key};

use std::marker::PhantomData;

use crate::data::Blob;
use crate::line_error;
use crate::provider::Provider;

// actors_rs::builder!(BlobBuilder {});
actors_rs::builder!(
    #[derive(Clone)]
    ClientBuilder {}
);

pub struct Client<P: BoxProvider + Send + Sync + 'static> {
    id: Id,
    _provider: PhantomData<P>,
}

#[derive(Serialize, Deserialize)]
pub enum ClientEvent {
    Stop,
}

#[derive(Serialize, Deserialize)]
pub enum BlobEvent {
    Create(Key<Provider>, Vec<u8>),
    Revoke(Key<Provider>, Vec<u8>),
    Read(Key<Provider>, Id),
    GC(Key<Provider>),
    DropOut,
}

pub struct ClientInit {
    tx: tokio::sync::mpsc::UnboundedSender<ClientEvent>,
    rx: tokio::sync::mpsc::UnboundedReceiver<ClientEvent>,
    service: Service,
    client: Client<Provider>,
}

pub struct BlobInit {
    tx: tokio::sync::mpsc::UnboundedSender<BlobEvent>,
    rx: tokio::sync::mpsc::UnboundedReceiver<BlobEvent>,
    service: Service,
    blob: Blob<Provider>,
}

pub struct ClientSender {
    tx: tokio::sync::mpsc::UnboundedSender<ClientEvent>,
}

pub struct BlobSender {
    tx: tokio::sync::mpsc::UnboundedSender<BlobEvent>,
}

impl ThroughType for ClientBuilder {
    type Through = ClientEvent;
}

impl Builder for ClientBuilder {
    type State = ClientInit;
    fn build(self) -> Self::State {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<ClientEvent>();
        ClientInit {
            tx,
            rx,
            service: Service::new(),
            client: Client {
                id: Id::random::<Provider>().expect(line_error!()),
                _provider: PhantomData,
            },
        }
        .set_name()
    }
}

impl<H: LauncherSender<ClientEvent> + AknShutdown<ClientInit>> AppBuilder<H> for ClientBuilder {}

impl<H: LauncherSender<ClientEvent> + AknShutdown<Self>> Actor<H> for ClientInit {}

impl Name for ClientInit {
    fn get_name(&self) -> String {
        self.service.name.clone()
    }
    fn set_name(mut self) -> Self {
        self.service.update_name("Client".to_string());
        self
    }
}

impl Passthrough<ClientEvent> for ClientSender {
    fn send_event(&mut self, _event: ClientEvent, _from_app_name: String) {}
    fn service(&mut self, _service: &Service) {}
    fn launcher_status_change(&mut self, _service: &Service) {}
    fn app_status_change(&mut self, _service: &Service) {}
}

impl Shutdown for ClientSender {
    fn shutdown(self) -> Option<Self> {
        let _ = self.tx.send(ClientEvent::Stop);
        None
    }
}

#[async_trait]
impl<H: LauncherSender<ClientEvent> + AknShutdown<ClientInit>> Starter<H> for ClientBuilder {
    type Ok = ClientSender;
    type Error = ();
    type Input = ClientInit;

    async fn starter(mut self, handle: H, mut _input: Option<Self::Input>) -> Result<Self::Ok, Self::Error> {
        let hello_world = self.build();
        let app_handle = ClientSender {
            tx: hello_world.tx.clone(),
        };
        tokio::spawn(hello_world.start(Some(handle)));

        Ok(app_handle)
    }
}

#[async_trait]
impl<H: LauncherSender<ClientEvent> + AknShutdown<Self>> Init<H> for ClientInit {
    async fn init(&mut self, status: Result<(), Need>, _supervisor: &mut Option<H>) -> Result<(), Need> {
        self.service.update_status(ServiceStatus::Initializing);
        self.client = Client {
            id: Id::random::<Provider>().expect(line_error!()),
            _provider: PhantomData,
        };
        _supervisor.as_mut().unwrap().status_change(self.service.clone());
        status
    }
}

#[async_trait]
impl<H: LauncherSender<ClientEvent> + AknShutdown<Self>> EventLoop<H> for ClientInit {
    async fn event_loop(&mut self, _status: Result<(), Need>, _supervisor: &mut Option<H>) -> Result<(), Need> {
        self.service.update_status(ServiceStatus::Running);
        _supervisor.as_mut().unwrap().status_change(self.service.clone());
        {
            let apps_through: Result<H::AppsEvents, _> = serde_json::from_str("{\"ClientInit\": \"Stop\"}");
            if let Ok(apps_events) = apps_through {
                match apps_events.try_get_my_event() {
                    Ok(ClientEvent::Stop) => {
                        _supervisor.as_mut().unwrap().shutdown_app(&self.get_name());
                    }
                    Err(other_app_event) => {
                        _supervisor
                            .as_mut()
                            .unwrap()
                            .passthrough(other_app_event, self.get_name());
                    }
                }
            } else {
                return Err(Need::Abort);
            };
        }
        while let Some(ClientEvent::Stop) = self.rx.recv().await {
            self.rx.close();
        }
        _status
    }
}

#[async_trait]
impl<H: LauncherSender<ClientEvent> + AknShutdown<Self>> Terminating<H> for ClientInit {
    async fn terminating(&mut self, _status: Result<(), Need>, _supervisor: &mut Option<H>) -> Result<(), Need> {
        self.service.update_status(ServiceStatus::Stopping);
        _supervisor.as_mut().unwrap().status_change(self.service.clone());
        _status
    }
}

launcher!(builder: AppsBuilder {[] -> Client: ClientBuilder}, state: Apps {});

impl Builder for AppsBuilder {
    type State = Apps;
    fn build(self) -> Self::State {
        let client_builder = ClientBuilder::new();

        self.Client(client_builder).to_apps()
    }
}

#[tokio::test]
async fn main() {
    let apps = AppsBuilder::new().build();

    apps.Client().await.start(None).await;
}
