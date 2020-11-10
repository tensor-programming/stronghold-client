use vault::{BoxProvider, Id, Key};

use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use crate::data::{Blob, Bucket};

pub struct Client<P: BoxProvider + Clone + Send + Sync + 'static> {
    id: Id,
    blobs: Blob<P>,
    _provider: PhantomData<P>,
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot<P: BoxProvider + Clone + Send + Sync> {
    pub id: Id,
    pub keys: HashSet<Key<P>>,
    pub state: HashMap<Vec<u8>, Vec<u8>>,
}

impl<P: BoxProvider + Clone + Send + Sync + 'static> Client<P> {
    pub fn new(id: Id, blobs: Blob<P>) -> Self {
        Self {
            id,
            blobs,
            _provider: PhantomData,
        }
    }

    pub fn new_from_snapshot(snapshot: Snapshot<P>) -> Self {
        let id = snapshot.id;
        let blobs = Blob::new_from_snapshot(snapshot);

        Self {
            id,
            blobs: blobs,
            _provider: PhantomData,
        }
    }

    pub fn add_vault(&mut self, key: &Key<P>) {
        self.blobs.add_vault(key, self.id);
    }

    pub fn create_record(&mut self, key: Key<P>, payload: Vec<u8>) -> Option<Id> {
        self.blobs.create_record(self.id, key, payload)
    }

    pub fn read_record(&mut self, key: Key<P>, id: Id) {
        self.blobs.read_record(id, key);
    }

    pub fn preform_gc(&mut self, key: Key<P>) {
        self.blobs.garbage_collect(self.id, key)
    }

    pub fn revoke_record_by_id(&mut self, id: Id, key: Key<P>) {
        self.blobs.revoke_record(self.id, id, key)
    }

    pub fn list_valid_ids_for_vault(&mut self, key: Key<P>) {
        self.blobs.list_all_valid_by_key(key)
    }
}

impl<P: BoxProvider + Clone + Send + Sync> Snapshot<P> {
    pub fn new(client: &mut Client<P>) -> Self {
        let id = client.id;
        let (vkeys, state) = client.blobs.clone().offload_data();

        let mut keys = HashSet::new();
        vkeys.iter().for_each(|k| {
            keys.insert(k.clone());
        });

        Self { id, keys, state }
    }

    pub fn offload(self) -> (Id, HashSet<Key<P>>, HashMap<Vec<u8>, Vec<u8>>) {
        (self.id, self.keys, self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Blob;
    use crate::line_error;
    use crate::provider::Provider;

    #[test]
    fn test_vaults() {
        let id = Id::random::<Provider>().expect(line_error!());
        let key = Key::<Provider>::random().expect(line_error!());

        let blobs = Blob::new();

        let mut client = Client::new(id, blobs);
        client.add_vault(&key);

        let tx_id = client.create_record(key.clone(), b"data".to_vec());

        let key_2 = Key::<Provider>::random().expect(line_error!());

        client.add_vault(&key_2);

        let tx_id_2 = client.create_record(key_2.clone(), b"more_data".to_vec());
        client.list_valid_ids_for_vault(key.clone());
        client.list_valid_ids_for_vault(key_2.clone());
        client.read_record(key_2.clone(), tx_id_2.unwrap());
        client.read_record(key, tx_id.unwrap());

        client.revoke_record_by_id(tx_id_2.unwrap(), key_2.clone());

        client.preform_gc(key_2);
    }
}
