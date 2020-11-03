use vault::{BoxProvider, Id, Key};

use std::{
    collections::{HashMap, HashSet},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use crate::data::Bucket;

pub struct Client<P: BoxProvider + Send + Sync + 'static, V: Bucket<P>> {
    id: Id,
    vaults: V,
    _provider: PhantomData<P>,
}

#[derive(Serialize, Deserialize)]
pub struct Snapshot<P: BoxProvider> {
    pub ids: HashSet<Id>,
    pub keys: HashSet<Key<P>>,
    state: HashMap<Vec<u8>, Vec<u8>>,
}

impl<P: BoxProvider + Send + Sync + 'static, V: Bucket<P>> Client<P, V> {
    pub fn new(id: Id, vaults: V) -> Self {
        Self {
            id,
            vaults,
            _provider: PhantomData,
        }
    }

    pub fn add_vault(&mut self, key: &Key<P>) {
        self.vaults.add_vault(key, self.id);
    }

    pub fn create_record(&mut self, key: Key<P>, payload: Vec<u8>) -> Option<Id> {
        self.vaults.create_record(self.id, key, payload)
    }

    pub fn read_record(&mut self, key: Key<P>, id: Id) {
        self.vaults.read_record(id, key);
    }

    pub fn preform_gc(&mut self, key: Key<P>) {
        self.vaults.garbage_collect(self.id, key)
    }

    pub fn revoke_record_by_id(&mut self, id: Id, key: Key<P>) {
        self.vaults.revoke_record(self.id, id, key)
    }

    pub fn list_valid_ids_for_vault(&mut self, key: Key<P>) {
        self.vaults.list_all_valid_by_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Blob;
    use crate::line_error;
    use crate::provider::Provider;

    #[test]
    fn test_stuff() {
        let id = Id::random::<Provider>().expect(line_error!());
        let key = Key::<Provider>::random().expect(line_error!());

        let vaults = Blob::new();

        let mut client = Client::new(id, vaults);
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
