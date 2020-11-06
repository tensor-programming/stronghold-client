use actors_rs::*;

mod blob;
mod client;

use crate::actor_test_client::blob::BlobBuilder;
use crate::actor_test_client::client::ClientBuilder;

launcher!(builder: AppsBuilder {[] -> Client: ClientBuilder, [Client] -> Blob: BlobBuilder}, state: Apps {});

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
