use actors_rs::*;

mod blob;
mod channel;
mod client;
mod registry;
mod stream;

use crate::actor_test_client::blob::BlobBuilder;
use crate::actor_test_client::client::{ClientBuilder, ClientEvent};
use crate::actor_test_client::registry::Registry;

launcher!(builder: AppsBuilder {[] -> Client: ClientBuilder, [] -> Blob: BlobBuilder}, state: Apps {});

impl Builder for AppsBuilder {
    type State = Apps;

    fn build(self) -> Self::State {
        let client_builder = ClientBuilder::new();
        let blob_builder = BlobBuilder::new();

        self.Client(client_builder).Blob(blob_builder).to_apps()
    }
}

#[tokio::test]
async fn main() {
    let apps = AppsBuilder::new().build();

    let client = apps.Client().await;

    client.start(Some(NullSupervisor)).await;
}

// #[tokio::test]
// async fn test_registry() {
//     impl Registry for ClientBuilder {};

//     let client = ClientBuilder::new();
//     let state = client.build();

//     ClientBuilder::register_once(state.tx.clone()).await;

//     let new_sender = ClientBuilder::from_registry::<ClientEvent>().await.unwrap();

//     assert_eq!(state.tx, new_sender);
// }
