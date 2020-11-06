use actors_rs::*;
use async_trait::async_trait;

use vault::Id;

use serde::{Deserialize, Serialize};

use crate::line_error;
use crate::provider::Provider;

actors_rs::builder!(
    #[derive(Clone)]
    ClientBuilder {}
);

#[derive(Clone)]
pub struct Client {
    id: Id,
}

#[derive(Serialize, Deserialize)]
pub enum ClientEvent {
    Stop,
    ReadID,
}

pub struct ClientInit {
    tx: tokio::sync::mpsc::UnboundedSender<ClientEvent>,
    rx: tokio::sync::mpsc::UnboundedReceiver<ClientEvent>,
    service: Service,
    client: Client,
}

pub struct ClientSender {
    tx: tokio::sync::mpsc::UnboundedSender<ClientEvent>,
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
            },
        }
        .set_name()
    }
}

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

impl<H: LauncherSender<ClientEvent> + AknShutdown<ClientInit>> AppBuilder<H> for ClientBuilder {}

impl<H: LauncherSender<ClientEvent> + AknShutdown<Self>> Actor<H> for ClientInit {}

#[async_trait]
impl<H: LauncherSender<ClientEvent> + AknShutdown<ClientInit>> Starter<H> for ClientBuilder {
    type Ok = ClientSender;
    type Error = ();
    type Input = ClientInit;

    async fn starter(mut self, handle: H, mut _input: Option<Self::Input>) -> Result<Self::Ok, Self::Error> {
        let client = self.build();
        let app_handle = ClientSender { tx: client.tx.clone() };

        tokio::spawn(client.start(Some(handle)));
        println!("starting client");

        Ok(app_handle)
    }
}

#[async_trait]
impl<H: LauncherSender<ClientEvent> + AknShutdown<Self>> Init<H> for ClientInit {
    async fn init(&mut self, status: Result<(), Need>, _supervisor: &mut Option<H>) -> Result<(), Need> {
        self.service.update_status(ServiceStatus::Initializing);
        self.client = Client {
            id: Id::random::<Provider>().expect(line_error!()),
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

        {}
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
